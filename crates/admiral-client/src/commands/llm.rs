//! LLM chat command: send messages to server, get response (non-stream or stream later).

use crate::commands::auth::AuthContext;
use crate::network::api_client::ApiError;
use admiral_types::api::LlmChatRequest;

/// Chat with LLM (non-stream). Returns response body bytes (JSON).
pub async fn llm_chat(ctx: &AuthContext, req: LlmChatRequest) -> Result<Vec<u8>, ApiError> {
    let token = ctx.get_access_token()?;
    ctx.api_client.llm_chat(&token, &req).await
}
