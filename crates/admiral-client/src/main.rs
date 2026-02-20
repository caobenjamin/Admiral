//! Tauri app entry point.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use admiral_client_lib::tauri_bridge;

fn main() {
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
            tauri_bridge::oauth_accounts_list,
            tauri_bridge::oauth_bind_accounts,
            tauri_bridge::oauth_delete_session,
            tauri_bridge::mapi_proxy,
            tauri_bridge::llm_chat,
            tauri_bridge::open_url,
            tauri_bridge::get_config_path,
            tauri_bridge::get_server_url,
            tauri_bridge::set_server_url,
        ])
        .run(tauri::generate_context!())
        .expect("error running Admiral");
}
