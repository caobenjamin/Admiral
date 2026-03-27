//! LLM proxy: POST /api/v1/llm/chat
//! 请求与响应（含报错）落表 external_request_log。

use admiral_types::api::LlmChatRequest;
use axum::{
    body::Body,
    extract::State,
    response::Response,
    Json,
};
use tracing::{info, warn};

use crate::error::Result;
use crate::state::AppState;

const DB_BODY_MAX: usize = 65535;

pub async fn chat(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
    Json(req): Json<LlmChatRequest>,
) -> Result<Response> {
    let user_id = auth_user.0;
    let llm = state
        .llm
        .as_ref()
        .ok_or(crate::error::AppError::BadRequest("LLM not configured".into()))?;
    // 联调阶段固定使用 deepseek-chat，忽略请求中的 model
    let mut body_obj = serde_json::map::Map::new();
    body_obj.insert("messages".to_string(), req.messages.clone());
    body_obj.insert("stream".to_string(), serde_json::json!(req.stream));
    body_obj.insert("model".to_string(), serde_json::json!("deepseek-chat"));
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
    let body = serde_json::Value::Object(body_obj);
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
        let bytes = match resp.bytes().await {
            Ok(b) => b,
            Err(e) => {
                let err_msg = e.to_string();
                if let Err(le) = crate::service::log_service::log_external_request(
                    &state.db,
                    "llm",
                    &request_url,
                    "POST",
                    Some(&request_body_str),
                    Some(status.as_u16()),
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
        let response_body_db = crate::utils::truncate_for_db(&String::from_utf8_lossy(&bytes), DB_BODY_MAX);
        if let Err(le) = crate::service::log_service::log_external_request(
            &state.db,
            "llm",
            &request_url,
            "POST",
            Some(&request_body_str),
            Some(status.as_u16()),
            Some(&response_body_db),
            None,
            Some(user_id),
        )
        .await
        {
            warn!(error = %le, "external_request_log insert failed");
        }
        info!(
            response_status = status.as_u16(),
            response_stream = true,
            response_body_preview = %crate::utils::bytes_preview(bytes.as_ref(), 512),
            "response to client"
        );
        Ok(Response::builder()
            .status(status.as_u16())
            .header("Content-Type", "text/event-stream")
            .body(Body::from(bytes.to_vec()))
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
