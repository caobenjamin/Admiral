//! 客户端 API 请求日志与外部调用（MAPI/LLM）日志表

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 客户端请求日志：来自客户端的 HTTP 调用
                CREATE TABLE IF NOT EXISTS api_request_log (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    created_at TIMESTAMPTZ DEFAULT NOW(),
                    method VARCHAR(10) NOT NULL,
                    path TEXT NOT NULL,
                    request_body TEXT,
                    user_id UUID,
                    client_ip VARCHAR(45),
                    response_status SMALLINT,
                    response_body TEXT,
                    error_message TEXT,
                    elapsed_ms BIGINT
                );
                CREATE INDEX IF NOT EXISTS idx_api_request_log_created_at ON api_request_log(created_at);
                CREATE INDEX IF NOT EXISTS idx_api_request_log_user_id ON api_request_log(user_id);

                -- 外部请求日志：向 MAPI、大模型等发出的请求
                CREATE TABLE IF NOT EXISTS external_request_log (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    created_at TIMESTAMPTZ DEFAULT NOW(),
                    kind VARCHAR(20) NOT NULL,
                    request_url TEXT NOT NULL,
                    request_method VARCHAR(10) NOT NULL,
                    request_body TEXT,
                    response_status SMALLINT,
                    response_body TEXT,
                    error_message TEXT,
                    user_id UUID
                );
                CREATE INDEX IF NOT EXISTS idx_external_request_log_created_at ON external_request_log(created_at);
                CREATE INDEX IF NOT EXISTS idx_external_request_log_kind ON external_request_log(kind);
                CREATE INDEX IF NOT EXISTS idx_external_request_log_user_id ON external_request_log(user_id);
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP TABLE IF EXISTS api_request_log;
                DROP TABLE IF EXISTS external_request_log;
                "#,
            )
            .await?;
        Ok(())
    }
}
