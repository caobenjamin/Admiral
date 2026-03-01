//! OAuth API: initiate, callback, status, delete session.

use admiral_types::api::BindAccountsRequest;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::Result;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

pub async fn initiate(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
) -> Result<Json<admiral_types::api::InitiateResponse>> {
    let oauth = state
        .oauth
        .as_ref()
        .ok_or(crate::error::AppError::BadRequest("OAuth not configured".into()))?;
    let out = oauth.initiate(auth_user.0).await?;
    Ok(Json(out))
}

pub async fn callback(
    State(state): State<AppState>,
    Query(q): Query<CallbackQuery>,
) -> Result<Response> {
    let oauth = state
        .oauth
        .as_ref()
        .ok_or(crate::error::AppError::BadRequest("OAuth not configured".into()))?;
    oauth.callback(q.code, q.state).await?;
    Ok(Html(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"/><title>授权成功</title></head>
           <body><p>授权成功，请关闭此窗口。</p></body></html>"#,
    )
    .into_response())
}

pub async fn status(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
) -> Result<Json<Vec<admiral_types::api::TokenMeta>>> {
    let oauth = state
        .oauth
        .as_ref()
        .ok_or(crate::error::AppError::BadRequest("OAuth not configured".into()))?;
    let out = oauth.status(auth_user.0).await?;
    Ok(Json(out))
}

pub async fn bindings_list(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
) -> Result<Json<Vec<admiral_types::api::BoundAccountRecord>>> {
    let oauth = state
        .oauth
        .as_ref()
        .ok_or(crate::error::AppError::BadRequest("OAuth not configured".into()))?;
    let out = oauth.list_bindings(auth_user.0).await?;
    Ok(Json(out))
}

pub async fn subjects_list(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<admiral_types::api::BiliSubjectRecord>>> {
    let oauth = state
        .oauth
        .as_ref()
        .ok_or(crate::error::AppError::BadRequest("OAuth not configured".into()))?;
    let oauth_session_id = params.get("oauth_session_id").and_then(|s| Uuid::parse_str(s).ok());
    let list = oauth.list_subjects(auth_user.0, oauth_session_id).await?;
    Ok(Json(list))
}

pub async fn accounts_list(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    let oauth = state
        .oauth
        .as_ref()
        .ok_or(crate::error::AppError::BadRequest("OAuth not configured".into()))?;
    let oauth_session_id = params.get("oauth_session_id").and_then(|s| Uuid::parse_str(s).ok());
    let subject_type = params
        .get("subject_type")
        .and_then(|s| s.parse::<i32>().ok())
        .ok_or_else(|| {
            crate::error::AppError::BadRequest(
                "subject_type is required; call GET /api/v1/oauth/bilibili/subjects first".into(),
            )
        })?;
    let subject_id = params
        .get("subject_id")
        .and_then(|s| s.parse::<i32>().ok())
        .ok_or_else(|| {
            crate::error::AppError::BadRequest(
                "subject_id is required; call GET /api/v1/oauth/bilibili/subjects first".into(),
            )
        })?;
    let resp = oauth
        .list_advertisers(auth_user.0, oauth_session_id, subject_type, subject_id)
        .await?;
    if resp.code != 0 {
        return Err(crate::error::AppError::Upstream(
            resp.message.unwrap_or_else(|| "bilibili error".into()),
        ));
    }
    let (records, total) = resp
        .data
        .map(|d| (d.records.unwrap_or_default(), d.total.unwrap_or(0)))
        .unwrap_or((vec![], 0));
    Ok(Json(serde_json::json!({ "records": records, "total": total })))
}

pub async fn bind_accounts(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
    Json(req): Json<BindAccountsRequest>,
) -> Result<axum::http::StatusCode> {
    let oauth = state
        .oauth
        .as_ref()
        .ok_or(crate::error::AppError::BadRequest("OAuth not configured".into()))?;
    oauth
        .bind_accounts(
            auth_user.0,
            req.oauth_token_id,
            req.subject_type,
            req.subject_id,
            req.account_ids,
            req.account_names,
        )
        .await?;
    Ok(axum::http::StatusCode::CREATED)
}

pub async fn unbind_account(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
    axum::extract::Path(account_id): axum::extract::Path<i64>,
) -> Result<StatusCode> {
    let oauth = state
        .oauth
        .as_ref()
        .ok_or(crate::error::AppError::BadRequest("OAuth not configured".into()))?;
    let deleted = oauth.unbind_account(auth_user.0, account_id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}

pub async fn delete_session(
    State(state): State<AppState>,
    auth_user: crate::middleware::auth_guard::AuthUser,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<StatusCode> {
    let oauth = state
        .oauth
        .as_ref()
        .ok_or(crate::error::AppError::BadRequest("OAuth not configured".into()))?;
    let deleted = oauth.delete_session(auth_user.0, id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}
