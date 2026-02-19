//! LLM chat command: send messages to server, get response (non-stream or stream later).

use crate::commands::auth::AuthContext;
use crate::network::api_client::ApiError;
use admiral_types::api::LlmChatRequest;

fn get_access_token(ctx: &AuthContext) -> Result<String, ApiError> {
    let cred = ctx.store.get().map_err(|e| ApiError::Credential(e.to_string()))?;
    cred.ok_or(ApiError::Unauthorized)
        .map(|c| c.access_token)
}

/// Chat with LLM (non-stream). Returns response body bytes (JSON).
pub async fn llm_chat(ctx: &AuthContext, req: LlmChatRequest) -> Result<Vec<u8>, ApiError> {
    let token = get_access_token(ctx)?;
    ctx.api_client.llm_chat(&token, &req).await
}
