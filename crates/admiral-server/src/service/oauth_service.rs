//! Bilibili OAuth: initiate, callback (exchange token), status, delete, list advertisers, bind accounts.

use crate::entities::{ad_account_binding, oauth_subject};
use crate::error::{AppError, Result};
use admiral_types::api::{BoundAccountRecord, BiliSubjectRecord, InitiateResponse, TokenMeta};
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
const SUBJECTS_URL: &str = "https://cm.bilibili.com/takumi/api/open_api/oauth2/subjects";
const ADVERTISERS_URL: &str = "https://cm.bilibili.com/takumi/api/open_api/oauth2/advertisers";

/// external_request_log 中 oauth 各类请求的 kind
const EXTERNAL_LOG_KIND_OAUTH_REFRESH: &str = "oauth_refresh";
const EXTERNAL_LOG_KIND_OAUTH_TOKEN_EXCHANGE: &str = "oauth_token_exchange";
const EXTERNAL_LOG_KIND_OAUTH_USERINFO: &str = "oauth_userinfo";
const EXTERNAL_LOG_KIND_OAUTH_SUBJECTS: &str = "oauth_subjects";
const EXTERNAL_LOG_KIND_OAUTH_ADVERTISERS: &str = "oauth_advertisers";
/// 刷新响应体落表最大长度（字节），UTF-8 安全截断
const OAUTH_REFRESH_RESPONSE_BODY_LOG_MAX: usize = 4096;
/// callback / userinfo / advertisers 响应体落表最大长度（2KB）
const OAUTH_CALLBACK_RESPONSE_BODY_LOG_MAX: usize = 2048;

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
struct BiliSubjectItem {
    pub subject_type: i32,
    pub subject_id: i32,
}

#[derive(Debug, Deserialize)]
struct BiliSubjectsResponse {
    pub code: i32,
    pub message: Option<String>,
    pub data: Option<Vec<BiliSubjectItem>>,
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
        let request_body_log = serde_json::json!({
            "grant_type": "authorization_code",
            "client_id": self.app_id,
            "redirect_uri": self.callback_url,
        })
        .to_string();

        let resp = self
            .http
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::Upstream(e.to_string()))?;
        let status = resp.status().as_u16();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::Upstream(e.to_string()))?;
        let raw = String::from_utf8_lossy(&bytes).to_string();
        let response_body_log = if raw.len() > OAUTH_CALLBACK_RESPONSE_BODY_LOG_MAX {
            format!(
                "{}... (truncated)",
                crate::utils::truncate_utf8_at(&raw, OAUTH_CALLBACK_RESPONSE_BODY_LOG_MAX)
            )
        } else {
            raw.clone()
        };

        let body_result: Result<BiliTokenResponse, _> = serde_json::from_slice(bytes.as_ref());
        let body = match body_result {
            Err(e) => {
                let err_msg = format!("error decoding response body: {}", e);
                let _ = crate::service::log_service::log_external_request(
                    &self.db,
                    EXTERNAL_LOG_KIND_OAUTH_TOKEN_EXCHANGE,
                    TOKEN_URL,
                    "POST",
                    Some(&request_body_log),
                    Some(status),
                    Some(&response_body_log),
                    Some(&err_msg),
                    Some(user_id),
                )
                .await;
                return Err(AppError::Upstream(err_msg));
            }
            Ok(b) => b,
        };
        if body.code != 0 {
            let err_msg = body
                .message
                .clone()
                .unwrap_or_else(|| "bilibili token error".into());
            let _ = crate::service::log_service::log_external_request(
                &self.db,
                EXTERNAL_LOG_KIND_OAUTH_TOKEN_EXCHANGE,
                TOKEN_URL,
                "POST",
                Some(&request_body_log),
                Some(status),
                Some(&response_body_log),
                Some(&err_msg),
                Some(user_id),
            )
            .await;
            return Err(AppError::Upstream(err_msg));
        }
        let data = match body.data {
            Some(d) => d,
            None => {
                let _ = crate::service::log_service::log_external_request(
                    &self.db,
                    EXTERNAL_LOG_KIND_OAUTH_TOKEN_EXCHANGE,
                    TOKEN_URL,
                    "POST",
                    Some(&request_body_log),
                    Some(status),
                    Some(&response_body_log),
                    Some("no data in response"),
                    Some(user_id),
                )
                .await;
                return Err(AppError::Upstream("no data in response".into()));
            }
        };
        let success_body_log = serde_json::json!({
            "code": body.code,
            "message": body.message,
            "data": { "expires_in": data.expires_in, "refresh_expires_in": data.refresh_expires_in },
        });
        let success_str = success_body_log.to_string();
        let success_truncated = if success_str.len() > OAUTH_CALLBACK_RESPONSE_BODY_LOG_MAX {
            format!(
                "{}... (truncated)",
                crate::utils::truncate_utf8_at(&success_str, OAUTH_CALLBACK_RESPONSE_BODY_LOG_MAX)
            )
        } else {
            success_str
        };
        let _ = crate::service::log_service::log_external_request(
            &self.db,
            EXTERNAL_LOG_KIND_OAUTH_TOKEN_EXCHANGE,
            TOKEN_URL,
            "POST",
            Some(&request_body_log),
            Some(status),
            Some(&success_truncated),
            None,
            Some(user_id),
        )
        .await;
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
                let status = resp.status().as_u16();
                let bytes = resp.bytes().await.unwrap_or_default();
                let raw = String::from_utf8_lossy(&bytes).to_string();
                let response_body_log = if raw.len() > OAUTH_CALLBACK_RESPONSE_BODY_LOG_MAX {
                    format!(
                        "{}... (truncated)",
                        crate::utils::truncate_utf8_at(&raw, OAUTH_CALLBACK_RESPONSE_BODY_LOG_MAX)
                    )
                } else {
                    raw.clone()
                };
                let ui_result: Result<BiliUserinfoResponse, _> = serde_json::from_slice(bytes.as_ref());
                match ui_result {
                    Err(e) => {
                        let err_msg = format!("error decoding response body: {}", e);
                        let _ = crate::service::log_service::log_external_request(
                            &self.db,
                            EXTERNAL_LOG_KIND_OAUTH_USERINFO,
                            USERINFO_URL,
                            "GET",
                            None,
                            Some(status),
                            Some(&response_body_log),
                            Some(&err_msg),
                            Some(user_id),
                        )
                        .await;
                        (None, None)
                    }
                    Ok(ui_resp) => {
                        let err_msg = if ui_resp.code != 0 {
                            ui_resp
                                .message
                                .clone()
                                .unwrap_or_else(|| "bilibili userinfo error".into())
                        } else {
                            String::new()
                        };
                        let _ = crate::service::log_service::log_external_request(
                            &self.db,
                            EXTERNAL_LOG_KIND_OAUTH_USERINFO,
                            USERINFO_URL,
                            "GET",
                            None,
                            Some(status),
                            Some(&response_body_log),
                            if err_msg.is_empty() { None } else { Some(&err_msg) },
                            Some(user_id),
                        )
                        .await;
                        ui_resp
                            .data
                            .map(|d| (d.bid, d.user_name))
                            .unwrap_or((None, None))
                    }
                }
            }
            Err(e) => {
                let err_msg = e.to_string();
                let _ = crate::service::log_service::log_external_request(
                    &self.db,
                    EXTERNAL_LOG_KIND_OAUTH_USERINFO,
                    USERINFO_URL,
                    "GET",
                    None,
                    None,
                    None,
                    Some(&err_msg),
                    Some(user_id),
                )
                .await;
                (None, None)
            }
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
            .order_by_desc(ad_account_binding::Column::CreatedAt)
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

    /// List subjects (组织) for the given token. 拉取后写入 oauth_subjects 表。
    pub async fn list_subjects(
        &self,
        user_id: Uuid,
        oauth_session_id: Option<Uuid>,
    ) -> Result<Vec<BiliSubjectRecord>> {
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
        let request_url = format!("{}?page=1&size=100", SUBJECTS_URL);
        let resp = self
            .http
            .get(SUBJECTS_URL)
            .query(&[("page", "1"), ("size", "100")])
            .header("Authorization", format!("Bearer {}", stored.access_token))
            .send()
            .await
            .map_err(|e| AppError::Upstream(e.to_string()))?;
        let status = resp.status().as_u16();
        let text = resp.text().await.map_err(|e| AppError::Upstream(e.to_string()))?;
        let response_body_log = if text.len() > OAUTH_CALLBACK_RESPONSE_BODY_LOG_MAX {
            format!(
                "{}... (truncated)",
                crate::utils::truncate_utf8_at(&text, OAUTH_CALLBACK_RESPONSE_BODY_LOG_MAX)
            )
        } else {
            text.clone()
        };
        let body_result: Result<BiliSubjectsResponse, _> = serde_json::from_str(&text);
        match body_result {
            Err(e) => {
                let err_msg = format!("error decoding response body: {}", e);
                let _ = crate::service::log_service::log_external_request(
                    &self.db,
                    EXTERNAL_LOG_KIND_OAUTH_SUBJECTS,
                    &request_url,
                    "GET",
                    None,
                    Some(status),
                    Some(&response_body_log),
                    Some(&err_msg),
                    Some(user_id),
                )
                .await;
                Err(AppError::Upstream(err_msg))
            }
            Ok(body) => {
                if body.code != 0 {
                    let err_msg = body
                        .message
                        .clone()
                        .unwrap_or_else(|| "bilibili subjects error".into());
                    let _ = crate::service::log_service::log_external_request(
                        &self.db,
                        EXTERNAL_LOG_KIND_OAUTH_SUBJECTS,
                        &request_url,
                        "GET",
                        None,
                        Some(status),
                        Some(&response_body_log),
                        Some(&err_msg),
                        Some(user_id),
                    )
                    .await;
                    return Err(AppError::Upstream(err_msg));
                }
                let _ = crate::service::log_service::log_external_request(
                    &self.db,
                    EXTERNAL_LOG_KIND_OAUTH_SUBJECTS,
                    &request_url,
                    "GET",
                    None,
                    Some(status),
                    Some(&response_body_log),
                    None,
                    Some(user_id),
                )
                .await;
                let items = body.data.unwrap_or_default();
                for item in &items {
                    let existing = oauth_subject::Entity::find()
                        .filter(oauth_subject::Column::OauthTokenId.eq(token_id))
                        .filter(oauth_subject::Column::SubjectType.eq(item.subject_type))
                        .filter(oauth_subject::Column::SubjectId.eq(item.subject_id))
                        .one(&self.db)
                        .await?;
                    if existing.is_none() {
                        oauth_subject::ActiveModel {
                            id: Set(Uuid::new_v4()),
                            oauth_token_id: Set(token_id),
                            subject_type: Set(item.subject_type),
                            subject_id: Set(item.subject_id),
                            created_at: Set(Utc::now().into()),
                            ..Default::default()
                        }
                        .insert(&self.db)
                        .await?;
                    }
                }
                Ok(items
                    .into_iter()
                    .map(|i| BiliSubjectRecord {
                        subject_type: i.subject_type,
                        subject_id: i.subject_id,
                    })
                    .collect())
            }
        }
    }

    /// List advertisers (accounts) from B站 for the given token and subject. 必须先选 subject。
    pub async fn list_advertisers(
        &self,
        user_id: Uuid,
        oauth_session_id: Option<Uuid>,
        subject_type: i32,
        subject_id: i32,
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
        let request_url = format!(
            "{}?page=1&size=100&subject_type={}&subject_id={}",
            ADVERTISERS_URL, subject_type, subject_id
        );
        let resp = self
            .http
            .get(ADVERTISERS_URL)
            .query(&[
                ("page", "1"),
                ("size", "100"),
                ("subject_type", subject_type.to_string().as_str()),
                ("subject_id", subject_id.to_string().as_str()),
            ])
            .header("Authorization", format!("Bearer {}", stored.access_token))
            .send()
            .await
            .map_err(|e| AppError::Upstream(e.to_string()))?;
        let status = resp.status().as_u16();
        let text = resp.text().await.map_err(|e| AppError::Upstream(e.to_string()))?;
        let response_body_log = if text.len() > OAUTH_CALLBACK_RESPONSE_BODY_LOG_MAX {
            format!(
                "{}... (truncated)",
                crate::utils::truncate_utf8_at(&text, OAUTH_CALLBACK_RESPONSE_BODY_LOG_MAX)
            )
        } else {
            text.clone()
        };
        let body_result: Result<BiliAdvertisersResponse, _> = serde_json::from_str(&text);
        match body_result {
            Err(e) => {
                let err_msg = format!("error decoding response body: {}", e);
                let _ = crate::service::log_service::log_external_request(
                    &self.db,
                    EXTERNAL_LOG_KIND_OAUTH_ADVERTISERS,
                    &request_url,
                    "GET",
                    None,
                    Some(status),
                    Some(&response_body_log),
                    Some(&err_msg),
                    Some(user_id),
                )
                .await;
                Err(AppError::Upstream(err_msg))
            }
            Ok(body) => {
                let err_msg = if body.code != 0 {
                    body.message
                        .clone()
                        .unwrap_or_else(|| "bilibili advertisers error".into())
                } else {
                    String::new()
                };
                let _ = crate::service::log_service::log_external_request(
                    &self.db,
                    EXTERNAL_LOG_KIND_OAUTH_ADVERTISERS,
                    &request_url,
                    "GET",
                    None,
                    Some(status),
                    Some(&response_body_log),
                    if err_msg.is_empty() { None } else { Some(&err_msg) },
                    Some(user_id),
                )
                .await;
                Ok(body)
            }
        }
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

    /// Bind account_ids to the given oauth_token for this user，并记录所选 subject。
    pub async fn bind_accounts(
        &self,
        user_id: Uuid,
        oauth_token_id: Uuid,
        subject_type: i32,
        subject_id: i32,
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
            .map(|v| {
                v.into_iter()
                    .map(|s| if s.is_empty() { None } else { Some(s) })
                    .collect()
            })
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
                active.subject_type = Set(Some(subject_type));
                active.subject_id = Set(Some(subject_id));
                active.save(&self.db).await?;
            } else {
                ad_account_binding::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    user_id: Set(user_id),
                    platform: Set("bilibili".to_string()),
                    account_id: Set(account_id_str),
                    oauth_token_id: Set(oauth_token_id),
                    account_name: Set(account_name),
                    subject_type: Set(Some(subject_type)),
                    subject_id: Set(Some(subject_id)),
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

    pub async fn refresh_one(&self, token_id: Uuid, user_id: Uuid) -> Result<()> {
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
        // 脱敏请求体：仅 grant_type、client_id，不记录 client_secret/refresh_token
        let request_body_log = serde_json::json!({
            "grant_type": "refresh_token",
            "client_id": self.app_id,
        })
        .to_string();

        let resp = self
            .http
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::Upstream(e.to_string()))?;
        let status = resp.status().as_u16();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::Upstream(e.to_string()))?;
        let raw = String::from_utf8_lossy(&bytes).to_string();
        let response_body_log = if raw.len() > OAUTH_REFRESH_RESPONSE_BODY_LOG_MAX {
            format!(
                "{}... (truncated)",
                crate::utils::truncate_utf8_at(&raw, OAUTH_REFRESH_RESPONSE_BODY_LOG_MAX)
            )
        } else {
            raw.clone()
        };

        let body_result: Result<BiliTokenResponse, _> = serde_json::from_slice(bytes.as_ref());
        match body_result {
            Err(e) => {
                let err_msg = format!("error decoding response body: {}", e);
                let _ = crate::service::log_service::log_external_request(
                    &self.db,
                    EXTERNAL_LOG_KIND_OAUTH_REFRESH,
                    TOKEN_URL,
                    "POST",
                    Some(&request_body_log),
                    Some(status),
                    Some(&response_body_log),
                    Some(&err_msg),
                    Some(user_id),
                )
                .await;
                return Err(AppError::Upstream(err_msg));
            }
            Ok(body) => {
                if body.code != 0 {
                    let err_msg = body
                        .message
                        .clone()
                        .unwrap_or_else(|| "bilibili refresh error".into());
                    let code_err_body = serde_json::json!({
                        "code": body.code,
                        "message": body.message,
                    })
                    .to_string();
                    let _ = crate::service::log_service::log_external_request(
                        &self.db,
                        EXTERNAL_LOG_KIND_OAUTH_REFRESH,
                        TOKEN_URL,
                        "POST",
                        Some(&request_body_log),
                        Some(status),
                        Some(&code_err_body),
                        Some(&err_msg),
                        Some(user_id),
                    )
                    .await;
                    return Err(AppError::Upstream(err_msg));
                }
                let data = body.data.ok_or(AppError::Upstream("no data".into()))?;
                // 成功时落表用脱敏 JSON（不含 access_token/refresh_token）
                let success_body_log = serde_json::json!({
                    "code": body.code,
                    "message": body.message,
                    "data": {
                        "expires_in": data.expires_in,
                        "refresh_expires_in": data.refresh_expires_in,
                    },
                });
                let success_body_str = success_body_log.to_string();
                let success_body_truncated = if success_body_str.len() > OAUTH_REFRESH_RESPONSE_BODY_LOG_MAX {
                    format!(
                        "{}... (truncated)",
                        crate::utils::truncate_utf8_at(
                            &success_body_str,
                            OAUTH_REFRESH_RESPONSE_BODY_LOG_MAX,
                        )
                    )
                } else {
                    success_body_str
                };
                let _ = crate::service::log_service::log_external_request(
                    &self.db,
                    EXTERNAL_LOG_KIND_OAUTH_REFRESH,
                    TOKEN_URL,
                    "POST",
                    Some(&request_body_log),
                    Some(status),
                    Some(&success_body_truncated),
                    None,
                    Some(user_id),
                )
                .await;
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
    }
}
