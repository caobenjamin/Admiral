//! Authentication: register, login, refresh, me, license.

use admiral_types::api::{
    LicenseResponse, RefreshResponse, TokenResponse, UserInfo,
};
use crate::config::AppConfig;
use crate::entities::{entitlement, invite, user};
use crate::error::{AppError, Result};
use crate::infra::jwt::{self, JwtKeys};
use argon2::password_hash::PasswordVerifier;
use chrono::{Duration, Utc};
use sea_orm::*;
use std::sync::Arc;
use uuid::Uuid;

pub struct AuthService {
    db: DatabaseConnection,
    jwt: Arc<JwtKeys>,
    access_ttl: i64,
    refresh_ttl: i64,
}

impl AuthService {
    pub fn new(db: DatabaseConnection, jwt: Arc<JwtKeys>, config: &AppConfig) -> Self {
        Self {
            db,
            jwt,
            access_ttl: config.auth.access_token_ttl_seconds as i64,
            refresh_ttl: config.auth.refresh_token_ttl_seconds as i64,
        }
    }

    pub async fn register(
        &self,
        email: String,
        password: String,
        name: String,
        company: Option<String>,
        invite_code: String,
    ) -> Result<TokenResponse> {
        // 邀请码统一按大写匹配，并去掉首尾空格（与生成时格式一致）
        let code = invite_code.trim().to_uppercase();
        let invite = invite::Entity::find()
            .filter(invite::Column::Code.eq(&code))
            .one(&self.db)
            .await?
            .ok_or(AppError::InviteInvalid)?;

        if invite.used_count >= invite.max_uses {
            return Err(AppError::InviteInvalid);
        }
        if let Some(exp) = invite.expires_at {
            if exp.timestamp_millis() < Utc::now().timestamp_millis() {
                return Err(AppError::InviteInvalid);
            }
        }

        if user::Entity::find()
            .filter(user::Column::Email.eq(&email))
            .one(&self.db)
            .await?
            .is_some()
        {
            return Err(AppError::EmailExists);
        }

        use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = argon2::Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("hash: {}", e)))?
            .to_string();

        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let starts_at = now;
        let expires_at = now + Duration::days(invite.trial_days as i64);

        let user_active = user::ActiveModel {
            id: Set(user_id),
            email: Set(email.clone()),
            password_hash: Set(password_hash),
            name: Set(name.clone()),
            company: Set(company),
            role: Set("user".to_string()),
            status: Set("active".to_string()),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        };
        user_active.insert(&self.db).await?;

        let ent_active = entitlement::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            invite_id: Set(Some(invite.id)),
            r#type: Set("trial".to_string()),
            starts_at: Set(starts_at.into()),
            expires_at: Set(expires_at.into()),
            is_active: Set(true),
            created_at: Set(now.into()),
            ..Default::default()
        };
        ent_active.insert(&self.db).await?;

        invite::ActiveModel {
            id: Set(invite.id),
            used_count: Set(invite.used_count + 1),
            ..Default::default()
        }
        .save(&self.db)
        .await?;

        self.issue_tokens(user_id, &email, &name, "user")
    }

    pub async fn login(&self, email: String, password: String) -> Result<TokenResponse> {
        let u = user::Entity::find()
            .filter(user::Column::Email.eq(&email))
            .one(&self.db)
            .await?
            .ok_or(AppError::InvalidCredentials)?;

        let parsed = argon2::password_hash::PasswordHash::new(&u.password_hash)
            .map_err(|_| AppError::InvalidCredentials)?;
        let ok = argon2::Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok();
        if !ok {
            return Err(AppError::InvalidCredentials);
        }

        self.issue_tokens(u.id, &u.email, &u.name, &u.role)
    }

    pub async fn refresh(&self, refresh_token: String) -> Result<RefreshResponse> {
        let claims = self
            .jwt
            .verify(&refresh_token)
            .map_err(|_| AppError::InvalidToken)?;
        if claims.token_type != "refresh" {
            return Err(AppError::InvalidToken);
        }
        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AppError::InvalidToken)?;
        let _u = user::Entity::find_by_id(user_id)
            .one(&self.db)
            .await?
            .ok_or(AppError::InvalidToken)?;

        let access_claims = jwt::build_claims(user_id, "access", self.access_ttl);
        let access_token = self
            .jwt
            .sign(&access_claims)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("jwt sign: {}", e)))?;

        Ok(RefreshResponse {
            access_token,
            expires_in: self.access_ttl as u64,
            token_type: "Bearer".to_string(),
        })
    }

    pub async fn me(&self, user_id: Uuid) -> Result<UserInfo> {
        let u = user::Entity::find_by_id(user_id)
            .one(&self.db)
            .await?
            .ok_or(AppError::NotFound("user".into()))?;
        Ok(UserInfo {
            id: u.id.to_string(),
            email: u.email,
            name: u.name,
            role: u.role,
        })
    }

    pub async fn license(&self, user_id: Uuid) -> Result<LicenseResponse> {
        let ent = entitlement::Entity::find()
            .filter(entitlement::Column::UserId.eq(user_id))
            .filter(entitlement::Column::IsActive.eq(true))
            .order_by_desc(entitlement::Column::ExpiresAt)
            .one(&self.db)
            .await?;

        let (is_active, expires_at, type_) = match ent {
            Some(e) => {
                let exp_ts = e.expires_at.timestamp_millis();
                let active = exp_ts > Utc::now().timestamp_millis();
                (
                    active,
                    Some(e.expires_at.to_rfc3339()),
                    Some(e.r#type),
                )
            }
            None => (false, None, None),
        };
        Ok(LicenseResponse {
            is_active,
            expires_at,
            r#type: type_,
        })
    }

    fn issue_tokens(
        &self,
        user_id: Uuid,
        email: &str,
        name: &str,
        role: &str,
    ) -> Result<TokenResponse> {
        let access_claims = jwt::build_claims(user_id, "access", self.access_ttl);
        let refresh_claims = jwt::build_claims(user_id, "refresh", self.refresh_ttl);
        let access_token = self
            .jwt
            .sign(&access_claims)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("jwt: {}", e)))?;
        let refresh_token = self
            .jwt
            .sign(&refresh_claims)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("jwt: {}", e)))?;
        Ok(TokenResponse {
            access_token,
            refresh_token,
            expires_in: self.access_ttl as u64,
            token_type: "Bearer".to_string(),
            user: UserInfo {
                id: user_id.to_string(),
                email: email.to_string(),
                name: name.to_string(),
                role: role.to_string(),
            },
        })
    }
}
