//! Tauri app entry point.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use admiral_client_lib::tauri_bridge;
use admiral_client_lib::db;

fn main() {
    if let Err(e) = db::migrations::run_migrations(&db::models::db_path()) {
        tracing::warn!(error = %e, "SQLite migration failed");
    }
    let state = tauri_bridge::create_app_state();
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state)
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
            tauri_bridge::agent_chat,
        ])
        .run(tauri::generate_context!())
        .expect("error running Admiral");
}
