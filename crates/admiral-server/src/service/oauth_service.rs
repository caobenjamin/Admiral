//! Bilibili OAuth: initiate, callback (exchange token), status, delete, list advertisers, bind accounts.

use crate::entities::ad_account_binding;
use crate::error::{AppError, Result};
use admiral_types::api::{BoundAccountRecord, InitiateResponse, TokenMeta};
use crate::service::token_vault::TokenVault;
use chrono::Utc;
use moka::future::Cache;
use reqwest::Client;
use sea_orm::*;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const AUTH_URL: &str = "https://ad.bilibili.com/developer/developer-authorization";
const TOKEN_URL: &str = "https://cm.bilibili.com/takumi/api/open_api/oauth2/token";
const USERINFO_URL: &str = "https://cm.bilibili.com/takumi/api/open_api/oauth2/userinfo";
const ADVERTISERS_URL: &str = "https://cm.bilibili.com/takumi/api/open_api/oauth2/advertisers";

#[derive(Clone)]
pub struct OauthService {
    pub app_id: String,
    pub app_secret: String,
    pub callback_url: String,
    pub state_cache: Arc<Cache<String, Uuid>>,
    pub vault: TokenVault,
    pub db: DatabaseConnection,
    pub http: Client,
    pub max_accounts_per_user: u32,
}

#[derive(Debug, Deserialize)]
struct BiliTokenResponse {
    code: i32,
    message: Option<String>,
    data: Option<BiliTokenData>,
}

#[derive(Debug, Deserialize)]
struct BiliTokenData {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    #[serde(default)]
    refresh_expires_in: Option<i64>,
}

/// B 站可能返回 account_role 为数字(如 2)或字符串，account_id 偶为字符串
fn deserialize_option_flexible<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    Ok(opt.and_then(|v| match v {
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::String(s) => Some(s),
        _ => None,
    }))
}

fn deserialize_i64_flexible<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::Number(n) => n
            .as_i64()
            .ok_or_else(|| serde::de::Error::custom("invalid number")),
        serde_json::Value::String(s) => s
            .parse::<i64>()
            .map_err(|_| serde::de::Error::custom("invalid account_id string")),
        _ => Err(serde::de::Error::custom("account_id must be number or string")),
    }
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct BiliAdvertiserRecord {
    #[serde(deserialize_with = "deserialize_i64_flexible")]
    pub account_id: i64,
    pub account_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_flexible")]
    pub account_role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BiliAdvertisersData {
    pub records: Option<Vec<BiliAdvertiserRecord>>,
    pub total: Option<i64>,
    pub page: Option<i32>,
    pub size: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct BiliAdvertisersResponse {
    pub code: i32,
    pub message: Option<String>,
    pub data: Option<BiliAdvertisersData>,
}

#[derive(Debug, Deserialize)]
struct BiliUserinfoData {
    pub bid: Option<i64>,
    pub user_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BiliUserinfoResponse {
    #[allow(dead_code)]
    pub code: i32,
    #[allow(dead_code)]
    pub message: Option<String>,
    pub data: Option<BiliUserinfoData>,
}

impl OauthService {
    pub fn new(
        app_id: String,
        app_secret: String,
        callback_url: String,
        state_ttl_secs: u64,
        max_accounts_per_user: u32,
        vault: TokenVault,
        db: DatabaseConnection,
    ) -> Self {
        let state_cache = Arc::new(
            Cache::builder()
                .time_to_live(Duration::from_secs(state_ttl_secs))
                .build(),
        );
        let http = Client::new();
        Self {
            app_id,
            app_secret,
            callback_url,
            state_cache,
            vault,
            db,
            http,
            max_accounts_per_user,
        }
    }

    pub async fn initiate(&self, user_id: Uuid) -> Result<InitiateResponse> {
        let state = Uuid::new_v4().to_string();
        self.state_cache.insert(state.clone(), user_id).await;
        let url = format!(
            "{}?client_id={}&state={}",
            AUTH_URL,
            self.app_id,
            state
        );
        Ok(InitiateResponse {
            authorization_url: url,
        })
    }

    pub async fn callback(&self, code: String, state: String) -> Result<Uuid> {
        let user_id = self
            .state_cache
            .get(&state)
            .await
            .ok_or(AppError::BadRequest("invalid or expired state".into()))?;
        self.state_cache.remove(&state).await;

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("client_id", self.app_id.as_str()),
            ("client_secret", self.app_secret.as_str()),
            ("redirect_uri", self.callback_url.as_str()),
        ];
        let resp = self
            .http
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::Upstream(e.to_string()))?;
        let _status = resp.status();
        let body: BiliTokenResponse = resp
            .json()
            .await
            .map_err(|e| AppError::Upstream(e.to_string()))?;
        if body.code != 0 {
            return Err(AppError::Upstream(
                body.message.unwrap_or_else(|| "bilibili token error".into()),
            ));
        }
        let data = body.data.ok_or(AppError::Upstream("no data in response".into()))?;
        let expires_at = Utc::now() + chrono::Duration::seconds(data.expires_in);
        let refresh_expires_at = data
            .refresh_expires_in
            .map(|s| Utc::now() + chrono::Duration::seconds(s));

        // 换取 token 后调用 B站 userinfo API 获取 bid、user_name
        let (bid, user_name) = match self
            .http
            .get(USERINFO_URL)
            .header("Authorization", format!("Bearer {}", data.access_token))
            .send()
            .await
        {
            Ok(resp) => {
                let ui_resp: BiliUserinfoResponse = resp
                    .json()
                    .await
                    .unwrap_or(BiliUserinfoResponse {
                        code: -1,
                        message: Some("parse error".into()),
                        data: None,
                    });
                ui_resp
                    .data
                    .map(|d| (d.bid, d.user_name))
                    .unwrap_or((None, None))
            }
            Err(_) => (None, None),
        };

        self.vault
            .store(
                user_id,
                "bilibili",
                &data.access_token,
                &data.refresh_token,
                expires_at,
                refresh_expires_at,
                bid,
                user_name,
            )
            .await?;
        Ok(user_id)
    }

    pub async fn status(&self, user_id: Uuid) -> Result<Vec<TokenMeta>> {
        self.vault.list_by_user(user_id).await
    }

    /// List bound ad accounts for the user (from ad_account_bindings).
    pub async fn list_bindings(&self, user_id: Uuid) -> Result<Vec<BoundAccountRecord>> {
        use crate::entities::ad_account_binding;
        let rows = ad_account_binding::Entity::find()
            .filter(ad_account_binding::Column::UserId.eq(user_id))
            .filter(ad_account_binding::Column::Platform.eq("bilibili"))
            .order_by_asc(ad_account_binding::Column::CreatedAt)
            .all(&self.db)
            .await?;
        let out: Vec<BoundAccountRecord> = rows
            .into_iter()
            .map(|r| {
                let account_id = r.account_id.parse().unwrap_or(0_i64);
                BoundAccountRecord {
                    account_id,
                    account_name: r.account_name,
                    platform: r.platform,
                    created_at: r.created_at.to_rfc3339(),
                }
            })
            .collect();
        Ok(out)
    }

    pub async fn delete_session(&self, user_id: Uuid, token_id: Uuid) -> Result<bool> {
        self.vault.delete(token_id, user_id).await
    }

    /// List advertisers (accounts) from B站 for the given token. If oauth_session_id is None, use latest token for user.
    pub async fn list_advertisers(
        &self,
        user_id: Uuid,
        oauth_session_id: Option<Uuid>,
    ) -> Result<BiliAdvertisersResponse> {
        let token_id = match oauth_session_id {
            Some(id) => id,
            None => {
                let list = self.vault.list_by_user(user_id).await?;
                list.into_iter()
                    .next()
                    .map(|m| m.id)
                    .ok_or(AppError::BadRequest("no OAuth session; please authorize first".into()))?
            }
        };
        let stored = self
            .vault
            .get(token_id)
            .await?
            .ok_or(AppError::NotFound("oauth_token".into()))?;
        let resp = self
            .http
            .get(ADVERTISERS_URL)
            .query(&[("page", "1"), ("size", "100")])
            .header("Authorization", format!("Bearer {}", stored.access_token))
            .send()
            .await
            .map_err(|e| AppError::Upstream(e.to_string()))?;
        let text = resp.text().await.map_err(|e| AppError::Upstream(e.to_string()))?;
        let body: BiliAdvertisersResponse = serde_json::from_str(&text).map_err(|e| {
            let snippet = text.chars().take(500).collect::<String>();
            AppError::Upstream(format!(
                "error decoding response body: {}; body snippet: {}",
                e, snippet
            ))
        })?;
        Ok(body)
    }

    /// Get access_token for (user_id, account_id). Refreshes if expiring within 5 minutes.
    pub async fn get_access_token_for_account(&self, user_id: Uuid, account_id: i64) -> Result<String> {
        let account_id_str = account_id.to_string();
        tracing::debug!(
            user_id = %user_id,
            platform = "bilibili",
            account_id = %account_id_str,
            "querying ad_account_bindings"
        );
        let binding = ad_account_binding::Entity::find()
            .filter(ad_account_binding::Column::UserId.eq(user_id))
            .filter(ad_account_binding::Column::Platform.eq("bilibili"))
            .filter(ad_account_binding::Column::AccountId.eq(&account_id_str))
            .one(&self.db)
            .await?;
        if binding.is_none() {
            tracing::warn!(
                user_id = %user_id,
                platform = "bilibili",
                account_id = %account_id_str,
                "ad_account_bindings 查询结果为空，请核对：1) user_id 是否与绑定时的用户一致 2) account_id 类型/值是否与绑定列表中的一致"
            );
        }
        let binding = binding.ok_or(AppError::Forbidden("account not bound".into()))?;
        let token_id = binding.oauth_token_id;
        let stored = self
            .vault
            .get(token_id)
            .await?
            .ok_or(AppError::NotFound("oauth_token".into()))?;
        let exp_in = stored.expires_at.timestamp_millis() - Utc::now().timestamp_millis();
        if exp_in < 5 * 60 * 1000 {
            let _ = self.refresh_one(token_id, user_id).await;
            let stored = self.vault.get(token_id).await?.ok_or(AppError::Internal(anyhow::anyhow!("token gone")))?;
            return Ok(stored.access_token);
        }
        Ok(stored.access_token)
    }

    /// 解绑单个账户。account_id 属于当前用户且已绑定。
    pub async fn unbind_account(&self, user_id: Uuid, account_id: i64) -> Result<bool> {
        let account_id_str = account_id.to_string();
        let binding = ad_account_binding::Entity::find()
            .filter(ad_account_binding::Column::UserId.eq(user_id))
            .filter(ad_account_binding::Column::Platform.eq("bilibili"))
            .filter(ad_account_binding::Column::AccountId.eq(&account_id_str))
            .one(&self.db)
            .await?;
        if let Some(b) = binding {
            b.delete(&self.db).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Bind account_ids to the given oauth_token for this user.
    pub async fn bind_accounts(
        &self,
        user_id: Uuid,
        oauth_token_id: Uuid,
        account_ids: Vec<i64>,
        account_names: Option<Vec<String>>,
    ) -> Result<()> {
        let meta = self.vault.list_by_user(user_id).await?;
        if !meta.iter().any(|m| m.id == oauth_token_id) {
            return Err(AppError::Forbidden("oauth_token not owned by user".into()));
        }

        let current_count = ad_account_binding::Entity::find()
            .filter(ad_account_binding::Column::UserId.eq(user_id))
            .filter(ad_account_binding::Column::Platform.eq("bilibili"))
            .count(&self.db)
            .await? as u32;

        let new_account_ids: std::collections::HashSet<_> = account_ids.iter().copied().collect();
        let existing_ids: std::collections::HashSet<_> = ad_account_binding::Entity::find()
            .filter(ad_account_binding::Column::UserId.eq(user_id))
            .filter(ad_account_binding::Column::Platform.eq("bilibili"))
            .all(&self.db)
            .await?
            .into_iter()
            .filter_map(|r| r.account_id.parse::<i64>().ok())
            .collect();
        let added_count = new_account_ids.difference(&existing_ids).count() as u32;
        if current_count + added_count > self.max_accounts_per_user {
            return Err(AppError::AccountLimitExceeded(self.max_accounts_per_user));
        }
        let names: Vec<Option<String>> = account_names
            .map(|v| v.into_iter().map(Some).collect())
            .unwrap_or_else(|| (0..account_ids.len()).map(|_| None).collect());
        for (i, &account_id) in account_ids.iter().enumerate() {
            let account_id_str = account_id.to_string();
            let account_name = names.get(i).and_then(|s| s.clone());
            let existing = ad_account_binding::Entity::find()
                .filter(ad_account_binding::Column::UserId.eq(user_id))
                .filter(ad_account_binding::Column::Platform.eq("bilibili"))
                .filter(ad_account_binding::Column::AccountId.eq(&account_id_str))
                .one(&self.db)
                .await?;
            let now = Utc::now();
            if let Some(ex) = existing {
                let mut active: ad_account_binding::ActiveModel = ex.into();
                active.oauth_token_id = Set(oauth_token_id);
                active.account_name = Set(account_name);
                active.save(&self.db).await?;
            } else {
                ad_account_binding::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    user_id: Set(user_id),
                    platform: Set("bilibili".to_string()),
                    account_id: Set(account_id_str),
                    oauth_token_id: Set(oauth_token_id),
                    account_name: Set(account_name),
                    created_at: Set(now.into()),
                    ..Default::default()
                }
                .insert(&self.db)
                .await?;
            }
        }
        Ok(())
    }

    /// Refresh a single token (used by token_refresh_job and optionally before MAPI call).

    /// Run one full refresh cycle at startup: check all tokens (expiring within 365 days) and refresh those that need it.
    pub async fn run_refresh_cycle_full(&self) {
        let within = chrono::Duration::days(365);
        let list = match self.vault.list_expiring_soon(within).await {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!("token_vault list_expiring_soon (full): {}", e);
                return;
            }
        };
        for (token_id, user_id) in list {
            if let Err(e) = self.refresh_one(token_id, user_id).await {
                tracing::warn!("refresh oauth_token {} failed: {}", token_id, e);
            }
        }
    }

    /// Run one refresh cycle: find tokens expiring within 11 hours and refresh them.
    pub async fn run_refresh_cycle(&self) {
        let within = chrono::Duration::hours(11);
        let list = match self.vault.list_expiring_soon(within).await {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!("token_vault list_expiring_soon: {}", e);
                return;
            }
        };
        for (token_id, user_id) in list {
            if let Err(e) = self.refresh_one(token_id, user_id).await {
                tracing::warn!("refresh oauth_token {} failed: {}", token_id, e);
            }
        }
    }

    pub async fn refresh_one(&self, token_id: Uuid, _user_id: Uuid) -> Result<()> {
        let stored = self
            .vault
            .get(token_id)
            .await?
            .ok_or(AppError::NotFound("oauth_token".into()))?;
        let params = [
            ("grant_type", "refresh_token"),
            ("client_id", self.app_id.as_str()),
            ("client_secret", self.app_secret.as_str()),
            ("refresh_token", stored.refresh_token.as_str()),
        ];
        let resp = self
            .http
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::Upstream(e.to_string()))?;
        let body: BiliTokenResponse = resp
            .json()
            .await
            .map_err(|e| AppError::Upstream(e.to_string()))?;
        if body.code != 0 {
            return Err(AppError::Upstream(
                body.message.unwrap_or_else(|| "bilibili refresh error".into()),
            ));
        }
        let data = body.data.ok_or(AppError::Upstream("no data".into()))?;
        let expires_at = Utc::now() + chrono::Duration::seconds(data.expires_in);
        let refresh_expires_at = data
            .refresh_expires_in
            .map(|s| Utc::now() + chrono::Duration::seconds(s));
        self.vault
            .update(
                token_id,
                &data.access_token,
                &data.refresh_token,
                expires_at,
                refresh_expires_at,
            )
            .await
    }
}
