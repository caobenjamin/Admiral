//! SeaORM migrations.

pub use sea_orm_migration::prelude::*;

mod m20240219_000001_create_users_invites_entitlements_devices;
mod m20240219_000002_create_oauth_tokens;
mod m20240219_000003_create_ad_account_bindings;
mod m20240220_000004_oauth_tokens_add_bid_username;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240219_000001_create_users_invites_entitlements_devices::Migration),
            Box::new(m20240219_000002_create_oauth_tokens::Migration),
            Box::new(m20240219_000003_create_ad_account_bindings::Migration),
            Box::new(m20240220_000004_oauth_tokens_add_bid_username::Migration),
        ]
    }
}
