//! LLM proxy: POST /api/v1/llm/chat, GET /api/v1/llm/models
//! 请求与响应（含报错）落表 external_request_log。

use admiral_types::api::{LlmChatRequest, LlmModelsResponse};
use axum::{
    body::Body,
    extract::{Query, State},
    response::Response,
    Json,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Deserialize;
use tracing::{info, warn};

use crate::entities::entitlement;
use crate::error::{AppError, Result};
use crate::service::llm_router::{default_model_for_provider, protocol_for_provider, TokenLimitField};
use crate::state::AppState;

const DB_BODY_MAX: usize = 65535;

#[derive(Debug, Deserialize)]
pub struct LlmModelsQuery {
    #[serde(default)]
    provider: Option<String>,
}

async fn ensure_active_entitlement(
    state: &AppState,
    user_id: uuid::Uuid,
) -> Result<()> {
    let entitlement = entitlement::Entity::find()
        .filter(entitlement::Column::UserId.eq(user_id))
        .filter(entitlement::Column::IsActive.eq(true))
        .order_by_desc(entitlement::Column::ExpiresAt)
        .one(&state.db)
        .await?;
    match entitlement {
        Some(ent) if ent.expires_at.timestamp_millis() > chrono::Utc::now().timestamp_millis() => Ok(()),
        Some(ent) => Err(AppError::LicenseInactive {
            expires_at: Some(ent.expires_at.to_rfc3339()),
        }),
        None => Err(AppError::LicenseInactive { expires_at: None }),
    }
}

fn normalize_response_format(
    provider: &str,
    _model: &str,
    response_format: Option<&serde_json::Value>,
) -> Option<serde_json::Value> {
    let response_format = response_format?.clone();
    let protocol = protocol_for_provider(provider);

    if !protocol.supports_json_schema_response_format
        && response_format
            .get("type")
            .and_then(|v| v.as_str())
            .is_some_and(|t| t == "json_schema")
    {
        return Some(serde_json::json!({ "type": "json_object" }));
    }

    Some(response_format)
}

fn resolve_model(req: &LlmChatRequest, provider: &str) -> String {
    match req.model.as_deref() {
        Some(model) if !model.trim().is_empty() && model != "fast" => model.to_string(),
        _ => default_model_for_provider(provider).to_string(),
    }
}

fn apply_token_limit(
    body_obj: &mut serde_json::map::Map<String, serde_json::Value>,
    provider: &str,
    max_tokens: u32,
) {
    match protocol_for_provider(provider).token_limit_field {
        TokenLimitField::MaxCompletionTokens => {
            body_obj.insert(
                "max_completion_tokens".to_string(),
                serde_json::json!(max_tokens),
            );
        }
        TokenLimitField::MaxTokens => {
            body_obj.insert("max_tokens".to_string(), serde_json::json!(max_tokens));
        }
    }
}

fn build_upstream_body(req: &LlmChatRequest, provider: &str) -> serde_json::Value {
    let model = resolve_model(req, provider);
    let mut body_obj = serde_json::map::Map::new();
    body_obj.insert("messages".to_string(), req.messages.clone());
    body_obj.insert("stream".to_string(), serde_json::json!(req.stream));
    body_obj.insert("model".to_string(), serde_json::json!(model));
    apply_token_limit(&mut body_obj, provider, req.max_tokens.unwrap_or(4096));
    if let Some(ref t) = req.tools {
        body_obj.insert("tools".to_string(), t.clone());
    }
    if let Some(ref tc) = req.tool_choice {
        body_obj.insert("tool_choice".to_string(), tc.clone());
    }
    if let Some(rf) = normalize_response_format(provider, &model, req.response_format.as_ref()) {
        body_obj.insert("response_format".to_string(), rf);
    }
    serde_json::Value::Object(body_obj)
}

pub async fn chat(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
    Json(req): Json<LlmChatRequest>,
) -> Result<Response> {
    let user_id = auth_user.0;
    ensure_active_entitlement(&state, user_id).await?;

    let llm_router = state
        .llm
        .as_ref()
        .ok_or(AppError::BadRequest("LLM not configured".into()))?;
    let provider_name = req
        .provider
        .as_deref()
        .or(Some(state.config.llm.default_provider.as_str()));
    let llm = llm_router.resolve_provider(provider_name)?;
    let body = build_upstream_body(&req, &llm.provider);
    let request_url = format!("{}/v1/chat/completions", llm.base_url);
    let request_body_str = crate::utils::truncate_for_db(
        &serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string()),
        DB_BODY_MAX,
    );

    if req.stream {
        let resp = match llm.chat_stream(&body).await {
            Ok(r) => r,
            Err(e) => {
                let err_msg = e.to_string();
                if let Err(le) = crate::service::log_service::log_external_request(
                    &state.db,
                    "llm",
                    &request_url,
                    "POST",
                    Some(&request_body_str),
                    None,
                    None,
                    Some(&err_msg),
                    Some(user_id),
                )
                .await
                {
                    warn!(error = %le, "external_request_log insert failed");
                }
                return Err(AppError::Upstream(err_msg));
            }
        };
        let status = resp.status();
        if let Err(le) = crate::service::log_service::log_external_request(
            &state.db,
            "llm",
            &request_url,
            "POST",
            Some(&request_body_str),
            Some(status.as_u16()),
            Some("[SSE stream]"),
            None,
            Some(user_id),
        )
        .await
        {
            warn!(error = %le, "external_request_log insert failed");
        }
        info!(response_status = status.as_u16(), response_stream = true, "streaming SSE to client");
        let stream = resp.bytes_stream();
        Ok(Response::builder()
            .status(status.as_u16())
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .body(Body::from_stream(stream))
            .unwrap())
    } else {
        match llm.chat(&body).await {
            Ok(bytes) => {
                let response_body_db =
                    crate::utils::truncate_for_db(&String::from_utf8_lossy(&bytes), DB_BODY_MAX);
                if let Err(le) = crate::service::log_service::log_external_request(
                    &state.db,
                    "llm",
                    &request_url,
                    "POST",
                    Some(&request_body_str),
                    Some(200),
                    Some(&response_body_db),
                    None,
                    Some(user_id),
                )
                .await
                {
                    warn!(error = %le, "external_request_log insert failed");
                }
                info!(
                    response_status = 200u16,
                    response_body = %crate::utils::bytes_preview(&bytes, 2048),
                    "response to client"
                );
                Ok(Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(Body::from(bytes))
                    .unwrap())
            }
            Err(e) => {
                let err_msg = e.to_string();
                if let Err(le) = crate::service::log_service::log_external_request(
                    &state.db,
                    "llm",
                    &request_url,
                    "POST",
                    Some(&request_body_str),
                    None,
                    None,
                    Some(&err_msg),
                    Some(user_id),
                )
                .await
                {
                    warn!(error = %le, "external_request_log insert failed");
                }
                Err(AppError::Upstream(err_msg))
            }
        }
    }
}

pub async fn list_models(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
    Query(query): Query<LlmModelsQuery>,
) -> Result<Json<LlmModelsResponse>> {
    let user_id = auth_user.0;
    ensure_active_entitlement(&state, user_id).await?;

    let llm_router = state
        .llm
        .as_ref()
        .ok_or(AppError::BadRequest("LLM not configured".into()))?;
    let requested_provider = query
        .provider
        .as_deref()
        .or(Some(state.config.llm.default_provider.as_str()));
    let provider = llm_router.resolve_provider(requested_provider)?;
    let response = provider.list_models().await?;
    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::{apply_token_limit, build_upstream_body, normalize_response_format, resolve_model};
    use admiral_types::api::LlmChatRequest;
    use serde_json::json;

    #[test]
    fn downgrades_json_schema_for_deepseek_chat() {
        let response_format = json!({
            "type": "json_schema",
            "json_schema": {
                "name": "agent_action",
                "strict": true,
                "schema": { "type": "object" }
            }
        });

        let normalized = normalize_response_format("deepseek", "deepseek-chat", Some(&response_format));

        assert_eq!(normalized, Some(json!({ "type": "json_object" })));
    }

    #[test]
    fn preserves_json_schema_for_openai() {
        let response_format = json!({
            "type": "json_schema",
            "json_schema": { "name": "agent_action", "schema": { "type": "object" } }
        });

        let normalized = normalize_response_format("openai", "gpt-4.1", Some(&response_format));

        assert_eq!(normalized, Some(response_format));
    }

    #[test]
    fn downgrades_json_schema_for_deepseek_reasoner_too() {
        let response_format = json!({
            "type": "json_schema",
            "json_schema": { "name": "agent_action", "schema": { "type": "object" } }
        });

        let normalized = normalize_response_format("deepseek", "deepseek-reasoner", Some(&response_format));

        assert_eq!(normalized, Some(json!({ "type": "json_object" })));
    }

    #[test]
    fn resolves_fast_model_to_provider_default() {
        let req = LlmChatRequest {
            messages: json!([]),
            stream: false,
            provider: Some("openai".to_string()),
            model: Some("fast".to_string()),
            max_tokens: None,
            tools: None,
            tool_choice: None,
            response_format: None,
        };

        assert_eq!(resolve_model(&req, "openai"), "gpt-4.1-mini");
    }

    #[test]
    fn build_upstream_body_uses_openai_model_verbatim() {
        let req = LlmChatRequest {
            messages: json!([{ "role": "user", "content": "hi" }]),
            stream: false,
            provider: Some("openai".to_string()),
            model: Some("gpt-4.1".to_string()),
            max_tokens: None,
            tools: None,
            tool_choice: None,
            response_format: Some(json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "agent_action",
                    "strict": true,
                    "schema": { "type": "object" }
                }
            })),
        };

        let body = build_upstream_body(&req, "openai");

        assert_eq!(body["model"], json!("gpt-4.1"));
        assert_eq!(body["max_completion_tokens"], json!(4096));
        assert!(body.get("max_tokens").is_none());
        assert_eq!(body["response_format"]["type"], json!("json_schema"));
    }

    #[test]
    fn apply_token_limit_uses_openai_field_name() {
        let mut body = serde_json::Map::new();

        apply_token_limit(&mut body, "openai", 1024);

        assert_eq!(body.get("max_completion_tokens"), Some(&json!(1024)));
        assert!(body.get("max_tokens").is_none());
    }

    #[test]
    fn apply_token_limit_keeps_legacy_field_for_deepseek() {
        let mut body = serde_json::Map::new();

        apply_token_limit(&mut body, "deepseek", 1024);

        assert_eq!(body.get("max_tokens"), Some(&json!(1024)));
        assert!(body.get("max_completion_tokens").is_none());
    }
}
