//! Tauri app entry point.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use admiral_client_lib::tauri_bridge;
use admiral_client_lib::db;
use tauri::Manager;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    let level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info,admiral_client=debug".to_string());
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(level))
        .with(tracing_subscriber::fmt::layer().pretty())
        .try_init();

    if let Err(e) = db::migrations::run_migrations(&db::models::db_path()) {
        tracing::warn!(error = %e, "SQLite migration failed");
    }
    let state = tauri_bridge::create_app_state();
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .setup(|app| {
            app.state::<tauri_bridge::AppState>().scheduler.start();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            tauri_bridge::auth_register,
            tauri_bridge::auth_login,
            tauri_bridge::auth_refresh,
            tauri_bridge::auth_logout,
            tauri_bridge::auth_me,
            tauri_bridge::auth_license,
            tauri_bridge::oauth_initiate,
            tauri_bridge::oauth_status,
            tauri_bridge::oauth_bindings_list,
            tauri_bridge::oauth_subjects_list,
            tauri_bridge::oauth_accounts_list,
            tauri_bridge::oauth_bind_accounts,
            tauri_bridge::oauth_delete_session,
            tauri_bridge::oauth_unbind_account,
            tauri_bridge::mapi_proxy,
            tauri_bridge::llm_chat,
            tauri_bridge::open_url,
            tauri_bridge::get_config_path,
            tauri_bridge::get_server_url,
            tauri_bridge::set_server_url,
            tauri_bridge::task_create,
            tauri_bridge::task_list,
            tauri_bridge::task_delete,
            tauri_bridge::task_get,
            tauri_bridge::task_run_dry_run,
            tauri_bridge::task_run_test_run,
            tauri_bridge::task_run_production,
            tauri_bridge::task_resume,
            tauri_bridge::task_approve_operation,
            tauri_bridge::task_reject_operation,
            tauri_bridge::task_get_pending_approval,
            tauri_bridge::permission_list,
            tauri_bridge::permission_revoke,
            tauri_bridge::task_transition,
            tauri_bridge::task_load_conversations,
            tauri_bridge::task_load_spec,
            tauri_bridge::task_list_run_reports,
            tauri_bridge::agent_get_latest_run,
            tauri_bridge::agent_get_run,
            tauri_bridge::agent_chat,
            tauri_bridge::agent_chat_stream,
        ])
        .run(tauri::generate_context!())
        .expect("error running Admiral");
}
