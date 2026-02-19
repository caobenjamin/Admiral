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
                CREATE TABLE IF NOT EXISTS ad_account_bindings (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    user_id UUID NOT NULL REFERENCES users(id),
                    platform VARCHAR(50) NOT NULL DEFAULT 'bilibili',
                    account_id VARCHAR(100) NOT NULL,
                    oauth_token_id UUID NOT NULL REFERENCES oauth_tokens(id),
                    account_name VARCHAR(200),
                    created_at TIMESTAMPTZ DEFAULT NOW(),
                    UNIQUE(user_id, platform, account_id)
                );
                CREATE INDEX IF NOT EXISTS idx_ad_account_bindings_user_platform ON ad_account_bindings(user_id, platform);
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS ad_account_bindings;")
            .await?;
        Ok(())
    }
}
