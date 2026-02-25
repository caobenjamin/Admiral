//! MAPI proxy: resolve token by (user_id, account_id), forward request to B站.
//! 请求与响应（含报错）落表 external_request_log。

use crate::error::{AppError, Result};
use crate::service::oauth_service::OauthService;
use reqwest::Client;
use sea_orm::DatabaseConnection;
use tracing::{info, warn};
use uuid::Uuid;

const DB_BODY_MAX: usize = 65535;

const MAPI_BASE: &str = "https://cm.bilibili.com/takumi/api";
const BODY_LOG_LIMIT: usize = 2048;

fn truncate_for_log(bytes: &[u8], limit: usize) -> String {
    if bytes.is_empty() {
        return "(empty)".to_string();
    }
    match String::from_utf8(bytes.to_vec()) {
        Ok(s) => {
            if s.len() > limit {
                format!("{}... (truncated, total {} chars)", &s[..limit], s.len())
            } else {
                s
            }
        }
        Err(_) => format!("<binary, {} bytes>", bytes.len()),
    }
}

pub async fn proxy(
    db: &DatabaseConnection,
    oauth: &OauthService,
    http: &Client,
    user_id: Uuid,
    account_id: i64,
    method: &str,
    path: &str,
    query: Option<&serde_json::Value>,
    body: Option<&serde_json::Value>,
) -> Result<(u16, Vec<u8>)> {
    let access_token = oauth.get_access_token_for_account(user_id, account_id).await?;
    let url = if let Some(q) = query {
        let qs = json_to_query_string(q);
        if qs.is_empty() {
            format!("{}/{}", MAPI_BASE.trim_end_matches('/'), path.trim_start_matches('/'))
        } else {
            format!(
                "{}/{}?{}",
                MAPI_BASE.trim_end_matches('/'),
                path.trim_start_matches('/'),
                qs
            )
        }
    } else {
        format!("{}/{}", MAPI_BASE.trim_end_matches('/'), path.trim_start_matches('/'))
    };
    let mut req = match method.to_uppercase().as_str() {
        "GET" => http.get(&url),
        "POST" => http.post(&url),
        "PUT" => http.put(&url),
        "DELETE" => http.delete(&url),
        "PATCH" => http.patch(&url),
        _ => return Err(AppError::BadRequest("unsupported method".into())),
    };
    req = req.header("Authorization", format!("Bearer {}", access_token));
    if let Some(b) = body {
        req = req.json(b);
    }

    let request_body_log = body
        .map(|b| serde_json::to_string(b).unwrap_or_else(|_| "<invalid json>".into()))
        .unwrap_or_else(|| "(none)".to_string());
    let request_body_db = if request_body_log.len() > DB_BODY_MAX {
        format!("{}... (truncated)", &request_body_log[..DB_BODY_MAX])
    } else {
        request_body_log.clone()
    };
    let req_body_display = if request_body_log.len() > BODY_LOG_LIMIT {
        format!("{}... (truncated)", &request_body_log[..BODY_LOG_LIMIT])
    } else {
        request_body_log
    };
    info!(
        mapi_url = %url,
        mapi_method = %method,
        mapi_request_body = %req_body_display,
        "MAPI external call"
    );

    let method_upper = method.to_uppercase();
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            let err_msg = e.to_string();
            if let Err(le) = crate::service::log_service::log_external_request(
                db,
                "mapi",
                &url,
                &method_upper,
                Some(&request_body_db),
                None,
                None,
                Some(&err_msg),
                Some(user_id),
            )
            .await
            {
                warn!(error = %le, "external_request_log insert failed");
            }
            return Err(AppError::Upstream(err_msg));
        }
    };
    let status = resp.status().as_u16();
    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            let err_msg = e.to_string();
            if let Err(le) = crate::service::log_service::log_external_request(
                db,
                "mapi",
                &url,
                &method_upper,
                Some(&request_body_db),
                Some(status),
                None,
                Some(&err_msg),
                Some(user_id),
            )
            .await
            {
                warn!(error = %le, "external_request_log insert failed");
            }
            return Err(AppError::Upstream(err_msg));
        }
    };
    let bytes_slice = bytes.as_ref();
    let response_body_db = String::from_utf8_lossy(bytes_slice);
    let response_body_db = if response_body_db.len() > DB_BODY_MAX {
        format!("{}... (truncated)", &response_body_db[..DB_BODY_MAX])
    } else {
        response_body_db.to_string()
    };
    if let Err(le) = crate::service::log_service::log_external_request(
        db,
        "mapi",
        &url,
        &method_upper,
        Some(&request_body_db),
        Some(status),
        Some(&response_body_db),
        None,
        Some(user_id),
    )
    .await
    {
        warn!(error = %le, "external_request_log insert failed");
    }
    let response_log = truncate_for_log(bytes_slice, BODY_LOG_LIMIT);
    info!(
        mapi_status = status,
        mapi_response_body = %response_log,
        "MAPI external response"
    );
    Ok((status, bytes.to_vec()))
}

fn json_to_query_string(v: &serde_json::Value) -> String {
    let Some(obj) = v.as_object() else {
        return String::new();
    };
    let mut pairs: Vec<String> = Vec::new();
    for (k, v) in obj {
        let s = match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            _ => continue,
        };
        pairs.push(format!("{}={}", urlencoding::encode(k), urlencoding::encode(&s)));
    }
    pairs.join("&")
}
