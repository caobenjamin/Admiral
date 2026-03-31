//! Tauri command bridge: managed state and invoke handlers.

use crate::commands::auth::{self, AuthContext};
use crate::config::{ClientConfig, LlmSelection};
use crate::credential::PlainFileCredentialStore;
use crate::db;
use crate::network::api_client::ApiClient;
use crate::agent::memory::MemoryStore;
use crate::agent::memory_updater;
use crate::agent::runtime::{AgentResponse, AgentRunState, AgentRunStatus, AgentRuntime, AgentStepState, AgentStepStatus, StreamEvent};
use crate::agent::context::{Message, Session};
use crate::skill::registry::SkillRegistry;
use crate::task::engine::TaskEngine;
use crate::task::scheduler::Scheduler;
use admiral_types::api::*;
use admiral_types::task::{Task, TaskCategory, TaskStatus, TaskType};
use std::sync::Arc;
use tauri::Emitter;
use tracing::{error, info};
use uuid::Uuid;

/// App state held by Tauri (thread-safe).
pub struct AppState {
    pub auth: Arc<AuthContext>,
    pub api_client: Arc<ApiClient>,
    pub task_engine: Arc<TaskEngine>,
    pub agent_runtime: Arc<tokio::sync::RwLock<Option<AgentRuntime>>>,
    pub scheduler: Arc<Scheduler>,
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

#[tauri::command]
pub async fn list_llm_models(
    state: tauri::State<'_, AppState>,
    provider: String,
) -> Result<admiral_types::api::LlmModelsResponse, String> {
    let token = state.auth.get_access_token().map_err(err_string)?;
    state
        .auth
        .api_client
        .llm_list_models(&token, &provider)
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

#[tauri::command]
pub fn get_llm_selection() -> LlmSelection {
    ClientConfig::from_env().llm_selection()
}

#[tauri::command]
pub fn set_llm_selection(provider: String, model: String) -> Result<(), String> {
    let mut config = ClientConfig::from_env();
    config.set_llm_selection(provider, model);
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
pub async fn task_delete(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    crate::commands::task::delete_task(state.task_engine.as_ref(), id)
}

#[tauri::command]
pub async fn task_get(state: tauri::State<'_, AppState>, id: String) -> Result<Option<Task>, String> {
    let id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    crate::commands::task::get_task(state.task_engine.as_ref(), id)
}

/// Create a progress channel and spawn a forwarder that emits `task-progress` events to the frontend.
fn spawn_progress_forwarder(
    app: &tauri::AppHandle,
) -> tokio::sync::mpsc::Sender<admiral_types::task::TaskProgress> {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<admiral_types::task::TaskProgress>(256);
    let app_clone = app.clone();
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let _ = app_clone.emit("task-progress", &event);
        }
    });
    tx
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
        None,
    )
    .await
}

#[tauri::command]
pub async fn task_run_test_run(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    task_id: String,
) -> Result<admiral_types::task::TaskRun, String> {
    let id = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    let progress_tx = spawn_progress_forwarder(&app);
    crate::commands::task::run_task(
        state.task_engine.as_ref(),
        state.auth.as_ref(),
        id,
        admiral_types::task::RunType::TestRun,
        Some(progress_tx),
    )
    .await
}

#[tauri::command]
pub async fn task_run_production(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    task_id: String,
) -> Result<admiral_types::task::TaskRun, String> {
    let id = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    let progress_tx = spawn_progress_forwarder(&app);
    crate::commands::task::run_task(
        state.task_engine.as_ref(),
        state.auth.as_ref(),
        id,
        admiral_types::task::RunType::Production,
        Some(progress_tx),
    )
    .await
}

#[tauri::command]
pub async fn task_resume(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
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
    let progress_tx = spawn_progress_forwarder(&app);
    crate::commands::task::resume_task(
        state.task_engine.as_ref(),
        state.auth.as_ref(),
        id,
        from_step,
        rt,
        Some(progress_tx),
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

// ---------- Task state transition ----------

#[tauri::command]
pub async fn task_transition(
    state: tauri::State<'_, AppState>,
    task_id: String,
    new_status: TaskStatus,
) -> Result<Task, String> {
    let id = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    let mut task = state
        .task_engine
        .get_task(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "task not found".to_string())?;
    state
        .task_engine
        .transition_to(&mut task, new_status)
        .map_err(|e| e.to_string())?;
    Ok(task)
}

// ---------- Conversation history for frontend display ----------

#[derive(serde::Serialize)]
pub struct ConversationMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<serde_json::Value>,
    pub name: Option<String>,
    pub timestamp: String,
}

#[derive(serde::Serialize)]
pub struct AgentRunDetail {
    pub run: AgentRunState,
    pub events: Vec<db::models::AgentEventRow>,
}

#[tauri::command]
pub async fn task_load_conversations(
    task_id: String,
) -> Result<Vec<ConversationMessage>, String> {
    let db_path = db::models::db_path();
    let conn = db::models::open_ro(&db_path).map_err(|e| e.to_string())?;
    let rows = db::models::load_conversations_by_task(&conn, &task_id).map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|r| ConversationMessage {
            id: r.id,
            role: r.role,
            content: r.content,
            tool_calls: r.tool_calls.and_then(|s| serde_json::from_str(&s).ok()),
            name: r.name,
            timestamp: r.timestamp,
        })
        .collect())
}

#[tauri::command]
pub async fn agent_get_latest_run(task_id: String) -> Result<Option<AgentRunDetail>, String> {
    let db_path = db::models::db_path();
    let conn = db::models::open_ro(&db_path).map_err(|e| e.to_string())?;
    let Some(run_row) = db::models::get_latest_agent_run_by_task(&conn, &task_id).map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let steps = db::models::list_agent_steps(&conn, &run_row.id).map_err(|e| e.to_string())?;
    let events = db::models::list_agent_events(&conn, &run_row.id).map_err(|e| e.to_string())?;
    Ok(Some(AgentRunDetail {
        run: map_agent_run_detail(run_row, steps)?,
        events,
    }))
}

#[tauri::command]
pub async fn agent_get_run(run_id: String) -> Result<Option<AgentRunDetail>, String> {
    let db_path = db::models::db_path();
    let conn = db::models::open_ro(&db_path).map_err(|e| e.to_string())?;
    let Some(run_row) = db::models::get_agent_run(&conn, &run_id).map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let steps = db::models::list_agent_steps(&conn, &run_row.id).map_err(|e| e.to_string())?;
    let events = db::models::list_agent_events(&conn, &run_row.id).map_err(|e| e.to_string())?;
    Ok(Some(AgentRunDetail {
        run: map_agent_run_detail(run_row, steps)?,
        events,
    }))
}

// ---------- Task spec ----------

#[tauri::command]
pub async fn task_load_spec(task_id: String) -> Result<String, String> {
    let id = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    let memory = MemoryStore::new().map_err(|e| e.to_string())?;
    memory.load_task_spec(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn task_list_run_reports(task_id: String) -> Result<Vec<String>, String> {
    let id = Uuid::parse_str(&task_id).map_err(|e| e.to_string())?;
    let memory = MemoryStore::new().map_err(|e| e.to_string())?;
    memory.list_run_reports(id).map_err(|e| e.to_string())
}

fn parse_agent_run_status(value: &str) -> Result<AgentRunStatus, String> {
    serde_json::from_str(&format!("\"{}\"", value)).map_err(|e| e.to_string())
}

fn parse_agent_step_status(value: &str) -> Result<AgentStepStatus, String> {
    serde_json::from_str(&format!("\"{}\"", value)).map_err(|e| e.to_string())
}

fn map_agent_run_detail(
    run_row: db::models::AgentRunRow,
    step_rows: Vec<db::models::AgentStepRow>,
) -> Result<AgentRunState, String> {
    let steps = step_rows
        .into_iter()
        .map(|row| {
            Ok(AgentStepState {
                step: row.step,
                title: row.title,
                status: parse_agent_step_status(&row.status)?,
                action_type: row.action_type,
                tool_name: row.tool_name,
                tool_arguments: row.tool_arguments.and_then(|s| serde_json::from_str(&s).ok()),
                tool_success: row.tool_success,
                tool_result: row.tool_result.and_then(|s| serde_json::from_str(&s).ok()),
                message: row.message,
                retry_count: row.retry_count,
                error: row.error,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(AgentRunState {
        run_id: run_row.id,
        task_id: run_row.task_id,
        session_id: run_row.session_id,
        status: parse_agent_run_status(&run_row.status)?,
        current_step: run_row.current_step,
        total_retries: run_row.total_retries,
        max_steps: run_row.max_steps,
        steps,
        final_message: run_row.final_message,
        waiting_for_user: run_row.waiting_for_user,
    })
}

// ---------- Agent chat helpers ----------

/// Load conversation history from DB and build a Session.
fn load_session(task_id: &Option<String>) -> (String, Option<Uuid>, Vec<Message>) {
    let task_uuid = task_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
    let session_id = task_uuid
        .map(|id| id.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let mut messages: Vec<Message> = Vec::new();
    let mut pending_tool_calls: std::collections::VecDeque<(String, Option<String>)> =
        std::collections::VecDeque::new();
    if let Some(tid) = task_id {
        let db_path = db::models::db_path();
        if let Ok(conn) = db::models::open_ro(&db_path) {
            if let Ok(rows) = db::models::load_conversations_by_task(&conn, tid) {
                for row in rows {
                    let tool_calls = row.tool_calls.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
                    let has_content = !row.content.trim().is_empty();
                    let has_tool_calls = tool_calls
                        .as_ref()
                        .and_then(|v| v.as_array())
                        .map(|arr| !arr.is_empty())
                        .unwrap_or(false);
                    if row.role == "assistant" && !has_content && !has_tool_calls {
                        continue;
                    }
                    if row.role == "assistant" {
                        if let Some(arr) = tool_calls.as_ref().and_then(|v| v.as_array()) {
                            for tc in arr {
                                let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                if id.is_empty() {
                                    continue;
                                }
                                let name = tc
                                    .get("function")
                                    .and_then(|v| v.get("name"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                                pending_tool_calls.push_back((id, name));
                            }
                        }
                    }

                    let mut name = row.name;
                    let mut tool_call_id = row.tool_call_id;
                    if row.role == "tool" && tool_call_id.is_none() {
                        if let Some((inferred_id, inferred_name)) = pending_tool_calls.pop_front() {
                            tool_call_id = Some(inferred_id);
                            if name.is_none() {
                                name = inferred_name;
                            }
                        }
                    } else if row.role == "tool" && tool_call_id.is_some() {
                        let _ = pending_tool_calls.pop_front();
                    }

                    if row.role == "tool" && tool_call_id.is_none() {
                        continue;
                    }

                    messages.push(Message {
                        role: row.role,
                        content: if row.content.is_empty() { None } else { Some(row.content) },
                        tool_calls,
                        name,
                        tool_call_id,
                    });
                }
            }
        }
    }
    (session_id, task_uuid, messages)
}

/// Persist newly added messages (after `msg_count_before`) to the conversation DB.
fn persist_new_messages(
    response: &AgentResponse,
    msg_count_before: usize,
    task_id: &Option<String>,
    session_id: &str,
) {
    let new_messages = &response.session.messages[msg_count_before..];
    if new_messages.is_empty() {
        return;
    }
    let db_path = db::models::db_path();
    if let Ok(conn) = db::models::open_rw(&db_path) {
        for msg in new_messages {
            if msg.role == "assistant"
                && msg.content.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true)
                && msg.tool_calls.as_ref().and_then(|v| v.as_array()).map(|arr| arr.is_empty()).unwrap_or(true)
            {
                continue;
            }
            let row = db::models::ConversationRow {
                id: Uuid::new_v4().to_string(),
                task_id: task_id.clone(),
                session_id: session_id.to_string(),
                role: msg.role.clone(),
                content: msg.content.clone().unwrap_or_default(),
                tool_calls: msg.tool_calls.as_ref().and_then(|v| serde_json::to_string(v).ok()),
                name: msg.name.clone(),
                tool_call_id: msg.tool_call_id.clone(),
                tool_result: None,
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            let _ = db::models::save_conversation_message(&conn, &row);
        }
    }
}

/// Spawn a non-blocking background task for memory extraction and spec update.
fn spawn_memory_update(
    api_client: &Arc<ApiClient>,
    task_engine: &Arc<TaskEngine>,
    token: &str,
    user_msg: &str,
    ai_response: &str,
    task_id: &Option<String>,
    task_uuid: Option<Uuid>,
) {
    let api_client = Arc::clone(api_client);
    let task_engine = Arc::clone(task_engine);
    let token_owned = token.to_string();
    let user_msg = user_msg.to_string();
    let ai_response = ai_response.to_string();
    let task_id_clone = task_id.clone();

    tokio::spawn(async move {
        let memory = match MemoryStore::new() {
            Ok(m) => m,
            Err(_) => return,
        };
        if let Some(tid) = task_uuid {
            let task_name = task_id_clone.as_deref().unwrap_or("unknown");
            memory_updater::maybe_update_task_memory(
                tid, task_name, &user_msg, &ai_response, &memory, &api_client, &token_owned,
            )
            .await;

            // Update task spec from conversation
            crate::agent::spec_updater::update_task_spec(
                tid, task_name, &user_msg, &ai_response,
                &task_engine, &api_client, &token_owned,
            )
            .await;
        }
        memory_updater::maybe_update_user_memory(
            &user_msg, &ai_response, &memory, &api_client, &token_owned,
        )
        .await;
    });
}

// ---------- Agent chat (streaming) ----------

#[tauri::command]
pub async fn agent_chat_stream(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    message: String,
    task_id: Option<String>,
) -> Result<AgentResponse, String> {
    info!(task_id = ?task_id, "agent_chat_stream invoked");
    let token = state.auth.get_access_token().map_err(|e| e.to_string())?;
    let (session_id, task_uuid, messages) = load_session(&task_id);
    let msg_count_before = messages.len();

    let session = Session { id: session_id.clone(), task_id: task_uuid, messages };

    let (tx, mut rx) = tokio::sync::mpsc::channel::<StreamEvent>(256);
    let app_clone = app.clone();
    let forward_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let _ = app_clone.emit("agent-stream", &event);
        }
    });

    let guard = state.agent_runtime.read().await;
    let runtime = guard.as_ref().ok_or("Agent not initialized")?;
    let response = match runtime
        .process_message_stream(session, &message, Some(&token), tx.clone())
        .await
    {
        Ok(response) => response,
        Err(e) => {
            let message = e.to_string();
            error!(error = %message, "agent_chat_stream failed");
            let _ = tx.send(StreamEvent::Error { message: message.clone() }).await;
            drop(tx);
            let _ = forward_task.await;
            return Err(message);
        }
    };
    drop(tx);
    let _ = forward_task.await;

    persist_new_messages(&response, msg_count_before, &task_id, &session_id);
    spawn_memory_update(&state.api_client, &state.task_engine, &token, &message, &response.content, &task_id, task_uuid);

    Ok(response)
}

// ---------- Agent chat (non-streaming, kept for backward compat) ----------

#[tauri::command]
pub async fn agent_chat(
    state: tauri::State<'_, AppState>,
    message: String,
    task_id: Option<String>,
) -> Result<AgentResponse, String> {
    let token = state.auth.get_access_token().map_err(|e| e.to_string())?;
    let (session_id, task_uuid, messages) = load_session(&task_id);
    let msg_count_before = messages.len();

    let session = Session { id: session_id.clone(), task_id: task_uuid, messages };

    let guard = state.agent_runtime.read().await;
    let runtime = guard.as_ref().ok_or("Agent not initialized")?;
    let response = runtime
        .process_message(session, &message, Some(&token))
        .await
        .map_err(|e| e.to_string())?;

    persist_new_messages(&response, msg_count_before, &task_id, &session_id);
    spawn_memory_update(&state.api_client, &state.task_engine, &token, &message, &response.content, &task_id, task_uuid);

    Ok(response)
}

/// Create AppState from config (used at startup).
pub fn create_app_state() -> AppState {
    let config = ClientConfig::from_env();
    let store = Arc::new(PlainFileCredentialStore::from_config(&config));
    let api_client = Arc::new(ApiClient::new(config.api_base_url));
    let auth = Arc::new(AuthContext::new(Arc::clone(&api_client), store));

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

    // Initialize and start the task scheduler
    let task_engine_for_scheduler = Arc::clone(&task_engine);
    let auth_for_scheduler = Arc::clone(&auth);
    let scheduler = Arc::new(Scheduler::new(Arc::new(move |task_id, _task| {
        let engine = Arc::clone(&task_engine_for_scheduler);
        let auth = Arc::clone(&auth_for_scheduler);
        tauri::async_runtime::spawn(async move {
            let mut task = match engine.get_task(task_id) {
                Ok(Some(t)) => t,
                _ => return,
            };
            let token = match auth.get_access_token() {
                Ok(token) => token,
                Err(_) => return,
            };
            let _ = engine.run(
                &mut task,
                admiral_types::task::RunType::Production,
                Some(&token),
                None,
            ).await;
        });
    })));
    AppState {
        auth,
        api_client,
        task_engine,
        agent_runtime,
        scheduler,
    }
}
