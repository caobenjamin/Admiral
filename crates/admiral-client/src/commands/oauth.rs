//! OAuth commands: initiate, status, accounts_list, bind_accounts, delete_session.
//! All require token from CredentialStore (via AuthContext).

use crate::commands::auth::AuthContext;
use crate::network::api_client::ApiError;
use admiral_types::api::{BindAccountsRequest, InitiateResponse, OAuthAccountsResponse, TokenMeta};

fn get_access_token(ctx: &AuthContext) -> Result<String, ApiError> {
    let cred = ctx.store.get().map_err(|e| ApiError::Credential(e.to_string()))?;
    cred.ok_or(ApiError::Unauthorized)
        .map(|c| c.access_token)
}

/// Get B站 authorization URL; frontend opens it in browser.
pub async fn oauth_initiate(ctx: &AuthContext) -> Result<InitiateResponse, ApiError> {
    let token = get_access_token(ctx)?;
    ctx.api_client.oauth_initiate(&token).await
}

pub async fn oauth_status(ctx: &AuthContext) -> Result<Vec<TokenMeta>, ApiError> {
    let token = get_access_token(ctx)?;
    ctx.api_client.oauth_status(&token).await
}

pub async fn oauth_accounts_list(
    ctx: &AuthContext,
    oauth_session_id: Option<uuid::Uuid>,
) -> Result<OAuthAccountsResponse, ApiError> {
    let token = get_access_token(ctx)?;
    ctx.api_client
        .oauth_accounts_list(&token, oauth_session_id)
        .await
}

pub async fn oauth_bind_accounts(
    ctx: &AuthContext,
    req: BindAccountsRequest,
) -> Result<(), ApiError> {
    let token = get_access_token(ctx)?;
    ctx.api_client.oauth_bind_accounts(&token, &req).await
}

pub async fn oauth_delete_session(
    ctx: &AuthContext,
    session_id: uuid::Uuid,
) -> Result<(), ApiError> {
    let token = get_access_token(ctx)?;
    ctx.api_client.oauth_delete_session(&token, session_id).await
}
