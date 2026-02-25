//! Auth commands: register, login, refresh, logout, me, license.
//! Token is read/written only via CredentialStore (Rust side).

use crate::credential::{CredentialError, CredentialStore, StoredCredential};
use crate::network::api_client::{ApiClient, ApiError};
use admiral_types::api::*;
use std::sync::Arc;

fn store_err(e: CredentialError) -> ApiError {
    ApiError::Credential(e.to_string())
}

/// Context for auth commands: API client and credential store.
pub struct AuthContext {
    pub api_client: Arc<ApiClient>,
    pub store: Arc<dyn CredentialStore>,
}

impl AuthContext {
    pub fn new(api_client: Arc<ApiClient>, store: Arc<dyn CredentialStore>) -> Self {
        Self { api_client, store }
    }

    /// Get current access token for API calls. Returns error if not logged in.
    pub fn get_access_token(&self) -> Result<String, ApiError> {
        let cred = self.store.get().map_err(|e| ApiError::Credential(e.to_string()))?;
        cred.ok_or(ApiError::Unauthorized).map(|c| c.access_token)
    }
}

/// Register: call API, on success save credentials to store.
pub async fn register(
    ctx: &AuthContext,
    req: RegisterRequest,
) -> Result<TokenResponse, ApiError> {
    let resp = ctx.api_client.register(&req).await?;
    let cred = StoredCredential::from_token_response(&resp);
    ctx.store.set(&cred).map_err(store_err)?;
    Ok(resp)
}

/// Login: call API, on success save credentials to store.
pub async fn login(ctx: &AuthContext, req: LoginRequest) -> Result<TokenResponse, ApiError> {
    let resp = ctx.api_client.login(&req).await?;
    let cred = StoredCredential::from_token_response(&resp);
    ctx.store.set(&cred).map_err(store_err)?;
    Ok(resp)
}

/// Refresh: use refresh_token from store, get new access token, update store.
pub async fn refresh(ctx: &AuthContext) -> Result<RefreshResponse, ApiError> {
    let cred = ctx.store.get().map_err(store_err)?.ok_or(ApiError::Unauthorized)?;
    let resp = ctx.api_client.refresh(&cred.refresh_token).await?;
    let new_cred = StoredCredential {
        access_token: resp.access_token.clone(),
        refresh_token: cred.refresh_token,
        user_id: cred.user_id,
        user_email: cred.user_email,
        user_name: cred.user_name,
        user_role: cred.user_role,
    };
    ctx.store.set(&new_cred).map_err(store_err)?;
    Ok(resp)
}

/// Logout: clear credentials from store.
pub fn logout(ctx: &AuthContext) -> Result<(), CredentialError> {
    ctx.store.clear()
}


/// Me: get current user (requires token; on 401 try refresh and retry once).
pub async fn me(ctx: &AuthContext) -> Result<UserInfo, ApiError> {
    let cred = ctx.store.get().map_err(store_err)?.ok_or(ApiError::Unauthorized)?;
    match ctx.api_client.me(&cred.access_token).await {
        Ok(u) => Ok(u),
        Err(ApiError::Unauthorized) => {
            let refresh_resp = ctx.api_client.refresh(&cred.refresh_token).await?;
            let new_cred = StoredCredential {
                access_token: refresh_resp.access_token.clone(),
                refresh_token: cred.refresh_token,
                user_id: cred.user_id.clone(),
                user_email: cred.user_email.clone(),
                user_name: cred.user_name.clone(),
                user_role: cred.user_role.clone(),
            };
            ctx.store.set(&new_cred).map_err(store_err)?;
            ctx.api_client.me(&refresh_resp.access_token).await
        }
        Err(e) => Err(e),
    }
}

/// License: get license status (requires token; on 401 try refresh and retry once).
pub async fn license(ctx: &AuthContext) -> Result<LicenseResponse, ApiError> {
    let cred = ctx.store.get().map_err(store_err)?.ok_or(ApiError::Unauthorized)?;
    match ctx.api_client.license(&cred.access_token).await {
        Ok(l) => Ok(l),
        Err(ApiError::Unauthorized) => {
            let refresh_resp = ctx.api_client.refresh(&cred.refresh_token).await?;
            let new_cred = StoredCredential {
                access_token: refresh_resp.access_token.clone(),
                refresh_token: cred.refresh_token,
                user_id: cred.user_id.clone(),
                user_email: cred.user_email.clone(),
                user_name: cred.user_name.clone(),
                user_role: cred.user_role.clone(),
            };
            ctx.store.set(&new_cred).map_err(store_err)?;
            ctx.api_client.license(&refresh_resp.access_token).await
        }
        Err(e) => Err(e),
    }
}
