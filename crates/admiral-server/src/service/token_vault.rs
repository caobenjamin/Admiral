//! Encrypted OAuth token storage (AES-256-GCM). One nonce per row; we use 24 bytes (12+12) for access and refresh.

use admiral_types::api::TokenMeta;
use crate::entities::oauth_token;
use crate::error::{AppError, Result};
use crate::infra::crypto;
use chrono::{DateTime, Utc};
use sea_orm::*;
use uuid::Uuid;

#[derive(Clone)]
pub struct TokenVault {
    db: DatabaseConnection,
    key: [u8; 32],
}

#[derive(Clone, Debug)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub refresh_expires_at: Option<DateTime<Utc>>,
}

impl TokenVault {
    pub fn new(db: DatabaseConnection, key_hex: &str) -> Result<Self> {
        let key = crypto::key_from_hex(key_hex).map_err(|e| AppError::Internal(anyhow::anyhow!("token key: {}", e)))?;
        Ok(Self { db, key })
    }

    pub async fn store(
        &self,
        user_id: Uuid,
        platform: &str,
        access_token: &str,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
        refresh_expires_at: Option<DateTime<Utc>>,
        bid: Option<i64>,
        user_name: Option<String>,
    ) -> Result<Uuid> {
        let (access_enc, nonce_access) = crypto::encrypt(access_token.as_bytes(), &self.key)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("encrypt: {}", e)))?;
        let (refresh_enc, nonce_refresh) = crypto::encrypt(refresh_token.as_bytes(), &self.key)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("encrypt: {}", e)))?;
        let mut nonce = nonce_access;
        nonce.extend_from_slice(&nonce_refresh);

        let id = Uuid::new_v4();
        let now = Utc::now();
        let mut active = oauth_token::ActiveModel {
            id: Set(id),
            user_id: Set(user_id),
            platform: Set(platform.to_string()),
            access_token_encrypted: Set(access_enc),
            refresh_token_encrypted: Set(refresh_enc),
            token_nonce: Set(nonce),
            expires_at: Set(expires_at.into()),
            refresh_expires_at: Set(refresh_expires_at.map(Into::into)),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        };
        active.bid = Set(bid);
        active.user_name = Set(user_name);
        active.insert(&self.db).await?;
        Ok(id)
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<StoredToken>> {
        let row = oauth_token::Entity::find_by_id(id)
            .one(&self.db)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let nonce = &row.token_nonce;
        if nonce.len() < 24 {
            return Err(AppError::Internal(anyhow::anyhow!("invalid nonce length")));
        }
        let (n1, n2) = nonce.split_at(12);
        let access = crypto::decrypt(&row.access_token_encrypted, n1, &self.key)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("decrypt: {}", e)))?;
        let refresh = crypto::decrypt(&row.refresh_token_encrypted, n2, &self.key)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("decrypt: {}", e)))?;
        let access_token = String::from_utf8(access).map_err(|e| AppError::Internal(anyhow::anyhow!("utf8: {}", e)))?;
        let refresh_token = String::from_utf8(refresh).map_err(|e| AppError::Internal(anyhow::anyhow!("utf8: {}", e)))?;
        let expires_at = row.expires_at.with_timezone(&Utc);
        let refresh_expires_at = row.refresh_expires_at.map(|t| t.with_timezone(&Utc));
        Ok(Some(StoredToken {
            access_token,
            refresh_token,
            expires_at,
            refresh_expires_at,
        }))
    }

    pub async fn update(
        &self,
        id: Uuid,
        access_token: &str,
        refresh_token: &str,
        expires_at: DateTime<Utc>,
        refresh_expires_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        let row = oauth_token::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound("oauth_token".into()))?;
        let (access_enc, nonce_access) = crypto::encrypt(access_token.as_bytes(), &self.key)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("encrypt: {}", e)))?;
        let (refresh_enc, nonce_refresh) = crypto::encrypt(refresh_token.as_bytes(), &self.key)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("encrypt: {}", e)))?;
        let mut nonce = nonce_access;
        nonce.extend_from_slice(&nonce_refresh);
        let mut active: oauth_token::ActiveModel = row.into();
        active.access_token_encrypted = Set(access_enc);
        active.refresh_token_encrypted = Set(refresh_enc);
        active.token_nonce = Set(nonce);
        active.expires_at = Set(expires_at.into());
        active.refresh_expires_at = Set(refresh_expires_at.map(Into::into));
        active.updated_at = Set(Utc::now().into());
        active.save(&self.db).await?;
        Ok(())
    }

    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<TokenMeta>> {
        let rows = oauth_token::Entity::find()
            .filter(oauth_token::Column::UserId.eq(user_id))
            .order_by_desc(oauth_token::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| TokenMeta {
                id: r.id,
                platform: r.platform,
                created_at: r.created_at.to_rfc3339(),
                bid: r.bid,
                user_name: r.user_name,
            })
            .collect())
    }

    /// List (token_id, user_id) for tokens expiring within the given duration (e.g. 1 hour).
    pub async fn list_expiring_soon(&self, within: chrono::Duration) -> Result<Vec<(Uuid, Uuid)>> {
        let threshold = Utc::now() + within;
        let rows = oauth_token::Entity::find()
            .filter(oauth_token::Column::ExpiresAt.lte(threshold))
            .all(&self.db)
            .await?;
        Ok(rows.into_iter().map(|r| (r.id, r.user_id)).collect())
    }

    pub async fn delete(&self, id: Uuid, user_id: Uuid) -> Result<bool> {
        let row = oauth_token::Entity::find_by_id(id)
            .filter(oauth_token::Column::UserId.eq(user_id))
            .one(&self.db)
            .await?;
        if let Some(r) = row {
            r.delete(&self.db).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
