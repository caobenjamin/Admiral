//! REST API request/response types shared between server and client.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------- Auth ----------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: String,
    #[serde(default)]
    pub company: Option<String>,
    pub invite_code: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub user: UserInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub token_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LicenseResponse {
    pub is_active: bool,
    pub expires_at: Option<String>,
    pub r#type: Option<String>,
}

// ---------- OAuth ----------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InitiateResponse {
    pub authorization_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenMeta {
    pub id: Uuid,
    pub platform: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BindAccountsRequest {
    pub oauth_token_id: Uuid,
    pub account_ids: Vec<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAuthAccountsResponse {
    pub records: Vec<BiliAdvertiserRecord>,
    pub total: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BiliAdvertiserRecord {
    pub account_id: i64,
    pub account_name: Option<String>,
    pub account_role: Option<String>,
}

// ---------- MAPI ----------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapiProxyRequest {
    pub account_id: i64,
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub query: Option<serde_json::Value>,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
}

// ---------- LLM ----------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmChatRequest {
    pub messages: serde_json::Value,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}
