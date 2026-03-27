//! MAPI proxy command: forward request to server with token from store.

use crate::commands::auth::AuthContext;
use crate::network::api_client::ApiError;
use admiral_types::api::MapiProxyRequest;

/// Call MAPI proxy; returns (status_code, body_bytes).
pub async fn mapi_proxy(
    ctx: &AuthContext,
    req: MapiProxyRequest,
) -> Result<(u16, Vec<u8>), ApiError> {
    let token = ctx.get_access_token()?;
    ctx.api_client.mapi_proxy(&token, &req).await
}
