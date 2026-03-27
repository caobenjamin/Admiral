//! REST API client: auth, OAuth, MAPI proxy, LLM chat.

use admiral_types::api::*;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("serialization: {0}")]
    Serialization(String),
    #[error("credential: {0}")]
    Credential(String),
    #[error("server: {code} - {message}")]
    Server { status: u16, code: String, message: String },
    #[error("unauthorized")]
    Unauthorized,
}

/// REST client for Admiral server API.
pub struct ApiClient {
    base_url: String,
    client: Client,
}

#[derive(Deserialize)]
struct ErrorBody {
    #[serde(default)]
    code: String,
    #[serde(default)]
    message: String,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client");
        Self { base_url, client }
    }

    fn url(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{}{}", base, path)
    }

    async fn check_response(&self, resp: reqwest::Response) -> Result<reqwest::Response, ApiError> {
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        let status_code = status.as_u16();
        let body = resp.text().await.unwrap_or_default();
        let (code, message) = if let Ok(b) = serde_json::from_str::<ErrorBody>(&body) {
            (b.code, b.message)
        } else {
            ("UNKNOWN".to_string(), body)
        };
        if status_code == 401 {
            return Err(ApiError::Unauthorized);
        }
        Err(ApiError::Server {
            status: status_code,
            code,
            message,
        })
    }

    /// Call with optional Bearer token. Returns response or ApiError (including Unauthorized for 401).
    pub async fn request_with_token(
        &self,
        method: reqwest::Method,
        path: &str,
        token: Option<&str>,
        body: Option<serde_json::Value>,
    ) -> Result<reqwest::Response, ApiError> {
        let url = self.url(path);
        let mut req = self.client.request(method, &url);
        if let Some(t) = token {
            req = req.header("Authorization", format!("Bearer {}", t));
        }
        req = req.header("Content-Type", "application/json");
        if let Some(b) = body {
            req = req.json(&b);
        }
        let resp = req.send().await?;
        self.check_response(resp).await
    }

    // ---------- Auth ----------

    pub async fn register(&self, req: &RegisterRequest) -> Result<TokenResponse, ApiError> {
        let path = "/api/v1/auth/register";
        let body = serde_json::to_value(req).map_err(|e| ApiError::Serialization(e.to_string()))?;
        let resp = self
            .request_with_token(reqwest::Method::POST, path, None, Some(body))
            .await?;
        let out: TokenResponse = resp.json().await?;
        Ok(out)
    }

    pub async fn login(&self, req: &LoginRequest) -> Result<TokenResponse, ApiError> {
        let path = "/api/v1/auth/login";
        let body = serde_json::to_value(req).map_err(|e| ApiError::Serialization(e.to_string()))?;
        let resp = self
            .request_with_token(reqwest::Method::POST, path, None, Some(body))
            .await?;
        let out: TokenResponse = resp.json().await?;
        Ok(out)
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<RefreshResponse, ApiError> {
        let path = "/api/v1/auth/refresh";
        let body = serde_json::json!({ "refresh_token": refresh_token });
        let resp = self
            .request_with_token(reqwest::Method::POST, path, None, Some(body))
            .await?;
        let out: RefreshResponse = resp.json().await?;
        Ok(out)
    }

    pub async fn me(&self, access_token: &str) -> Result<UserInfo, ApiError> {
        let path = "/api/v1/auth/me";
        let resp = self
            .request_with_token(reqwest::Method::GET, path, Some(access_token), None)
            .await?;
        let out: UserInfo = resp.json().await?;
        Ok(out)
    }

    pub async fn license(&self, access_token: &str) -> Result<LicenseResponse, ApiError> {
        let path = "/api/v1/auth/license";
        let resp = self
            .request_with_token(reqwest::Method::GET, path, Some(access_token), None)
            .await?;
        let out: LicenseResponse = resp.json().await?;
        Ok(out)
    }

    // ---------- OAuth ----------

    pub async fn oauth_initiate(&self, access_token: &str) -> Result<InitiateResponse, ApiError> {
        let path = "/api/v1/oauth/bilibili/initiate";
        let resp = self
            .request_with_token(reqwest::Method::POST, path, Some(access_token), None)
            .await?;
        let out: InitiateResponse = resp.json().await?;
        Ok(out)
    }

    pub async fn oauth_status(&self, access_token: &str) -> Result<Vec<TokenMeta>, ApiError> {
        let path = "/api/v1/oauth/bilibili/status";
        let resp = self
            .request_with_token(reqwest::Method::GET, path, Some(access_token), None)
            .await?;
        let out: Vec<TokenMeta> = resp.json().await?;
        Ok(out)
    }

    pub async fn oauth_bindings_list(
        &self,
        access_token: &str,
    ) -> Result<Vec<admiral_types::api::BoundAccountRecord>, ApiError> {
        let path = "/api/v1/oauth/bilibili/bindings";
        let resp = self
            .request_with_token(reqwest::Method::GET, path, Some(access_token), None)
            .await?;
        let out: Vec<admiral_types::api::BoundAccountRecord> = resp.json().await?;
        Ok(out)
    }

    pub async fn oauth_subjects_list(
        &self,
        access_token: &str,
        oauth_session_id: Option<uuid::Uuid>,
    ) -> Result<Vec<admiral_types::api::BiliSubjectRecord>, ApiError> {
        let path = match oauth_session_id {
            Some(id) => format!("/api/v1/oauth/bilibili/subjects?oauth_session_id={}", id),
            None => "/api/v1/oauth/bilibili/subjects".to_string(),
        };
        let resp = self
            .request_with_token(
                reqwest::Method::GET,
                &path,
                Some(access_token),
                None,
            )
            .await?;
        let out: Vec<admiral_types::api::BiliSubjectRecord> = resp.json().await?;
        Ok(out)
    }

    pub async fn oauth_accounts_list(
        &self,
        access_token: &str,
        oauth_session_id: Option<uuid::Uuid>,
        subject_type: i32,
        subject_id: i32,
    ) -> Result<OAuthAccountsResponse, ApiError> {
        let path = match oauth_session_id {
            Some(id) => format!(
                "/api/v1/oauth/bilibili/accounts?oauth_session_id={}&subject_type={}&subject_id={}",
                id, subject_type, subject_id
            ),
            None => format!(
                "/api/v1/oauth/bilibili/accounts?subject_type={}&subject_id={}",
                subject_type, subject_id
            ),
        };
        let resp = self
            .request_with_token(
                reqwest::Method::GET,
                &path,
                Some(access_token),
                None,
            )
            .await?;
        let out: OAuthAccountsResponse = resp.json().await?;
        Ok(out)
    }

    pub async fn oauth_bind_accounts(
        &self,
        access_token: &str,
        req: &BindAccountsRequest,
    ) -> Result<(), ApiError> {
        let path = "/api/v1/oauth/bilibili/bind-accounts";
        let body = serde_json::to_value(req).map_err(|e| ApiError::Serialization(e.to_string()))?;
        self.request_with_token(reqwest::Method::POST, path, Some(access_token), Some(body))
            .await?;
        Ok(())
    }

    pub async fn oauth_delete_session(
        &self,
        access_token: &str,
        session_id: uuid::Uuid,
    ) -> Result<(), ApiError> {
        let path = format!("/api/v1/oauth/bilibili/sessions/{}", session_id);
        self.request_with_token(reqwest::Method::DELETE, &path, Some(access_token), None)
            .await?;
        Ok(())
    }

    pub async fn oauth_unbind_account(
        &self,
        access_token: &str,
        account_id: i64,
    ) -> Result<(), ApiError> {
        let path = format!("/api/v1/oauth/bilibili/bindings/{}", account_id);
        self.request_with_token(reqwest::Method::DELETE, &path, Some(access_token), None)
            .await?;
        Ok(())
    }

    // ---------- MAPI ----------

    pub async fn mapi_proxy(
        &self,
        access_token: &str,
        req: &MapiProxyRequest,
    ) -> Result<(u16, Vec<u8>), ApiError> {
        let path = "/api/v1/mapi/proxy";
        let body = serde_json::to_value(req).map_err(|e| ApiError::Serialization(e.to_string()))?;
        let url = self.url(path);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;
        let status = resp.status().as_u16();
        let bytes = resp.bytes().await?.to_vec();
        let ok = status >= 200 && status < 300;
        if !ok {
            return Err(ApiError::Server {
                status,
                code: "MAPI_PROXY_ERROR".to_string(),
                message: String::from_utf8_lossy(&bytes).to_string(),
            });
        }
        Ok((status, bytes))
    }

    // ---------- LLM ----------

    pub async fn llm_chat(
        &self,
        access_token: &str,
        req: &LlmChatRequest,
    ) -> Result<Vec<u8>, ApiError> {
        let path = "/api/v1/llm/chat";
        let body = serde_json::to_value(req).map_err(|e| ApiError::Serialization(e.to_string()))?;
        let url = self.url(path);
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let bytes = resp.bytes().await?.to_vec();
            return Err(ApiError::Server {
                status,
                code: "LLM_ERROR".to_string(),
                message: String::from_utf8_lossy(&bytes).to_string(),
            });
        }
        let bytes = resp.bytes().await?.to_vec();
        Ok(bytes)
    }

    /// LLM chat streaming: returns the raw response for SSE processing.
    pub async fn llm_chat_stream(
        &self,
        access_token: &str,
        req: &LlmChatRequest,
    ) -> Result<reqwest::Response, ApiError> {
        let path = "/api/v1/llm/chat";
        let mut stream_req = req.clone();
        stream_req.stream = true;
        let body = serde_json::to_value(&stream_req).map_err(|e| ApiError::Serialization(e.to_string()))?;
        let url = self.url(path);
        let resp = self
            .client
            .post(&url)
            .timeout(Duration::from_secs(300))
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let bytes = resp.bytes().await?.to_vec();
            return Err(ApiError::Server {
                status,
                code: "LLM_ERROR".to_string(),
                message: String::from_utf8_lossy(&bytes).to_string(),
            });
        }
        Ok(resp)
    }

    // ---------- Knowledge (RAG) ----------

    pub async fn knowledge_query(
        &self,
        access_token: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<KnowledgeSnippet>, ApiError> {
        let path = "/api/v1/knowledge/query";
        let body = serde_json::json!({ "query": query, "top_k": top_k });
        let resp = self
            .request_with_token(reqwest::Method::POST, path, Some(access_token), Some(body))
            .await?;
        let out: KnowledgeQueryResponse = resp.json().await?;
        Ok(out.snippets)
    }
}

#[derive(serde::Deserialize)]
struct KnowledgeQueryResponse {
    snippets: Vec<KnowledgeSnippet>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct KnowledgeSnippet {
    pub content: String,
    pub source_file: String,
    #[serde(default)]
    pub title: Option<String>,
}
