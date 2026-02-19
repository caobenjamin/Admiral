//! Request ID injection (X-Request-Id).

use axum::{
    extract::Request,
    http::header::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Middleware: set or propagate X-Request-Id.
pub async fn request_id_layer(request: Request, next: Next) -> Response {
    let id = request
        .headers()
        .get(&X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let mut response = next.run(request).await;
    response.headers_mut().insert(
        X_REQUEST_ID,
        HeaderValue::try_from(id.as_str()).unwrap_or(HeaderValue::from_static("")),
    );
    response
}
