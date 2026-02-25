//! Knowledge query API: POST /api/v1/knowledge/query

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::middleware::auth_guard::AuthUser;
use crate::service::knowledge_service;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct KnowledgeQueryRequest {
    pub query: String,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

fn default_top_k() -> usize {
    5
}

#[derive(Debug, Serialize)]
pub struct KnowledgeQueryResponse {
    pub snippets: Vec<knowledge_service::KnowledgeSnippet>,
}

pub async fn query(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<KnowledgeQueryRequest>,
) -> Result<Json<KnowledgeQueryResponse>> {
    let snippets = knowledge_service::query(&state.db, &req.query, req.top_k).await?;
    Ok(Json(KnowledgeQueryResponse { snippets }))
}
