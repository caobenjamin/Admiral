//! 请求落表：客户端 API 请求日志、外部调用（MAPI/LLM）日志。

use crate::error::Result;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use uuid::Uuid;

/// 客户端请求落表：原始请求、用户 IP、用户 ID、返回内容（报错也落表）。
pub async fn log_api_request(
    db: &sea_orm::DatabaseConnection,
    method: &str,
    path: &str,
    request_body: Option<&str>,
    user_id: Option<Uuid>,
    client_ip: Option<&str>,
    response_status: Option<u16>,
    response_body: Option<&str>,
    error_message: Option<&str>,
    elapsed_ms: Option<u64>,
) -> Result<()> {
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        r#"
        INSERT INTO api_request_log (
            method, path, request_body, user_id, client_ip,
            response_status, response_body, error_message, elapsed_ms
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
        [
            sea_orm::Value::String(Some(Box::new(method.to_string()))),
            sea_orm::Value::String(Some(Box::new(path.to_string()))),
            request_body.map(|s| sea_orm::Value::String(Some(Box::new(s.to_string()))))
                .unwrap_or(sea_orm::Value::String(None)),
            user_id.map(|u| sea_orm::Value::Uuid(Some(Box::new(u)))).unwrap_or(sea_orm::Value::Uuid(None)),
            client_ip.map(|s| sea_orm::Value::String(Some(Box::new(s.to_string()))))
                .unwrap_or(sea_orm::Value::String(None)),
            response_status.map(|n| sea_orm::Value::SmallInt(Some(n as i16)))
                .unwrap_or(sea_orm::Value::SmallInt(None)),
            response_body.map(|s| sea_orm::Value::String(Some(Box::new(s.to_string()))))
                .unwrap_or(sea_orm::Value::String(None)),
            error_message.map(|s| sea_orm::Value::String(Some(Box::new(s.to_string()))))
                .unwrap_or(sea_orm::Value::String(None)),
            elapsed_ms.map(|n| sea_orm::Value::BigInt(Some(n as i64)))
                .unwrap_or(sea_orm::Value::BigInt(None)),
        ],
    );
    db.execute(stmt).await?;
    Ok(())
}

/// 外部请求落表：MAPI、大模型等。请求地址、请求内容、返回内容；报错也落表。
pub async fn log_external_request(
    db: &sea_orm::DatabaseConnection,
    kind: &str,
    request_url: &str,
    request_method: &str,
    request_body: Option<&str>,
    response_status: Option<u16>,
    response_body: Option<&str>,
    error_message: Option<&str>,
    user_id: Option<Uuid>,
) -> Result<()> {
    let stmt = Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        r#"
        INSERT INTO external_request_log (
            kind, request_url, request_method, request_body,
            response_status, response_body, error_message, user_id
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
        [
            sea_orm::Value::String(Some(Box::new(kind.to_string()))),
            sea_orm::Value::String(Some(Box::new(request_url.to_string()))),
            sea_orm::Value::String(Some(Box::new(request_method.to_string()))),
            request_body.map(|s| sea_orm::Value::String(Some(Box::new(s.to_string()))))
                .unwrap_or(sea_orm::Value::String(None)),
            response_status.map(|n| sea_orm::Value::SmallInt(Some(n as i16)))
                .unwrap_or(sea_orm::Value::SmallInt(None)),
            response_body.map(|s| sea_orm::Value::String(Some(Box::new(s.to_string()))))
                .unwrap_or(sea_orm::Value::String(None)),
            error_message.map(|s| sea_orm::Value::String(Some(Box::new(s.to_string()))))
                .unwrap_or(sea_orm::Value::String(None)),
            user_id.map(|u| sea_orm::Value::Uuid(Some(Box::new(u)))).unwrap_or(sea_orm::Value::Uuid(None)),
        ],
    );
    db.execute(stmt).await?;
    Ok(())
}
