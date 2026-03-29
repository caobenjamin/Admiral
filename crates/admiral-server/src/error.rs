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
    #[error("account limit exceeded: max={0}")]
    AccountLimitExceeded(u32),

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

    #[error("license inactive or expired")]
    LicenseInactive { expires_at: Option<String> },
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_accounts: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<String>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, max_accounts) = match &self {
            AppError::AccountLimitExceeded(max) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorBody {
                        code: "ACCOUNT_LIMIT_EXCEEDED".to_string(),
                        message: format!("已达账户数量上限（{}），请删除部分账户后再添加。", max),
                        max_accounts: Some(*max),
                        expires_at: None,
                    }),
                )
                    .into_response()
            }
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND", None),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", None),
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", None),
            AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "FORBIDDEN", None),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT", None),
            AppError::InviteInvalid | AppError::EmailExists | AppError::InvalidCredentials => {
                (StatusCode::BAD_REQUEST, "BAD_REQUEST", None)
            }
            AppError::InvalidToken => (StatusCode::UNAUTHORIZED, "INVALID_TOKEN", None),
            AppError::Upstream(_) => (StatusCode::BAD_GATEWAY, "UPSTREAM_ERROR", None),
            AppError::LicenseInactive { .. } => (StatusCode::FORBIDDEN, "LICENSE_INACTIVE", None),
            AppError::Internal(_) | AppError::Config(_) | AppError::Db(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", None)
            }
        };
        let body = ErrorBody {
            code: code.to_string(),
            message: self.to_string(),
            max_accounts,
            expires_at: match &self {
                AppError::LicenseInactive { expires_at } => expires_at.clone(),
                _ => None,
            },
        };
        (status, Json(body)).into_response()
    }
}
