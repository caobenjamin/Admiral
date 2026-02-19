//! Unified error type and HTTP response mapping.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

pub type Result<T, E = AppError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),

    #[error("config error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("database error: {0}")]
    Db(#[from] sea_orm::DbErr),

    #[error("invalid token")]
    InvalidToken,

    #[error("invite code invalid or already used")]
    InviteInvalid,

    #[error("email already registered")]
    EmailExists,

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("upstream error: {0}")]
    Upstream(String),
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "FORBIDDEN"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
            AppError::InviteInvalid | AppError::EmailExists | AppError::InvalidCredentials => {
                (StatusCode::BAD_REQUEST, "BAD_REQUEST")
            }
            AppError::InvalidToken => (StatusCode::UNAUTHORIZED, "INVALID_TOKEN"),
            AppError::Upstream(_) => (StatusCode::BAD_GATEWAY, "UPSTREAM_ERROR"),
            AppError::Internal(_) | AppError::Config(_) | AppError::Db(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR")
            }
        };
        let body = ErrorBody {
            code: code.to_string(),
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}
