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
                CREATE TABLE IF NOT EXISTS oauth_tokens (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    user_id UUID NOT NULL REFERENCES users(id),
                    platform VARCHAR(50) NOT NULL DEFAULT 'bilibili',
                    access_token_encrypted BYTEA NOT NULL,
                    refresh_token_encrypted BYTEA NOT NULL,
                    token_nonce BYTEA NOT NULL,
                    expires_at TIMESTAMPTZ NOT NULL,
                    refresh_expires_at TIMESTAMPTZ,
                    created_at TIMESTAMPTZ DEFAULT NOW(),
                    updated_at TIMESTAMPTZ DEFAULT NOW()
                );
                CREATE INDEX IF NOT EXISTS idx_oauth_tokens_user_platform ON oauth_tokens(user_id, platform);
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS oauth_tokens;")
            .await?;
        Ok(())
    }
}
