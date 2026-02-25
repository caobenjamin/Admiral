//! Tauri IPC commands: create_task, list_tasks, get_task, run_dry_run, run_test_run, etc.

use crate::commands::auth::AuthContext;
use crate::network::api_client::ApiError;
use crate::task::engine::TaskEngine;
use admiral_types::task::{Task, TaskStatus, TaskType};
use std::sync::Arc;
use uuid::Uuid;

fn get_access_token(ctx: &AuthContext) -> Result<String, ApiError> {
    let cred = ctx.store.get().map_err(|e| ApiError::Credential(e.to_string()))?;
    cred.ok_or(ApiError::Unauthorized)
        .map(|c| c.access_token)
}

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

pub fn update_task_plan(
    engine: &TaskEngine,
    id: Uuid,
    plan: admiral_types::task::TaskPlan,
) -> Result<Task, String> {
    let mut task = engine
        .get_task(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "task not found".to_string())?;
    task.plan = Some(plan);
    task.updated_at = chrono::Utc::now();
    engine.save_task(&task).map_err(|e| e.to_string())?;
    Ok(task)
}

pub fn transition_task(
    engine: &TaskEngine,
    id: Uuid,
    new_status: TaskStatus,
) -> Result<Task, String> {
    let mut task = engine
        .get_task(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "task not found".to_string())?;
    engine
        .transition_to(&mut task, new_status)
        .map_err(|e| e.to_string())?;
    Ok(task)
}

pub async fn run_dry_run(
    engine: &TaskEngine,
    auth: &AuthContext,
    task_id: Uuid,
) -> Result<admiral_types::task::TaskRun, String> {
    let token = get_access_token(auth).map_err(|e| e.to_string())?;
    let task = engine
        .get_task(task_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "task not found".to_string())?;
    engine
        .run_dry_run(&task, Some(&token))
        .await
        .map_err(|e| e.to_string())
}

pub async fn run_test_run(
    engine: &TaskEngine,
    auth: &AuthContext,
    task_id: Uuid,
) -> Result<admiral_types::task::TaskRun, String> {
    let token = get_access_token(auth).map_err(|e| e.to_string())?;
    let task = engine
        .get_task(task_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "task not found".to_string())?;
    engine
        .run_test_run(&task, Some(&token))
        .await
        .map_err(|e| e.to_string())
}

pub async fn run_production(
    engine: &TaskEngine,
    auth: &AuthContext,
    task_id: Uuid,
) -> Result<admiral_types::task::TaskRun, String> {
    let token = get_access_token(auth).map_err(|e| e.to_string())?;
    let task = engine
        .get_task(task_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "task not found".to_string())?;
    engine
        .run_production(&task, Some(&token))
        .await
        .map_err(|e| e.to_string())
}
