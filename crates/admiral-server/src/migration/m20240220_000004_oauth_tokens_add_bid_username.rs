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
                ALTER TABLE oauth_tokens ADD COLUMN IF NOT EXISTS bid BIGINT;
                ALTER TABLE oauth_tokens ADD COLUMN IF NOT EXISTS user_name VARCHAR(200);
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
                ALTER TABLE oauth_tokens DROP COLUMN IF EXISTS bid;
                ALTER TABLE oauth_tokens DROP COLUMN IF EXISTS user_name;
                "#,
            )
            .await?;
        Ok(())
    }
}
