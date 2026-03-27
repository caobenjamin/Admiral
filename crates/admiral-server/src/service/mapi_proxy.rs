//! MAPI proxy: resolve token by (user_id, account_id), forward request to B站.
//! 请求与响应（含报错）落表 external_request_log。
//! 报表 custom/query 请求在转发前做校验与规范化（order_by 数组、type 字符串、dimensions/metrics 合法）。

use crate::error::{AppError, Result};
use crate::service::oauth_service::OauthService;
use reqwest::Client;
use sea_orm::DatabaseConnection;
use tracing::{info, warn};
use uuid::Uuid;

const DB_BODY_MAX: usize = 65535;

const MAPI_BASE: &str = "https://cm.bilibili.com/takumi/api";
const BODY_LOG_LIMIT: usize = 2048;

/// 报表 custom/query 路径（用于识别是否需要校验与规范化 body）
const REPORT_CUSTOM_QUERY_PATH: &str = "open_api/report/v3/custom/query";

/// dimensions 允许的枚举值（文档允许；不包含 date，统一用 date_time）
const ALLOWED_DIMENSIONS: &[&str] = &[
    "date_time",
    "account_id",
    "campaign_id",
    "unit_id",
    "creative_id",
];

/// metrics 至少包含的指标（报表常用且排序字段须在其中）
const DEFAULT_METRICS: &[&str] = &["show_count", "click_count", "charged_cost_milli"];


/// 校验并规范化报表 custom/query 的 body：order_by 为长度 1 的数组，type 为字符串 "1"/"2"，
/// dimensions 仅允许值，metrics 至少包含 show_count/click_count/charged_cost_milli。
fn validate_and_normalize_report_body(body: &serde_json::Value) -> Result<serde_json::Value> {
    let mut obj = body
        .as_object()
        .ok_or_else(|| AppError::BadRequest("report body must be JSON object".into()))?
        .clone();

    // --- order_by: 必须是数组且长度为 1，field 在 metrics 中，type 为 "1" 或 "2" ---
    let metrics_arr: Vec<String> = obj
        .get("metrics")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::BadRequest("report body must have metrics array".into()))?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    let metrics_set: std::collections::HashSet<String> = metrics_arr.iter().cloned().collect();

    let order_by_raw = obj
        .get("order_by")
        .ok_or_else(|| AppError::BadRequest("report body must have order_by".into()))?;
    let order_by_arr: Vec<serde_json::Value> = if order_by_raw.is_array() {
        order_by_raw.as_array().cloned().unwrap_or_default()
    } else if order_by_raw.is_object() {
        vec![order_by_raw.clone()]
    } else {
        return Err(AppError::BadRequest("order_by must be array or single object".into()));
    };
    if order_by_arr.len() != 1 {
        return Err(AppError::BadRequest(
            "order_by must be array with exactly one element".into(),
        ));
    }
    let ob = order_by_arr[0].as_object().ok_or_else(|| {
        AppError::BadRequest("order_by[0] must be object with field and type".into())
    })?;
    let field = ob
        .get("field")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("order_by[0].field required".into()))?
        .to_string();
    if !metrics_set.contains(&field) {
        return Err(AppError::BadRequest(format!(
            "order_by.field must be one of metrics; got {}",
            field
        )));
    }
    let type_val = ob.get("type").ok_or_else(|| {
        AppError::BadRequest("order_by[0].type required (\"1\" asc / \"2\" desc)".into())
    })?;
    let type_str = match type_val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => {
            let s = n.to_string();
            if s == "1" || s == "2" {
                s
            } else {
                return Err(AppError::BadRequest(
                    "order_by[0].type must be \"1\" or \"2\"".into(),
                ));
            }
        }
        _ => {
            return Err(AppError::BadRequest(
                "order_by[0].type must be string \"1\" or \"2\"".into(),
            ));
        }
    };
    if type_str != "1" && type_str != "2" {
        return Err(AppError::BadRequest(
            "order_by[0].type must be \"1\" (asc) or \"2\" (desc)".into(),
        ));
    }
    let order_by_normalized = serde_json::json!([{ "field": field, "type": type_str }]);
    obj.insert("order_by".to_string(), order_by_normalized);

    // --- dimensions: 仅保留允许的枚举值；非法值（如 date）替换为 date_time ---
    if let Some(dims) = obj.get("dimensions").and_then(|v| v.as_array()) {
        let allowed: std::collections::HashSet<&str> = ALLOWED_DIMENSIONS.iter().copied().collect();
        let normalized: Vec<serde_json::Value> = dims
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| {
                if s == "date" {
                    "date_time".to_string()
                } else if allowed.contains(s) {
                    s.to_string()
                } else {
                    "date_time".to_string()
                }
            })
            .map(serde_json::Value::String)
            .collect();
        let mut uniq = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for v in normalized {
            if let Some(s) = v.as_str() {
                if seen.insert(s.to_string()) {
                    uniq.push(v);
                }
            }
        }
        obj.insert("dimensions".to_string(), serde_json::Value::Array(uniq));
    }

    // --- metrics: 至少包含 show_count, click_count, charged_cost_milli ---
    let mut metrics: Vec<String> = metrics_arr;
    let default_set: std::collections::HashSet<&str> =
        DEFAULT_METRICS.iter().copied().collect();
    for d in default_set {
        if !metrics.iter().any(|m| m.as_str() == d) {
            metrics.push(d.to_string());
        }
    }
    obj.insert(
        "metrics".to_string(),
        serde_json::Value::Array(
            metrics
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        ),
    );

    Ok(serde_json::Value::Object(obj))
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

    let body_to_send: Option<serde_json::Value> = if path.contains(REPORT_CUSTOM_QUERY_PATH) {
        body.map(|b| validate_and_normalize_report_body(b)).transpose()?
    } else {
        body.cloned()
    };

    if let Some(ref b) = body_to_send {
        req = req.json(b);
    }

    let request_body_log = body_to_send
        .as_ref()
        .or(body)
        .map(|b| serde_json::to_string(b).unwrap_or_else(|_| "<invalid json>".into()))
        .unwrap_or_else(|| "(none)".to_string());
    let request_body_db = crate::utils::truncate_for_db(&request_body_log, DB_BODY_MAX);
    let req_body_display = crate::utils::truncate_for_db(&request_body_log, BODY_LOG_LIMIT);
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
    let response_body_db = crate::utils::truncate_for_db(&String::from_utf8_lossy(bytes_slice), DB_BODY_MAX);
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
    let response_log = crate::utils::bytes_preview(bytes_slice, BODY_LOG_LIMIT);
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
