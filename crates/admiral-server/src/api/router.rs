//! Axum router: /health, /api/v1/auth, /api/v1/oauth/bilibili, etc.

use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;

use crate::api::{admin, auth, knowledge, llm, mapi, oauth, task};
use crate::middleware::request_log;
use crate::state::AppState;

pub fn app(state: AppState) -> Router {
    let auth_routes = Router::new()
        .route("/register", post(auth::register))
        .route("/login", post(auth::login))
        .route("/refresh", post(auth::refresh))
        .route("/me", get(auth::me))
        .route("/license", get(auth::license));

    let oauth_bilibili_routes = Router::new()
        .route("/initiate", post(oauth::initiate))
        .route("/callback", get(oauth::callback))
        .route("/status", get(oauth::status))
        .route("/subjects", get(oauth::subjects_list))
        .route("/bindings", get(oauth::bindings_list))
        .route("/accounts", get(oauth::accounts_list))
        .route("/bind-accounts", post(oauth::bind_accounts))
        .route("/bindings/{account_id}", axum::routing::delete(oauth::unbind_account))
        .route("/sessions/{id}", axum::routing::delete(oauth::delete_session));

    Router::new()
        .route("/health", get(health))
        .nest("/api/v1/auth", auth_routes)
        .nest("/api/v1/oauth/bilibili", oauth_bilibili_routes)
        .route("/api/v1/mapi/proxy", axum::routing::post(mapi::proxy))
        .route("/api/v1/llm/responses", axum::routing::post(llm::responses))
        .route("/api/v1/llm/models", get(llm::list_models))
        .route("/api/v1/knowledge/query", axum::routing::post(knowledge::query))
        .route("/api/v1/tasks/capabilities", get(task::capabilities))
        .route("/api/v1/admin/invites", axum::routing::get(admin::list_invites).post(admin::create_invite))
        .route("/api/v1/admin/users/{id}/extend", axum::routing::post(admin::extend_user))
        .route("/api/v1/admin/knowledge/ingest", axum::routing::post(admin::knowledge_ingest))
        .layer(CorsLayer::permissive())
        .layer(from_fn_with_state(state.clone(), request_log::request_log_layer))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
