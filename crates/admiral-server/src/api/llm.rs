//! LLM proxy: POST /api/v1/llm/chat
//! 请求与响应（含报错）落表 external_request_log。

use admiral_types::api::LlmChatRequest;
use axum::{
    body::Body,
    extract::State,
    response::Response,
    Json,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use tracing::{info, warn};

use crate::error::Result;
use crate::entities::entitlement;
use crate::state::AppState;

const DB_BODY_MAX: usize = 65535;

fn normalize_response_format_for_model(model: &str, response_format: Option<&serde_json::Value>) -> Option<serde_json::Value> {
    let response_format = response_format?.clone();

    if model == "deepseek-chat"
        && response_format
            .get("type")
            .and_then(|v| v.as_str())
            .is_some_and(|t| t == "json_schema")
    {
        return Some(serde_json::json!({ "type": "json_object" }));
    }

    Some(response_format)
}

fn build_upstream_body(req: &LlmChatRequest) -> serde_json::Value {
    // 联调阶段固定使用 deepseek-chat，忽略请求中的 model
    let model = "deepseek-chat";
    let mut body_obj = serde_json::map::Map::new();
    body_obj.insert("messages".to_string(), req.messages.clone());
    body_obj.insert("stream".to_string(), serde_json::json!(req.stream));
    body_obj.insert("model".to_string(), serde_json::json!(model));
    body_obj.insert(
        "max_tokens".to_string(),
        serde_json::json!(req.max_tokens.unwrap_or(4096)),
    );
    if let Some(ref t) = req.tools {
        body_obj.insert("tools".to_string(), t.clone());
    }
    if let Some(ref tc) = req.tool_choice {
        body_obj.insert("tool_choice".to_string(), tc.clone());
    }
    if let Some(rf) = normalize_response_format_for_model(model, req.response_format.as_ref()) {
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
    let entitlement = entitlement::Entity::find()
        .filter(entitlement::Column::UserId.eq(user_id))
        .filter(entitlement::Column::IsActive.eq(true))
        .order_by_desc(entitlement::Column::ExpiresAt)
        .one(&state.db)
        .await?;
    match entitlement {
        Some(ent) if ent.expires_at.timestamp_millis() > chrono::Utc::now().timestamp_millis() => {}
        Some(ent) => {
            return Err(crate::error::AppError::LicenseInactive {
                expires_at: Some(ent.expires_at.to_rfc3339()),
            });
        }
        None => {
            return Err(crate::error::AppError::LicenseInactive { expires_at: None });
        }
    }

    let llm = state
        .llm
        .as_ref()
        .ok_or(crate::error::AppError::BadRequest("LLM not configured".into()))?;
    let body = build_upstream_body(&req);
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
                return Err(crate::error::AppError::Upstream(err_msg));
            }
        };
        let status = resp.status();
        // Log request only; response body is streamed and not buffered
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
                let response_body_db = crate::utils::truncate_for_db(&String::from_utf8_lossy(&bytes), DB_BODY_MAX);
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
                Err(crate::error::AppError::Upstream(err_msg))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{build_upstream_body, normalize_response_format_for_model};
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

        let normalized = normalize_response_format_for_model("deepseek-chat", Some(&response_format));

        assert_eq!(normalized, Some(json!({ "type": "json_object" })));
    }

    #[test]
    fn preserves_non_schema_response_format_for_deepseek_chat() {
        let response_format = json!({ "type": "json_object" });

        let normalized = normalize_response_format_for_model("deepseek-chat", Some(&response_format));

        assert_eq!(normalized, Some(response_format));
    }

    #[test]
    fn build_upstream_body_uses_fixed_model_and_normalized_response_format() {
        let req = LlmChatRequest {
            messages: json!([{ "role": "user", "content": "hi" }]),
            stream: false,
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

        let body = build_upstream_body(&req);

        assert_eq!(body["model"], json!("deepseek-chat"));
        assert_eq!(body["max_tokens"], json!(4096));
        assert_eq!(body["response_format"], json!({ "type": "json_object" }));
    }
}
