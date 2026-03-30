//! LLM proxy router: select provider and forward to provider-specific upstream APIs.

use std::collections::HashMap;

use admiral_types::api::{LlmModelInfo, LlmModelsResponse};
use crate::error::{AppError, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use tracing::info;

const BODY_LOG_LIMIT: usize = 2048;
const DEEPSEEK_PROVIDER: &str = "deepseek";
const OPENAI_PROVIDER: &str = "openai";

fn body_preview(v: &Value, limit: usize) -> String {
    match serde_json::to_string(v) {
        Ok(s) => {
            if s.len() > limit {
                format!(
                    "{}... (truncated, {} chars)",
                    crate::utils::truncate_utf8_at(&s, limit),
                    s.len()
                )
            } else {
                s
            }
        }
        Err(_) => "<invalid>".to_string(),
    }
}

#[derive(Clone)]
pub struct LlmProviderClient {
    pub base_url: String,
    pub api_key: String,
    pub provider: String,
    http: Client,
}

impl LlmProviderClient {
    pub fn new(provider: &str, base_url: String, api_key: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            provider: provider.to_string(),
            http: Client::new(),
        }
    }

    pub async fn chat(&self, body: &Value) -> Result<Vec<u8>> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        info!(
            llm_url = %url,
            llm_provider = %self.provider,
            llm_request_body = %body_preview(body, BODY_LOG_LIMIT),
            "LLM external call"
        );
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
        let bytes_slice = bytes.as_ref();
        info!(
            llm_status = %status,
            llm_response_body = %crate::utils::bytes_preview(bytes_slice, BODY_LOG_LIMIT),
            "LLM external response"
        );
        if !status.is_success() {
            return Err(AppError::Upstream(String::from_utf8_lossy(&bytes).to_string()));
        }
        Ok(bytes.to_vec())
    }

    pub async fn chat_stream(&self, body: &Value) -> Result<reqwest::Response> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        info!(
            llm_url = %url,
            llm_provider = %self.provider,
            llm_request_body = %body_preview(body, BODY_LOG_LIMIT),
            llm_stream = true,
            "LLM external call"
        );
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
        info!(
            llm_status = %status,
            llm_stream = true,
            "LLM external response (stream)"
        );
        if !status.is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            return Err(AppError::Upstream(err_body));
        }
        Ok(resp)
    }

    pub async fn list_models(&self) -> Result<LlmModelsResponse> {
        if self.provider == DEEPSEEK_PROVIDER {
            return Ok(LlmModelsResponse {
                provider: self.provider.clone(),
                models: vec![
                    LlmModelInfo {
                        id: "deepseek-chat".to_string(),
                        provider: self.provider.clone(),
                        owned_by: Some(DEEPSEEK_PROVIDER.to_string()),
                        created: None,
                    },
                    LlmModelInfo {
                        id: "deepseek-reasoner".to_string(),
                        provider: self.provider.clone(),
                        owned_by: Some(DEEPSEEK_PROVIDER.to_string()),
                        created: None,
                    },
                ],
            });
        }

        let url = format!("{}/v1/models", self.base_url);
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| AppError::Upstream(e.to_string()))?;
        let status = resp.status();
        let body = resp.bytes().await.map_err(|e| AppError::Upstream(e.to_string()))?;
        if !status.is_success() {
            return Err(AppError::Upstream(String::from_utf8_lossy(&body).to_string()));
        }

        let list: OpenAiModelsList = serde_json::from_slice(&body)
            .map_err(|e| AppError::Upstream(format!("parse models list: {}", e)))?;
        let mut models: Vec<LlmModelInfo> = list
            .data
            .into_iter()
            .map(|model| LlmModelInfo {
                id: model.id,
                provider: self.provider.clone(),
                owned_by: model.owned_by,
                created: model.created,
            })
            .collect();
        models.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(LlmModelsResponse {
            provider: self.provider.clone(),
            models,
        })
    }
}

#[derive(Clone)]
pub struct LlmRouter {
    default_provider: String,
    providers: HashMap<String, LlmProviderClient>,
}

impl LlmRouter {
    pub fn new(default_provider: String, providers: HashMap<String, LlmProviderClient>) -> Self {
        Self {
            default_provider,
            providers,
        }
    }

    pub fn resolve_provider<'a>(&'a self, requested_provider: Option<&str>) -> Result<&'a LlmProviderClient> {
        let provider = requested_provider
            .filter(|p| !p.trim().is_empty())
            .unwrap_or(self.default_provider.as_str());
        self.providers.get(provider).ok_or_else(|| {
            AppError::BadRequest(format!("LLM provider `{}` is not configured on server", provider))
        })
    }
}

#[derive(Deserialize)]
struct OpenAiModelsList {
    data: Vec<OpenAiModel>,
}

#[derive(Deserialize)]
struct OpenAiModel {
    id: String,
    #[serde(default)]
    owned_by: Option<String>,
    #[serde(default)]
    created: Option<i64>,
}

pub fn default_model_for_provider(provider: &str) -> &'static str {
    match provider {
        OPENAI_PROVIDER => "gpt-4.1-mini",
        _ => "deepseek-chat",
    }
}

pub fn from_env(
    default_provider: &str,
    deepseek_key: Option<&str>,
    openai_key: Option<&str>,
) -> Option<LlmRouter> {
    let mut providers = HashMap::new();

    if let Some(api_key) = deepseek_key {
        providers.insert(
            DEEPSEEK_PROVIDER.to_string(),
            LlmProviderClient::new(
                DEEPSEEK_PROVIDER,
                "https://api.deepseek.com".to_string(),
                api_key.to_string(),
            ),
        );
    }

    if let Some(api_key) = openai_key {
        providers.insert(
            OPENAI_PROVIDER.to_string(),
            LlmProviderClient::new(
                OPENAI_PROVIDER,
                "https://api.openai.com".to_string(),
                api_key.to_string(),
            ),
        );
    }

    if providers.is_empty() {
        return None;
    }

    let default_provider = if providers.contains_key(default_provider) {
        default_provider.to_string()
    } else if providers.contains_key(DEEPSEEK_PROVIDER) {
        DEEPSEEK_PROVIDER.to_string()
    } else {
        OPENAI_PROVIDER.to_string()
    };

    Some(LlmRouter::new(default_provider, providers))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{default_model_for_provider, LlmProviderClient, LlmRouter};

    #[test]
    fn default_model_matches_provider() {
        assert_eq!(default_model_for_provider("deepseek"), "deepseek-chat");
        assert_eq!(default_model_for_provider("openai"), "gpt-4.1-mini");
    }

    #[test]
    fn resolve_provider_prefers_requested_provider() {
        let mut providers = HashMap::new();
        providers.insert(
            "deepseek".to_string(),
            LlmProviderClient::new("deepseek", "https://api.deepseek.com".to_string(), "a".to_string()),
        );
        providers.insert(
            "openai".to_string(),
            LlmProviderClient::new("openai", "https://api.openai.com".to_string(), "b".to_string()),
        );
        let router = LlmRouter::new("deepseek".to_string(), providers);

        let resolved = router.resolve_provider(Some("openai")).unwrap();

        assert_eq!(resolved.provider, "openai");
    }
}
