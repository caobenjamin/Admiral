//! MAPI proxy endpoint: POST /api/v1/mapi/proxy

use admiral_types::api::MapiProxyRequest;
use axum::{extract::State, Json};
use tracing::info;

use crate::error::Result;
use crate::service::mapi_proxy;
use crate::state::AppState;

pub async fn proxy(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
    Json(req): Json<MapiProxyRequest>,
) -> Result<axum::response::Response> {
    let oauth = state
        .oauth
        .as_ref()
        .ok_or(crate::error::AppError::BadRequest("OAuth not configured".into()))?;
    let (status, body) = mapi_proxy::proxy(
        &state.db,
        oauth,
        &state.http,
        auth_user.0,
        req.account_id,
        &req.method,
        &req.path,
        req.query.as_ref(),
        req.body.as_ref(),
    )
    .await?;
    let status = axum::http::StatusCode::from_u16(status)
        .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    info!(
        response_status = status.as_u16(),
        response_body = %crate::utils::bytes_preview(&body, 2048),
        "response to client"
    );
    Ok(axum::response::Response::builder()
        .status(status)
        .body(axum::body::Body::from(body))
        .unwrap())
}
