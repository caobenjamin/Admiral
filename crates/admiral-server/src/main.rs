//! Admiral server binary.

use admiral_server::config;
use admiral_server::infra::db;
use admiral_server::infra::jwt::JwtKeys;
use admiral_server::migration::{Migrator, MigratorTrait};
use admiral_server::service::{oauth_service::OauthService, token_vault::TokenVault};
use admiral_server::state::AppState;
use axum::middleware::from_fn;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let (app_config, secrets) = config::load().map_err(|e| anyhow::anyhow!("config: {}", e))?;

    let level = std::env::var("ADMIRAL_LOG_LEVEL").unwrap_or_else(|_| app_config.logging.level.clone());
    let use_json = app_config.logging.format == "json";
    if use_json {
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::new(level))
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::new(level))
            .with(tracing_subscriber::fmt::layer().pretty())
            .init();
    }

    let db = db::connect(
        &secrets.database_url,
        app_config.database.max_connections,
        app_config.database.min_connections,
    )
    .await?;

    Migrator::up(&db, None).await?;

    let jwt = match JwtKeys::from_rsa_pem_files(&secrets.jwt_private_key_path, &secrets.jwt_public_key_path) {
        Ok(k) => Some(Arc::new(k)),
        Err(e) => {
            tracing::warn!("JWT keys not loaded (auth endpoints will fail): {}", e);
            None
        }
    };

    let oauth = match (
        secrets.token_encryption_key_hex.as_deref(),
        !secrets.bilibili_app_id.is_empty(),
    ) {
        (Some(key), true) => {
            let vault = TokenVault::new(db.clone(), key).map_err(|e| anyhow::anyhow!("vault: {}", e))?;
            let svc = OauthService::new(
                secrets.bilibili_app_id.clone(),
                secrets.bilibili_app_secret.clone(),
                secrets.bilibili_callback_url.clone(),
                app_config.oauth.state_cache_ttl_seconds,
                app_config.oauth.default_max_accounts_per_user,
                vault,
                db.clone(),
            );
            let oauth = Arc::new(svc);
            // 启动时立刻检查并刷新所有即将过期的 token
            oauth.run_refresh_cycle_full().await;
            let oauth_clone = Arc::clone(&oauth);
            let interval_secs = app_config.oauth.token_refresh_interval_hours * 3600;
            tokio::spawn(async move {
                let interval = Duration::from_secs(interval_secs);
                loop {
                    tokio::time::sleep(interval).await;
                    oauth_clone.run_refresh_cycle().await;
                }
            });
            Some(oauth)
        }
        _ => {
            tracing::debug!("OAuth not configured (missing key or Bilibili credentials)");
            None
        }
    };

    let llm = admiral_server::service::llm_router::from_env(
        &app_config.llm.default_provider,
        secrets.deepseek_api_key.as_deref(),
        secrets.openai_api_key.as_deref(),
    )
    .map(Arc::new);

    let state = AppState {
        db,
        jwt,
        config: app_config.clone(),
        oauth,
        llm,
        http: reqwest::Client::new(),
    };

    let app = admiral_server::api::router::app(state)
        .layer(from_fn(admiral_server::middleware::request_id::request_id_layer))
        .layer(from_fn(admiral_server::middleware::logging::logging_layer));

    let addr = SocketAddr::from((
        app_config.server.host.parse::<std::net::IpAddr>()?,
        app_config.server.port,
    ));
    tracing::info!(%addr, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
