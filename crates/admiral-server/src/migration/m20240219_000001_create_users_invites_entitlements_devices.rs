use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                email VARCHAR(255) UNIQUE NOT NULL,
                password_hash VARCHAR(255) NOT NULL,
                name VARCHAR(100) NOT NULL,
                company VARCHAR(200),
                role VARCHAR(20) DEFAULT 'user',
                status VARCHAR(20) DEFAULT 'active',
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW()
            );
            CREATE TABLE IF NOT EXISTS devices (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID NOT NULL REFERENCES users(id),
                device_id VARCHAR(64) NOT NULL,
                device_name VARCHAR(100),
                os_type VARCHAR(20),
                is_active BOOLEAN DEFAULT FALSE,
                last_seen_at TIMESTAMPTZ,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                UNIQUE(user_id, device_id)
            );
            CREATE TABLE IF NOT EXISTS invites (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                code VARCHAR(20) UNIQUE NOT NULL,
                trial_days INTEGER NOT NULL DEFAULT 7,
                max_uses INTEGER DEFAULT 1,
                used_count INTEGER DEFAULT 0,
                created_by UUID REFERENCES users(id),
                expires_at TIMESTAMPTZ,
                created_at TIMESTAMPTZ DEFAULT NOW()
            );
            CREATE TABLE IF NOT EXISTS entitlements (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID NOT NULL REFERENCES users(id),
                invite_id UUID REFERENCES invites(id),
                type VARCHAR(20) DEFAULT 'trial',
                starts_at TIMESTAMPTZ NOT NULL,
                expires_at TIMESTAMPTZ NOT NULL,
                is_active BOOLEAN DEFAULT TRUE,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                extended_by UUID REFERENCES users(id),
                extend_reason TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_entitlements_user ON entitlements(user_id, is_active);
            "#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            DROP TABLE IF EXISTS entitlements;
            DROP TABLE IF EXISTS invites;
            DROP TABLE IF EXISTS devices;
            DROP TABLE IF EXISTS users;
            "#,
        )
        .await?;
        Ok(())
    }
}
