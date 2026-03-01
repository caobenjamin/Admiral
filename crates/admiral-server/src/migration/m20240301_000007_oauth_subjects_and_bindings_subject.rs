//! oauth_subjects 表：记录 token 对应的多个 subject；
//! ad_account_bindings 增加 subject_type、subject_id，绑定时记录所选组织。

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
                CREATE TABLE IF NOT EXISTS oauth_subjects (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    oauth_token_id UUID NOT NULL REFERENCES oauth_tokens(id) ON DELETE CASCADE,
                    subject_type INT NOT NULL,
                    subject_id INT NOT NULL,
                    created_at TIMESTAMPTZ DEFAULT NOW(),
                    UNIQUE(oauth_token_id, subject_type, subject_id)
                );
                CREATE INDEX IF NOT EXISTS idx_oauth_subjects_oauth_token_id ON oauth_subjects(oauth_token_id);

                ALTER TABLE ad_account_bindings ADD COLUMN IF NOT EXISTS subject_type INT;
                ALTER TABLE ad_account_bindings ADD COLUMN IF NOT EXISTS subject_id INT;
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
                ALTER TABLE ad_account_bindings DROP COLUMN IF EXISTS subject_type;
                ALTER TABLE ad_account_bindings DROP COLUMN IF EXISTS subject_id;
                DROP TABLE IF EXISTS oauth_subjects;
                "#,
            )
            .await?;
        Ok(())
    }
}
