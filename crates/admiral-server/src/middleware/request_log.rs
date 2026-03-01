//! 客户端请求落表中间件：请求内容、IP、用户 ID、返回内容（含报错）写入 api_request_log。

use axum::{
    body::to_bytes,
    extract::State,
    http::{header::AUTHORIZATION, Request},
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::warn;
use uuid::Uuid;

use crate::state::AppState;

/// 由 logging 中间件写入的请求体，供落表使用（最大 64k 字符）。
#[derive(Clone)]
pub struct RequestBodyForLog(pub Option<String>);

const RESPONSE_BODY_LOG_LIMIT: usize = 64 * 1024; // 64k
const DB_BODY_MAX_LEN: usize = 65535;

fn truncate_for_db(s: &str, max: usize) -> &str {
    crate::utils::truncate_utf8_at(s, max)
}

fn client_ip_from_headers(request: &Request<axum::body::Body>) -> Option<String> {
    request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
}

fn optional_user_id_from_jwt(state: &AppState, request: &Request<axum::body::Body>) -> Option<Uuid> {
    let jwt = state.jwt.as_ref()?;
    let auth = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())?;
    let token = auth.strip_prefix("Bearer ")?.trim();
    let claims = jwt.verify(token).ok()?;
    if claims.token_type != "access" {
        return None;
    }
    Uuid::parse_str(&claims.sub).ok()
}

pub async fn request_log_layer(
    State(state): State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let request_body: Option<String> = request
        .extensions()
        .get::<RequestBodyForLog>()
        .and_then(|r| r.0.clone())
        .map(|s| truncate_for_db(&s, DB_BODY_MAX_LEN).to_string());
    let client_ip = client_ip_from_headers(&request);
    let user_id = optional_user_id_from_jwt(&state, &request);

    let start = Instant::now();
    let response = next.run(request).await;
    let elapsed_ms = start.elapsed().as_millis() as u64;
    let response_status = response.status().as_u16();

    let (parts, body) = response.into_parts();
    let (response_body_for_db, body_for_response) = match to_bytes(body, RESPONSE_BODY_LOG_LIMIT).await {
        Ok(bytes) => {
            let s = String::from_utf8_lossy(&bytes);
            let for_db = Some(truncate_for_db(&s, DB_BODY_MAX_LEN).to_string());
            (for_db, axum::body::Body::from(bytes))
        }
        Err(e) => {
            warn!(error = %e, "response body read failed for request log");
            (None, axum::body::Body::empty())
        }
    };

    let db = state.db.clone();
    if let Err(e) = crate::service::log_service::log_api_request(
        &db,
        &method,
        &path,
        request_body.as_deref(),
        user_id,
        client_ip.as_deref(),
        Some(response_status),
        response_body_for_db.as_deref(),
        None,
        Some(elapsed_ms),
    )
    .await
    {
        warn!(error = %e, "api_request_log insert failed");
    }

    Response::from_parts(parts, body_for_response)
}
