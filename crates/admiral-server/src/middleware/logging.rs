//! Request/response logging middleware (tracing).

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::info;

pub async fn logging_layer(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let start = Instant::now();
    let response = next.run(request).await;
    let status = response.status().as_u16();
    let elapsed = start.elapsed();
    info!(
        method = %method,
        path = %path,
        status = status,
        elapsed_ms = elapsed.as_millis()
    );
    response
}
