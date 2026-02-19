//! LLM proxy: single default provider, forward to OpenAI-compatible or provider-specific API.

use crate::error::{AppError, Result};
use reqwest::Client;
use serde_json::Value;

#[derive(Clone)]
pub struct LlmProxy {
    pub base_url: String,
    pub api_key: String,
    pub provider: String,
    http: Client,
}

impl LlmProxy {
    pub fn new(provider: &str, base_url: String, api_key: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            provider: provider.to_string(),
            http: Client::new(),
        }
    }

    /// Chat completion (non-stream). Returns the JSON response body.
    pub async fn chat(&self, body: &Value) -> Result<Vec<u8>> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| AppError::Upstream(e.to_string()))?;
        let status = resp.status();
        let bytes = resp.bytes().await.map_err(|e| AppError::Upstream(e.to_string()))?;
        if !status.is_success() {
            return Err(AppError::Upstream(
                String::from_utf8_lossy(&bytes).to_string(),
            ));
        }
        Ok(bytes.to_vec())
    }

    /// Chat completion (stream). Returns the response body stream.
    pub async fn chat_stream(&self, body: &Value) -> Result<reqwest::Response> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| AppError::Upstream(e.to_string()))?;
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Upstream(body));
        }
        Ok(resp)
    }
}

/// Build LlmProxy from provider name and optional env keys. Returns None if provider has no key.
///
/// 联调阶段仅启用 DeepSeek（deepseek-chat），其他厂商暂不解析。
pub fn from_env(
    default_provider: &str,
    deepseek_key: Option<&str>,
    _openai_key: Option<&str>,
    _openai_base: Option<&str>,
    _doubao_key: Option<&str>,
    _doubao_base: Option<&str>,
    _gemini_key: Option<&str>,
) -> Option<LlmProxy> {
    // 联调阶段仅 deepseek；其他厂商屏蔽，后续再开放
    if default_provider.to_lowercase().as_str() != "deepseek" {
        return None;
    }
    let (base_url, api_key) = deepseek_key.map(|k| ("https://api.deepseek.com".to_string(), k.to_string()))?;
    Some(LlmProxy::new("deepseek", base_url, api_key))
}
