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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bid: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_name: Option<String>,
}

/// B 站 OAuth 返回的 subject（组织），拉取广告主列表前需先选一个。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BiliSubjectRecord {
    pub subject_type: i32,
    pub subject_id: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BindAccountsRequest {
    pub oauth_token_id: Uuid,
    /// 绑定时必须带所选 subject，以便后续 MAPI 使用
    pub subject_type: i32,
    pub subject_id: i32,
    pub account_ids: Vec<i64>,
    /// 与 account_ids 一一对应的账户名称（来自 B 站 advertisers 列表），用于持久化展示
    #[serde(default)]
    pub account_names: Option<Vec<String>>,
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

/// 已绑定的广告账户
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoundAccountRecord {
    pub account_id: i64,
    pub account_name: Option<String>,
    pub platform: String,
    pub created_at: String,
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
    /// OpenAI-format tools array; when set, LLM may return tool_calls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<serde_json::Value>,
    /// e.g. "auto" or "required"; forwarded to upstream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
    /// Optional structured output schema; forwarded to upstream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<serde_json::Value>,
}
