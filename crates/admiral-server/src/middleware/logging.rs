//! Request/response logging middleware (tracing).
//!
//! 记录：客户端请求详情（方法+地址+请求体）、响应状态与耗时。

use axum::{
    body::{to_bytes, Body},
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{info, warn};

/// 请求体日志最大长度（字节）；超过则 to_bytes 返回 Err，请求体无法透传，handler 可能收到空 body
const REQUEST_BODY_LOG_LIMIT: usize = 1024 * 1024; // 1MB
/// 日志中请求体展示的最大字符数；超过则截断
const REQUEST_BODY_DISPLAY_LIMIT: usize = 4096;

pub async fn logging_layer(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let uri_string = uri.to_string();

    let (parts, body) = request.into_parts();
    let (request_body_log, body_for_next) = match to_bytes(body, REQUEST_BODY_LOG_LIMIT).await {
        Ok(bytes) => {
            let display = if bytes.is_empty() {
                "(empty)".to_string()
            } else {
                match String::from_utf8(bytes.to_vec()) {
                    Ok(s) => {
                        if s.len() > REQUEST_BODY_DISPLAY_LIMIT {
                            format!("{}... (truncated, total {} chars)", &s[..REQUEST_BODY_DISPLAY_LIMIT], s.len())
                        } else {
                            s
                        }
                    }
                    Err(_) => format!("<binary, {} bytes>", bytes.len()),
                }
            };
            (display, Body::from(bytes))
        }
        Err(e) => {
            warn!(error = %e, "request body read failed or exceeded limit");
            (String::from("(omitted: too large or error)"), Body::empty())
        }
    };
    let request = Request::from_parts(parts, body_for_next);

    let start = Instant::now();
    let response = next.run(request).await;
    let status = response.status().as_u16();
    let elapsed = start.elapsed();

    info!(
        method = %method,
        uri = %uri_string,
        request_body = %request_body_log,
        response_status = status,
        elapsed_ms = elapsed.as_millis(),
        "client request"
    );
    response
}
