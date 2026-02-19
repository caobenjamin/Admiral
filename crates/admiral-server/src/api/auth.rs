//! Auth API: register, login, refresh, me, license.

use admiral_types::api::{
    LoginRequest, RefreshRequest, RegisterRequest,
    LicenseResponse, RefreshResponse, TokenResponse, UserInfo,
};
use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;

use crate::error::Result;
use crate::service::auth_service::AuthService;
use crate::state::AppState;

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<TokenResponse>)> {
    let jwt = state.jwt.as_ref().ok_or(crate::error::AppError::Internal(anyhow::anyhow!("JWT not configured")))?;
    let auth = AuthService::new(state.db.clone(), Arc::clone(jwt), &state.config);
    let out = auth
        .register(
            req.email,
            req.password,
            req.name,
            req.company,
            req.invite_code,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(out)))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<TokenResponse>> {
    let jwt = state.jwt.as_ref().ok_or(crate::error::AppError::Internal(anyhow::anyhow!("JWT not configured")))?;
    let auth = AuthService::new(state.db.clone(), Arc::clone(jwt), &state.config);
    let out = auth.login(req.email, req.password).await?;
    Ok(Json(out))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>> {
    let jwt = state.jwt.as_ref().ok_or(crate::error::AppError::Internal(anyhow::anyhow!("JWT not configured")))?;
    let auth = AuthService::new(state.db.clone(), Arc::clone(jwt), &state.config);
    let out = auth.refresh(req.refresh_token).await?;
    Ok(Json(out))
}

pub async fn me(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
) -> Result<Json<UserInfo>> {
    let jwt = state.jwt.as_ref().ok_or(crate::error::AppError::Internal(anyhow::anyhow!("JWT not configured")))?;
    let auth = AuthService::new(state.db.clone(), Arc::clone(jwt), &state.config);
    let out = auth.me(auth_user.0).await?;
    Ok(Json(out))
}

pub async fn license(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
) -> Result<Json<LicenseResponse>> {
    let jwt = state.jwt.as_ref().ok_or(crate::error::AppError::Internal(anyhow::anyhow!("JWT not configured")))?;
    let auth = AuthService::new(state.db.clone(), Arc::clone(jwt), &state.config);
    let out = auth.license(auth_user.0).await?;
    Ok(Json(out))
}
