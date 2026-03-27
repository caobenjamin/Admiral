//! Tauri IPC commands: create_task, list_tasks, get_task, run_dry_run, run_test_run, etc.

use crate::commands::auth::AuthContext;
use crate::task::engine::TaskEngine;
use admiral_types::task::{Task, TaskStatus, TaskType};
use uuid::Uuid;

pub fn create_task(
    engine: &TaskEngine,
    name: String,
    description: Option<String>,
    task_type: TaskType,
) -> Result<Task, String> {
    engine
        .create_task(name, description, task_type)
        .map_err(|e| e.to_string())
}

pub fn list_tasks(
    engine: &TaskEngine,
    status_filter: Option<TaskStatus>,
) -> Result<Vec<Task>, String> {
    engine
        .list_tasks(status_filter)
        .map_err(|e| e.to_string())
}

pub fn get_task(engine: &TaskEngine, id: Uuid) -> Result<Option<Task>, String> {
    engine.get_task(id).map_err(|e| e.to_string())
}

pub async fn run_task(
    engine: &TaskEngine,
    auth: &AuthContext,
    task_id: Uuid,
    run_type: admiral_types::task::RunType,
) -> Result<admiral_types::task::TaskRun, String> {
    let token = auth.get_access_token().map_err(|e| e.to_string())?;
    let task = engine
        .get_task(task_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "task not found".to_string())?;
    engine
        .run(&task, run_type, Some(&token))
        .await
        .map_err(|e| e.to_string())
}
