//! SeaORM migrations.

pub use sea_orm_migration::prelude::*;

mod m20240219_000001_create_users_invites_entitlements_devices;
mod m20240219_000002_create_oauth_tokens;
mod m20240219_000003_create_ad_account_bindings;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240219_000001_create_users_invites_entitlements_devices::Migration),
            Box::new(m20240219_000002_create_oauth_tokens::Migration),
            Box::new(m20240219_000003_create_ad_account_bindings::Migration),
        ]
    }
}
