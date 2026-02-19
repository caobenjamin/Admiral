//! LLM proxy: POST /api/v1/llm/chat

use admiral_types::api::LlmChatRequest;
use axum::{
    body::Body,
    extract::State,
    response::Response,
    Json,
};

use crate::error::Result;
use crate::state::AppState;

pub async fn chat(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
    Json(req): Json<LlmChatRequest>,
) -> Result<Response> {
    let _ = auth_user;
    let llm = state
        .llm
        .as_ref()
        .ok_or(crate::error::AppError::BadRequest("LLM not configured".into()))?;
    // 联调阶段固定使用 deepseek-chat，忽略请求中的 model
    let body = serde_json::json!({
        "messages": req.messages,
        "stream": req.stream,
        "model": "deepseek-chat",
        "max_tokens": req.max_tokens.unwrap_or(4096),
    });
    if req.stream {
        let resp = llm.chat_stream(&body).await?;
        let status = resp.status();
        let bytes = resp.bytes().await.map_err(|e| crate::error::AppError::Upstream(e.to_string()))?;
        Ok(Response::builder()
            .status(status.as_u16())
            .header("Content-Type", "text/event-stream")
            .body(Body::from(bytes.to_vec()))
            .unwrap())
    } else {
        let bytes = llm.chat(&body).await?;
        Ok(Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(Body::from(bytes))
            .unwrap())
    }
}
