use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct TaskCapabilitiesResponse {
    pub mode: &'static str,
    pub server_managed_runs: bool,
    pub notes: &'static str,
}

pub async fn capabilities() -> Json<TaskCapabilitiesResponse> {
    Json(TaskCapabilitiesResponse {
        mode: "client_local",
        server_managed_runs: false,
        notes: "Tasks are currently orchestrated in the desktop client. The server only provides tool backends such as llm, oauth, mapi, and knowledge.",
    })
}
