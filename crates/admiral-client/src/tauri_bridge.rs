//! Tauri command bridge: managed state and invoke handlers.

use crate::commands::auth::{self, AuthContext};
use crate::config::ClientConfig;
use crate::credential::PlainFileCredentialStore;
use crate::db;
use crate::network::api_client::ApiClient;
use crate::agent::memory::MemoryStore;
use crate::agent::memory_updater;
use crate::agent::runtime::{AgentResponse, AgentRuntime};
use crate::agent::context::{Message, Session};
use crate::skill::registry::SkillRegistry;
use crate::task::engine::TaskEngine;
use admiral_types::api::*;
use admiral_types::task::{Task, TaskCategory, TaskStatus, TaskType};
use std::sync::Arc;
use uuid::Uuid;

/// App state held by Tauri (thread-safe).
pub struct AppState {
    pub auth: Arc<AuthContext>,
    pub api_client: Arc<ApiClient>,
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

#[tauri::command]
pub fn get_config_path() -> Option<String> {
    ClientConfig::config_file_path().and_then(|p| p.into_os_string().into_string().ok())
}

#[tauri::command]
pub fn get_server_url() -> String {
    ClientConfig::from_env().api_base_url
}

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
    task_category: Option<TaskCategory>,
) -> Result<Task, String> {
    crate::commands::task::create_task(
        state.task_engine.as_ref(),
        name,
        description,
        task_type,
        task_category.unwrap_or_default(),
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
    crate::commands::task::run_task(
        state.task_engine.as_ref(),
        state.auth.as_ref(),
        id,
        admiral_types::task::RunType::DryRun,
    )
    .await
}

#[tauri::command]
pub async fn task_run_test_run(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<admiral_types::task::TaskRun, String> {
    let id = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    crate::commands::task::run_task(
        state.task_engine.as_ref(),
        state.auth.as_ref(),
        id,
        admiral_types::task::RunType::TestRun,
    )
    .await
}

#[tauri::command]
pub async fn task_run_production(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<admiral_types::task::TaskRun, String> {
    let id = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    crate::commands::task::run_task(
        state.task_engine.as_ref(),
        state.auth.as_ref(),
        id,
        admiral_types::task::RunType::Production,
    )
    .await
}

#[tauri::command]
pub async fn task_resume(
    state: tauri::State<'_, AppState>,
    task_id: String,
    from_step: Option<u32>,
    run_type: Option<String>,
) -> Result<admiral_types::task::TaskRun, String> {
    let id = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    let rt = match run_type.as_deref() {
        Some("dry_run") => admiral_types::task::RunType::DryRun,
        Some("test_run") => admiral_types::task::RunType::TestRun,
        _ => admiral_types::task::RunType::Production,
    };
    crate::commands::task::resume_task(
        state.task_engine.as_ref(),
        state.auth.as_ref(),
        id,
        from_step,
        rt,
    )
    .await
}

#[tauri::command]
pub async fn task_approve_operation(
    state: tauri::State<'_, AppState>,
    task_id: String,
    auto_approve_future: bool,
) -> Result<admiral_types::task::TaskRun, String> {
    let id = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    crate::commands::task::approve_operation(
        state.task_engine.as_ref(),
        state.auth.as_ref(),
        id,
        auto_approve_future,
    )
    .await
}

#[tauri::command]
pub async fn task_reject_operation(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<Task, String> {
    let id = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    crate::commands::task::reject_operation(state.task_engine.as_ref(), id)
}

#[tauri::command]
pub async fn task_get_pending_approval(
    task_id: String,
) -> Result<Option<admiral_types::task::PendingApproval>, String> {
    let id = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    crate::commands::task::get_pending_approval(id)
}

#[tauri::command]
pub async fn permission_list() -> Result<Vec<admiral_types::task::OperationPermission>, String> {
    crate::commands::task::list_permissions()
}

#[tauri::command]
pub async fn permission_revoke(id: String) -> Result<(), String> {
    crate::commands::task::revoke_permission(id)
}

// ---------- Agent chat (with conversation persistence) ----------

#[tauri::command]
pub async fn agent_chat(
    state: tauri::State<'_, AppState>,
    message: String,
    task_id: Option<String>,
) -> Result<AgentResponse, String> {
    let token = state.auth.get_access_token().map_err(|e| e.to_string())?;

    let task_uuid = task_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
    let session_id = task_uuid
        .map(|id| id.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Load conversation history from DB
    let mut messages: Vec<Message> = Vec::new();
    if let Some(ref tid) = task_id {
        let db_path = db::models::db_path();
        if let Ok(conn) = db::models::open_ro(&db_path) {
            if let Ok(rows) = db::models::load_conversations_by_task(&conn, tid) {
                for row in rows {
                    messages.push(Message {
                        role: row.role,
                        content: if row.content.is_empty() { None } else { Some(row.content) },
                        tool_calls: row.tool_calls.and_then(|s| serde_json::from_str(&s).ok()),
                        name: None,
                        tool_call_id: None,
                    });
                }
            }
        }
    }

    let msg_count_before = messages.len();

    let session = Session {
        id: session_id.clone(),
        task_id: task_uuid,
        messages,
    };

    let guard = state.agent_runtime.read().await;
    let runtime = guard.as_ref().ok_or("Agent not initialized")?;
    let response = runtime
        .process_message(session, &message, Some(&token))
        .await
        .map_err(|e| e.to_string())?;

    // Persist new messages to DB
    let new_messages = &response.session.messages[msg_count_before..];
    if !new_messages.is_empty() {
        let db_path = db::models::db_path();
        if let Ok(conn) = db::models::open_rw(&db_path) {
            for msg in new_messages {
                let row = db::models::ConversationRow {
                    id: Uuid::new_v4().to_string(),
                    task_id: task_id.clone(),
                    session_id: session_id.clone(),
                    role: msg.role.clone(),
                    content: msg.content.clone().unwrap_or_default(),
                    tool_calls: msg.tool_calls.as_ref().and_then(|v| serde_json::to_string(v).ok()),
                    tool_result: None,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                let _ = db::models::save_conversation_message(&conn, &row);
            }
        }
    }

    // Async memory update (non-blocking)
    {
        let api_client = Arc::clone(&state.api_client);
        let token_owned = token.clone();
        let user_msg = message.clone();
        let ai_response = response.content.clone();
        let task_id_clone = task_id.clone();
        let task_uuid_clone = task_uuid;

        tokio::spawn(async move {
            let memory = match MemoryStore::new() {
                Ok(m) => m,
                Err(_) => return,
            };

            if let Some(tid) = task_uuid_clone {
                let task_name = task_id_clone.as_deref().unwrap_or("unknown");
                memory_updater::maybe_update_task_memory(
                    tid, task_name, &user_msg, &ai_response, &memory, &api_client, &token_owned,
                )
                .await;
            }

            memory_updater::maybe_update_user_memory(
                &user_msg, &ai_response, &memory, &api_client, &token_owned,
            )
            .await;
        });
    }

    Ok(response)
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
        Arc::clone(&api_client),
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
        api_client,
        task_engine,
        agent_runtime,
    }
}
