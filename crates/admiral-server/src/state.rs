//! Application state (DB, JWT, config, OAuth) for Axum.

use crate::config::AppConfig;
use crate::infra::jwt::JwtKeys;
use crate::middleware::auth_guard::AuthState;
use crate::service::llm_router::LlmProxy;
use crate::service::oauth_service::OauthService;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub jwt: Option<Arc<JwtKeys>>,
    pub config: AppConfig,
    pub oauth: Option<Arc<OauthService>>,
    pub llm: Option<Arc<LlmProxy>>,
}

impl AuthState for AppState {
    fn jwt_keys(&self) -> Option<&JwtKeys> {
        self.jwt.as_deref()
    }
}
