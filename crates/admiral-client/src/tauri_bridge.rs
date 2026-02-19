//! Tauri command bridge: managed state and invoke handlers.

use crate::commands::auth::{self, AuthContext};
use crate::config::ClientConfig;
use crate::credential::PlainFileCredentialStore;
use crate::network::api_client::ApiClient;
use admiral_types::api::*;
use std::sync::Arc;

/// App state held by Tauri (thread-safe).
pub struct AppState {
    pub auth: Arc<AuthContext>,
}

fn err_string(e: impl std::fmt::Display) -> String {
    e.to_string()
}

// ---------- Auth commands (invokable from frontend) ----------

#[tauri::command]
pub async fn auth_register(
    state: tauri::State<'_, AppState>,
    req: RegisterRequest,
) -> Result<TokenResponse, String> {
    auth::register(&state.auth, req).await.map_err(err_string)
}

#[tauri::command]
pub async fn auth_login(
    state: tauri::State<'_, AppState>,
    req: LoginRequest,
) -> Result<TokenResponse, String> {
    auth::login(&state.auth, req).await.map_err(err_string)
}

#[tauri::command]
pub async fn auth_refresh(state: tauri::State<'_, AppState>) -> Result<RefreshResponse, String> {
    auth::refresh(&state.auth).await.map_err(err_string)
}

#[tauri::command]
pub async fn auth_logout(state: tauri::State<'_, AppState>) -> Result<(), String> {
    auth::logout(&state.auth).map_err(err_string)
}

#[tauri::command]
pub async fn auth_me(state: tauri::State<'_, AppState>) -> Result<UserInfo, String> {
    auth::me(&state.auth).await.map_err(err_string)
}

#[tauri::command]
pub async fn auth_license(state: tauri::State<'_, AppState>) -> Result<LicenseResponse, String> {
    auth::license(&state.auth).await.map_err(err_string)
}

// ---------- OAuth ----------

#[tauri::command]
pub async fn oauth_initiate(state: tauri::State<'_, AppState>) -> Result<InitiateResponse, String> {
    crate::commands::oauth::oauth_initiate(&state.auth).await.map_err(err_string)
}

#[tauri::command]
pub async fn oauth_status(state: tauri::State<'_, AppState>) -> Result<Vec<TokenMeta>, String> {
    crate::commands::oauth::oauth_status(&state.auth).await.map_err(err_string)
}

#[tauri::command]
pub async fn oauth_accounts_list(
    state: tauri::State<'_, AppState>,
    oauth_session_id: Option<String>,
) -> Result<admiral_types::api::OAuthAccountsResponse, String> {
    let id = oauth_session_id
        .and_then(|s| uuid::Uuid::parse_str(&s).ok());
    crate::commands::oauth::oauth_accounts_list(&state.auth, id)
        .await
        .map_err(err_string)
}

#[tauri::command]
pub async fn oauth_bind_accounts(
    state: tauri::State<'_, AppState>,
    req: BindAccountsRequest,
) -> Result<(), String> {
    crate::commands::oauth::oauth_bind_accounts(&state.auth, req)
        .await
        .map_err(err_string)
}

#[tauri::command]
pub async fn oauth_delete_session(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let id = uuid::Uuid::parse_str(&session_id).map_err(err_string)?;
    crate::commands::oauth::oauth_delete_session(&state.auth, id)
        .await
        .map_err(err_string)
}

// ---------- MAPI ----------

#[tauri::command]
pub async fn mapi_proxy(
    state: tauri::State<'_, AppState>,
    req: MapiProxyRequest,
) -> Result<(u16, Vec<u8>), String> {
    crate::commands::mapi::mapi_proxy(&state.auth, req)
        .await
        .map_err(err_string)
}

// ---------- LLM ----------

#[tauri::command]
pub async fn llm_chat(
    state: tauri::State<'_, AppState>,
    req: LlmChatRequest,
) -> Result<Vec<u8>, String> {
    crate::commands::llm::llm_chat(&state.auth, req)
        .await
        .map_err(err_string)
}

#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        open::that(&url).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Create AppState from config (used at startup).
pub fn create_app_state() -> AppState {
    let config = ClientConfig::from_env();
    let store = Arc::new(PlainFileCredentialStore::from_config(&config));
    let api_client = ApiClient::new(config.api_base_url);
    let auth = AuthContext::new(api_client, store);
    AppState {
        auth: Arc::new(auth),
    }
}
