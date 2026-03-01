//! Tauri command bridge: managed state and invoke handlers.

use crate::commands::auth::{self, AuthContext};
use crate::config::ClientConfig;
use crate::credential::PlainFileCredentialStore;
use crate::network::api_client::ApiClient;
use crate::agent::runtime::{AgentResponse, AgentRuntime};
use crate::agent::context::Session;
use crate::skill::registry::SkillRegistry;
use crate::task::engine::TaskEngine;
use admiral_types::api::*;
use admiral_types::task::{Task, TaskStatus, TaskType};
use std::sync::Arc;
use uuid::Uuid;

/// App state held by Tauri (thread-safe).
pub struct AppState {
    pub auth: Arc<AuthContext>,
    pub task_engine: Arc<TaskEngine>,
    pub agent_runtime: Arc<tokio::sync::RwLock<Option<AgentRuntime>>>,
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
pub async fn oauth_bindings_list(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<admiral_types::api::BoundAccountRecord>, String> {
    crate::commands::oauth::oauth_bindings_list(&state.auth)
        .await
        .map_err(err_string)
}

#[tauri::command]
pub async fn oauth_subjects_list(
    state: tauri::State<'_, AppState>,
    oauth_session_id: Option<String>,
) -> Result<Vec<admiral_types::api::BiliSubjectRecord>, String> {
    let id = oauth_session_id
        .and_then(|s| uuid::Uuid::parse_str(&s).ok());
    crate::commands::oauth::oauth_subjects_list(&state.auth, id)
        .await
        .map_err(err_string)
}

#[tauri::command]
pub async fn oauth_accounts_list(
    state: tauri::State<'_, AppState>,
    oauth_session_id: Option<String>,
    subject_type: i32,
    subject_id: i32,
) -> Result<admiral_types::api::OAuthAccountsResponse, String> {
    let id = oauth_session_id
        .and_then(|s| uuid::Uuid::parse_str(&s).ok());
    crate::commands::oauth::oauth_accounts_list(&state.auth, id, subject_type, subject_id)
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

#[tauri::command]
pub async fn oauth_unbind_account(
    state: tauri::State<'_, AppState>,
    account_id: i64,
) -> Result<(), String> {
    crate::commands::oauth::oauth_unbind_account(&state.auth, account_id)
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

// ---------- Client config (persistent config file) ----------

/// Path to the config file (e.g. ~/.config/admiral/client.json). Frontend can show this for manual edit.
#[tauri::command]
pub fn get_config_path() -> Option<String> {
    ClientConfig::config_file_path().and_then(|p| p.into_os_string().into_string().ok())
}

/// Current server API base URL (from config file + env, as loaded at startup).
#[tauri::command]
pub fn get_server_url() -> String {
    ClientConfig::from_env().api_base_url
}

/// Save server URL to the persistent config file. Restart the app for the new URL to take effect.
#[tauri::command]
pub fn set_server_url(url: String) -> Result<(), String> {
    let mut config = ClientConfig::from_env();
    config.set_api_base_url(url);
    config.save().map_err(|e| e.to_string())
}

// ---------- Task commands ----------

#[tauri::command]
pub async fn task_create(
    state: tauri::State<'_, AppState>,
    name: String,
    description: Option<String>,
    task_type: TaskType,
) -> Result<Task, String> {
    crate::commands::task::create_task(
        state.task_engine.as_ref(),
        name,
        description,
        task_type,
    )
}

#[tauri::command]
pub async fn task_list(
    state: tauri::State<'_, AppState>,
    status_filter: Option<TaskStatus>,
) -> Result<Vec<Task>, String> {
    crate::commands::task::list_tasks(state.task_engine.as_ref(), status_filter)
}

#[tauri::command]
pub async fn task_get(state: tauri::State<'_, AppState>, id: String) -> Result<Option<Task>, String> {
    let id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    crate::commands::task::get_task(state.task_engine.as_ref(), id)
}

#[tauri::command]
pub async fn task_run_dry_run(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<admiral_types::task::TaskRun, String> {
    let id = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    crate::commands::task::run_dry_run(
        state.task_engine.as_ref(),
        state.auth.as_ref(),
        id,
    )
    .await
}

#[tauri::command]
pub async fn task_run_test_run(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<admiral_types::task::TaskRun, String> {
    let id = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    crate::commands::task::run_test_run(
        state.task_engine.as_ref(),
        state.auth.as_ref(),
        id,
    )
    .await
}

#[tauri::command]
pub async fn agent_chat(
    state: tauri::State<'_, AppState>,
    message: String,
    task_id: Option<String>,
) -> Result<AgentResponse, String> {
    let token = state.auth.get_access_token().map_err(|e| e.to_string())?;
    let session = Session {
        id: uuid::Uuid::new_v4().to_string(),
        task_id: task_id.and_then(|s| uuid::Uuid::parse_str(&s).ok()),
        messages: Vec::new(),
    };
    let guard = state.agent_runtime.write().await;
    let runtime = guard.as_ref().ok_or("Agent not initialized")?;
    runtime
        .process_message(session, &message, Some(&token))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn task_run_production(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<admiral_types::task::TaskRun, String> {
    let id = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    crate::commands::task::run_production(
        state.task_engine.as_ref(),
        state.auth.as_ref(),
        id,
    )
    .await
}

/// Create AppState from config (used at startup).
pub fn create_app_state() -> AppState {
    let config = ClientConfig::from_env();
    let store = Arc::new(PlainFileCredentialStore::from_config(&config));
    let api_client = Arc::new(ApiClient::new(config.api_base_url));
    let auth = AuthContext::new(Arc::clone(&api_client), store);

    let skill_registry = Arc::new(SkillRegistry::new());
    let _ = skill_registry.load_from_default_dir();
    let skill_executor = Arc::new(crate::skill::executor::SkillExecutor::new(
        Arc::clone(&skill_registry),
        api_client,
    ));
    let task_engine = Arc::new(TaskEngine::new(
        Arc::clone(&skill_registry),
        skill_executor,
    ));

    let agent_runtime = match AgentRuntime::new(Arc::clone(&auth.api_client), Arc::clone(&skill_registry)) {
        Ok(r) => Arc::new(tokio::sync::RwLock::new(Some(r))),
        Err(_) => Arc::new(tokio::sync::RwLock::new(None)),
    };

    AppState {
        auth: Arc::new(auth),
        task_engine,
        agent_runtime,
    }
}
