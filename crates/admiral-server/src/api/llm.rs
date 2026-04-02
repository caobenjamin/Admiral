//! LLM proxy: POST /api/v1/llm/responses, GET /api/v1/llm/models
//! 请求与响应（含报错）落表 external_request_log。

use admiral_types::api::{LlmResponsesRequest, LlmModelsResponse};
use axum::{
    body::Body,
    extract::{Query, State},
    response::Response,
    Json,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use tracing::{info, warn};

use crate::entities::entitlement;
use crate::error::{AppError, Result};
use crate::service::llm_router::{
    default_model_for_provider, generation_path_for_provider, protocol_for_provider,
    TokenLimitField,
};
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

fn normalize_openai_text_format(response_format: &Value) -> Value {
    let Some(obj) = response_format.as_object() else {
        return response_format.clone();
    };

    if obj.get("type").and_then(|v| v.as_str()) != Some("json_schema") {
        return response_format.clone();
    }

    let Some(json_schema) = obj.get("json_schema").and_then(|v| v.as_object()) else {
        return response_format.clone();
    };

    let mut normalized = Map::new();
    normalized.insert("type".to_string(), json!("json_schema"));

    for field in ["name", "description", "strict", "schema"] {
        if let Some(value) = json_schema.get(field) {
            normalized.insert(field.to_string(), value.clone());
        }
    }

    Value::Object(normalized)
}

fn resolve_model(req: &LlmResponsesRequest, provider: &str) -> String {
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

fn normalize_openai_input(messages: &Value) -> Value {
    let Some(items) = messages.as_array() else {
        return messages.clone();
    };

    Value::Array(
        items
            .iter()
            .filter_map(|item| {
                let obj = item.as_object()?;
                let role = obj.get("role").and_then(|v| v.as_str()).unwrap_or("user");
                let normalized_role = match role {
                    "system" | "user" | "assistant" => role,
                    "tool" => "assistant",
                    _ => "user",
                };

                let mut content = obj
                    .get("content")
                    .cloned()
                    .unwrap_or(Value::String(String::new()));

                if role == "tool" {
                    let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let text = match &content {
                        Value::String(s) => s.clone(),
                        Value::Null => "{}".to_string(),
                        other => serde_json::to_string(other).unwrap_or_else(|_| "{}".to_string()),
                    };
                    content = Value::String(format!("Tool {} returned:\n{}", name, text));
                } else if let Some(tool_calls) = obj.get("tool_calls").and_then(|v| v.as_array()) {
                    let summary = tool_calls
                        .iter()
                        .filter_map(|call| {
                            let function = call.get("function")?;
                            let name = function.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                            let arguments = function.get("arguments").cloned().unwrap_or(Value::String("{}".to_string()));
                            let arguments = match arguments {
                                Value::String(s) => s,
                                other => serde_json::to_string(&other).unwrap_or_else(|_| "{}".to_string()),
                            };
                            Some(format!("Called tool {} with arguments {}", name, arguments))
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    if !summary.is_empty() {
                        content = match content {
                            Value::String(existing) if !existing.trim().is_empty() => {
                                Value::String(format!("{}\n{}", existing, summary))
                            }
                            _ => Value::String(summary),
                        };
                    }
                }

                Some(json!({
                    "role": normalized_role,
                    "content": content,
                }))
            })
            .collect(),
    )
}

fn normalize_openai_tools(tools: &Value) -> Value {
    let Some(items) = tools.as_array() else {
        return tools.clone();
    };

    Value::Array(
        items
            .iter()
            .map(|item| {
                let Some(obj) = item.as_object() else {
                    return item.clone();
                };
                if obj.get("type").and_then(|v| v.as_str()) != Some("function") {
                    return item.clone();
                }
                if let Some(function) = obj.get("function").and_then(|v| v.as_object()) {
                    let mut normalized = Map::new();
                    normalized.insert("type".to_string(), json!("function"));
                    if let Some(name) = function.get("name") {
                        normalized.insert("name".to_string(), name.clone());
                    }
                    if let Some(description) = function.get("description") {
                        normalized.insert("description".to_string(), description.clone());
                    }
                    if let Some(parameters) = function.get("parameters") {
                        normalized.insert("parameters".to_string(), parameters.clone());
                    }
                    normalized.insert("strict".to_string(), json!(true));
                    return Value::Object(normalized);
                }
                item.clone()
            })
            .collect(),
    )
}

fn normalize_openai_tool_choice(tool_choice: &Value) -> Value {
    if let Some(mode) = tool_choice.as_str() {
        return json!(mode);
    }

    let Some(obj) = tool_choice.as_object() else {
        return tool_choice.clone();
    };

    if obj.get("type").and_then(|v| v.as_str()) == Some("function") {
        if let Some(function) = obj.get("function").and_then(|v| v.as_object()) {
            if let Some(name) = function.get("name").and_then(|v| v.as_str()) {
                return json!({
                    "type": "function",
                    "name": name,
                });
            }
        }
    }

    tool_choice.clone()
}

fn build_openai_responses_body(req: &LlmResponsesRequest, model: &str) -> Value {
    let mut body_obj = Map::new();
    body_obj.insert("model".to_string(), json!(model));
    body_obj.insert("input".to_string(), normalize_openai_input(&req.messages));
    body_obj.insert("stream".to_string(), json!(req.stream));
    body_obj.insert(
        "max_output_tokens".to_string(),
        json!(req.max_tokens.unwrap_or(4096)),
    );
    if let Some(ref t) = req.tools {
        body_obj.insert("tools".to_string(), normalize_openai_tools(t));
    }
    if let Some(ref tc) = req.tool_choice {
        body_obj.insert("tool_choice".to_string(), normalize_openai_tool_choice(tc));
    }
    if let Some(format) = normalize_response_format("openai", model, req.response_format.as_ref()) {
        body_obj.insert(
            "text".to_string(),
            json!({
                "format": normalize_openai_text_format(&format),
            }),
        );
    }
    Value::Object(body_obj)
}

fn build_upstream_body(req: &LlmResponsesRequest, provider: &str) -> serde_json::Value {
    let model = resolve_model(req, provider);
    if provider == "openai" {
        return build_openai_responses_body(req, &model);
    }
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

fn extract_legacy_chat_completions_text(response: &Value) -> Option<String> {
    let message = response.pointer("/choices/0/message/content")?;
    match message {
        Value::String(text) => {
            if text.trim().is_empty() { None } else { Some(text.to_string()) }
        }
        Value::Array(items) => {
            let text = items
                .iter()
                .filter_map(|item| match item {
                    Value::Object(map) => map.get("text").and_then(|v| v.as_str()),
                    _ => None,
                })
                .collect::<String>();
            if text.trim().is_empty() { None } else { Some(text) }
        }
        _ => None,
    }
}

fn normalize_upstream_response(provider: &str, bytes: &[u8]) -> Result<Vec<u8>> {
    if provider == "openai" {
        return Ok(bytes.to_vec());
    }

    let response: Value = serde_json::from_slice(bytes)
        .map_err(|e| AppError::Upstream(format!("parse upstream response: {}", e)))?;
    let text = extract_legacy_chat_completions_text(&response)
        .ok_or_else(|| AppError::Upstream("missing text in upstream response".to_string()))?;

    serde_json::to_vec(&json!({
        "output_text": text.clone(),
        "output": [{
            "type": "message",
            "content": [{
                "type": "output_text",
                "text": text
            }]
        }]
    }))
    .map_err(|e| AppError::Upstream(format!("serialize normalized response: {}", e)))
}

pub async fn responses(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
    Json(req): Json<LlmResponsesRequest>,
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
    let request_url = format!("{}/{}", llm.base_url, generation_path_for_provider(&llm.provider));
    let request_body_str = crate::utils::truncate_for_db(
        &serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string()),
        DB_BODY_MAX,
    );

    if req.stream {
        let resp = match llm.create_response_stream(&body).await {
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
        match llm.create_response(&body).await {
            Ok(bytes) => {
                let normalized_bytes = normalize_upstream_response(&llm.provider, &bytes)?;
                let response_body_db =
                    crate::utils::truncate_for_db(&String::from_utf8_lossy(&normalized_bytes), DB_BODY_MAX);
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
                    response_body = %crate::utils::bytes_preview(&normalized_bytes, 2048),
                    "response to client"
                );
                Ok(Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(Body::from(normalized_bytes))
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
    use super::{
        apply_token_limit, build_upstream_body, normalize_openai_input,
        normalize_openai_text_format, normalize_response_format, resolve_model,
    };
    use admiral_types::api::LlmResponsesRequest;
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
    fn normalize_openai_text_format_flattens_legacy_json_schema_shape() {
        let response_format = json!({
            "type": "json_schema",
            "json_schema": {
                "name": "agent_action",
                "description": "Structured agent action",
                "strict": true,
                "schema": { "type": "object" }
            }
        });

        let normalized = normalize_openai_text_format(&response_format);

        assert_eq!(normalized, json!({
            "type": "json_schema",
            "name": "agent_action",
            "description": "Structured agent action",
            "strict": true,
            "schema": { "type": "object" }
        }));
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
        let req = LlmResponsesRequest {
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
        let req = LlmResponsesRequest {
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
        assert_eq!(body["max_output_tokens"], json!(4096));
        assert!(body.get("input").is_some());
        assert_eq!(body["text"]["format"]["type"], json!("json_schema"));
        assert_eq!(body["text"]["format"]["name"], json!("agent_action"));
        assert_eq!(body["text"]["format"]["strict"], json!(true));
        assert_eq!(body["text"]["format"]["schema"], json!({ "type": "object" }));
        assert!(body["text"]["format"].get("json_schema").is_none());
    }

    #[test]
    fn build_upstream_body_normalizes_openai_tools_and_tool_choice() {
        let req = LlmResponsesRequest {
            messages: json!([{ "role": "user", "content": "hi" }]),
            stream: false,
            provider: Some("openai".to_string()),
            model: Some("gpt-4.1".to_string()),
            max_tokens: Some(2048),
            tools: Some(json!([{
                "type": "function",
                "function": {
                    "name": "lookup",
                    "description": "Find a record",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" }
                        },
                        "required": ["id"]
                    }
                }
            }])),
            tool_choice: Some(json!({
                "type": "function",
                "function": { "name": "lookup" }
            })),
            response_format: None,
        };

        let body = build_upstream_body(&req, "openai");

        assert_eq!(body["tools"], json!([{
            "type": "function",
            "name": "lookup",
            "description": "Find a record",
            "parameters": {
                "type": "object",
                "properties": {
                    "id": { "type": "string" }
                },
                "required": ["id"]
            },
            "strict": true
        }]));
        assert_eq!(body["tool_choice"], json!({
            "type": "function",
            "name": "lookup"
        }));
    }

    #[test]
    fn normalize_openai_input_flattens_tool_messages() {
        let input = json!([
            {
                "role": "assistant",
                "tool_calls": [{
                    "type": "function",
                    "function": {
                        "name": "lookup",
                        "arguments": "{\"id\":1}"
                    }
                }]
            },
            {
                "role": "tool",
                "name": "lookup",
                "content": "{\"ok\":true}"
            }
        ]);

        let normalized = normalize_openai_input(&input);

        assert_eq!(normalized[0]["role"], json!("assistant"));
        assert!(normalized[0]["content"].as_str().unwrap().contains("Called tool lookup"));
        assert_eq!(normalized[1]["role"], json!("assistant"));
        assert!(normalized[1]["content"].as_str().unwrap().contains("Tool lookup returned"));
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
