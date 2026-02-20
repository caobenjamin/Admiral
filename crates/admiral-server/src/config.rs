//! Server configuration (config-rs + env for secrets).

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub max_connections: u32,
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
}

fn default_min_connections() -> u32 {
    2
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub access_token_ttl_seconds: u64,
    pub refresh_token_ttl_seconds: u64,
    #[serde(default)]
    pub login_rate_limit_attempts: u32,
    #[serde(default)]
    pub login_rate_limit_window_seconds: u64,
    #[serde(default)]
    pub login_lockout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BilibiliOAuthConfig {
    #[serde(default)]
    pub app_id: String,
    #[serde(default)]
    pub app_secret: String,
    #[serde(default)]
    pub callback_url: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct OAuthConfig {
    #[serde(default = "default_state_cache_ttl")]
    pub state_cache_ttl_seconds: u64,
    #[serde(default = "default_token_refresh_hours")]
    pub token_refresh_interval_hours: u64,
    #[serde(default = "default_max_accounts_per_user")]
    pub default_max_accounts_per_user: u32,
}

fn default_state_cache_ttl() -> u64 {
    600
}
fn default_token_refresh_hours() -> u64 {
    10
}
fn default_max_accounts_per_user() -> u32 {
    10
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    pub default_provider: String,
    pub request_timeout_seconds: u64,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub bilibili: BilibiliOAuthConfig,
    #[serde(default)]
    pub oauth: OAuthConfig,
    pub llm: LlmConfig,
    pub logging: LoggingConfig,
}

/// Sensitive and env-only settings (not in config files).
#[derive(Debug, Clone)]
pub struct EnvSecrets {
    pub database_url: String,
    pub jwt_private_key_path: PathBuf,
    pub jwt_public_key_path: PathBuf,
    pub token_encryption_key_hex: Option<String>,
    pub bilibili_app_id: String,
    pub bilibili_app_secret: String,
    pub bilibili_callback_url: String,
    pub llm_default_provider: Option<String>,
    pub deepseek_api_key: Option<String>,
    pub doubao_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
}

impl EnvSecrets {
    /// Load secrets from environment. Only ADMIRAL_DATABASE_URL is required for basic run.
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let database_url = std::env::var("ADMIRAL_DATABASE_URL")
            .map_err(|_| config::ConfigError::NotFound("ADMIRAL_DATABASE_URL".into()))?;
        let jwt_private_key_path = std::env::var("ADMIRAL_JWT_PRIVATE_KEY_PATH")
            .unwrap_or_else(|_| "./keys/private.pem".into())
            .into();
        let jwt_public_key_path = std::env::var("ADMIRAL_JWT_PUBLIC_KEY_PATH")
            .unwrap_or_else(|_| "./keys/public.pem".into())
            .into();
        let token_encryption_key_hex = std::env::var("ADMIRAL_TOKEN_ENCRYPTION_KEY").ok();
        let bilibili_app_id = std::env::var("ADMIRAL_BILI_APP_ID").unwrap_or_default();
        let bilibili_app_secret = std::env::var("ADMIRAL_BILI_APP_SECRET").unwrap_or_default();
        let bilibili_callback_url =
            std::env::var("ADMIRAL_BILI_CALLBACK_URL").unwrap_or_default();
        let llm_default_provider = std::env::var("ADMIRAL_LLM_DEFAULT_PROVIDER").ok();
        let deepseek_api_key = std::env::var("ADMIRAL_DEEPSEEK_API_KEY").ok();
        let doubao_api_key = std::env::var("ADMIRAL_DOUBAO_API_KEY").ok();
        let openai_api_key = std::env::var("ADMIRAL_OPENAI_API_KEY").ok();
        let gemini_api_key = std::env::var("ADMIRAL_GEMINI_API_KEY").ok();

        Ok(Self {
            database_url,
            jwt_private_key_path,
            jwt_public_key_path,
            token_encryption_key_hex,
            bilibili_app_id,
            bilibili_app_secret,
            bilibili_callback_url,
            llm_default_provider,
            deepseek_api_key,
            doubao_api_key,
            openai_api_key,
            gemini_api_key,
        })
    }
}

/// Resolve config directory: ADMIRAL_CONFIG_DIR, or config/ under cwd, or crates/admiral-server/config when run from workspace root.
fn config_dir() -> String {
    if let Ok(d) = std::env::var("ADMIRAL_CONFIG_DIR") {
        return d;
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    if cwd.join("config/default.toml").exists() {
        return "config".into();
    }
    if cwd.join("crates/admiral-server/config/default.toml").exists() {
        return "crates/admiral-server/config".into();
    }
    "config".into()
}

/// Load full config: merge config files + overlay env-only secrets.
pub fn load() -> Result<(AppConfig, EnvSecrets), config::ConfigError> {
    let run_mode = std::env::var("ADMIRAL_RUN_MODE").unwrap_or_else(|_| "development".into());
    let config_dir = config_dir();

    let app_config: AppConfig = config::Config::builder()
        .add_source(config::File::with_name(&format!("{}/default", config_dir)).required(true))
        .add_source(
            config::File::with_name(&format!("{}/{}", config_dir, run_mode)).required(false),
        )
        .add_source(
            config::Environment::with_prefix("ADMIRAL")
                .separator("__")
                .try_parsing(true),
        )
        .build()?
        .try_deserialize()?;

    let secrets = EnvSecrets::from_env()?;

    Ok((app_config, secrets))
}
