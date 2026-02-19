//! MAPI proxy command: forward request to server with token from store.

use crate::commands::auth::AuthContext;
use crate::network::api_client::ApiError;
use admiral_types::api::MapiProxyRequest;

fn get_access_token(ctx: &AuthContext) -> Result<String, ApiError> {
    let cred = ctx.store.get().map_err(|e| ApiError::Credential(e.to_string()))?;
    cred.ok_or(ApiError::Unauthorized)
        .map(|c| c.access_token)
}

/// Call MAPI proxy; returns (status_code, body_bytes).
pub async fn mapi_proxy(
    ctx: &AuthContext,
    req: MapiProxyRequest,
) -> Result<(u16, Vec<u8>), ApiError> {
    let token = get_access_token(ctx)?;
    ctx.api_client.mapi_proxy(&token, &req).await
}
