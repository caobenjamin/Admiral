//! JWT auth: extractor that requires valid Bearer token and returns user_id.

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use uuid::Uuid;

/// Extractor: requires valid JWT and yields user_id. Use on handlers that need auth.
#[derive(Clone, Copy, Debug)]
pub struct AuthUser(pub Uuid);

/// AppState must be in request extensions (from Router::with_state). Contains JwtKeys for verification.
pub trait AuthState: Clone + Send + Sync {
    fn jwt_keys(&self) -> Option<&crate::infra::jwt::JwtKeys>;
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: AuthState + Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok());
        let token = match auth_header {
            Some(h) if h.starts_with("Bearer ") => h.trim_start_matches("Bearer ").trim(),
            _ => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({
                        "code": "UNAUTHORIZED",
                        "message": "missing or invalid Authorization header"
                    })),
                )
                    .into_response());
            }
        };
        let jwt = state.jwt_keys().ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "code": "CONFIG_ERROR",
                    "message": "JWT not configured"
                })),
            )
                .into_response()
        })?;
        let claims = jwt.verify(token).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "code": "INVALID_TOKEN",
                    "message": "invalid or expired token"
                })),
            )
                .into_response()
        })?;
        if claims.token_type != "access" {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "code": "INVALID_TOKEN",
                    "message": "refresh token not accepted here"
                })),
            )
                .into_response());
        }
        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "code": "INVALID_TOKEN",
                    "message": "invalid subject"
                })),
            )
                .into_response()
        })?;
        Ok(AuthUser(user_id))
    }
}
