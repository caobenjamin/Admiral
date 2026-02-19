//! MAPI proxy: resolve token by (user_id, account_id), forward request to B站.

use crate::error::{AppError, Result};
use crate::service::oauth_service::OauthService;
use reqwest::Client;
use uuid::Uuid;

const MAPI_BASE: &str = "https://cm.bilibili.com/takumi/api";

pub async fn proxy(
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
    let resp = req.send().await.map_err(|e| AppError::Upstream(e.to_string()))?;
    let status = resp.status().as_u16();
    let bytes = resp.bytes().await.map_err(|e| AppError::Upstream(e.to_string()))?;
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
