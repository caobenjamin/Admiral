//! Tauri IPC commands: task CRUD, run, approve, resume, permissions.

use crate::commands::auth::AuthContext;
use crate::db;
use crate::task::engine::TaskEngine;
use admiral_types::task::{
    Task, TaskCategory, TaskStatus, TaskType, RunType,
    OperationPermission, PendingApproval, PermissionScope,
};
use chrono::Utc;
use uuid::Uuid;

pub fn create_task(
    engine: &TaskEngine,
    name: String,
    description: Option<String>,
    task_type: TaskType,
    task_category: TaskCategory,
) -> Result<Task, String> {
    engine
        .create_task(name, description, task_type, task_category)
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
    run_type: RunType,
) -> Result<admiral_types::task::TaskRun, String> {
    let token = auth.get_access_token().map_err(|e| e.to_string())?;
    let mut task = engine
        .get_task(task_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "task not found".to_string())?;
    engine
        .run(&mut task, run_type, Some(&token))
        .await
        .map_err(|e| e.to_string())
}

/// Resume a partially succeeded task from a given step.
pub async fn resume_task(
    engine: &TaskEngine,
    auth: &AuthContext,
    task_id: Uuid,
    from_step: Option<u32>,
    run_type: RunType,
) -> Result<admiral_types::task::TaskRun, String> {
    let token = auth.get_access_token().map_err(|e| e.to_string())?;
    let mut task = engine
        .get_task(task_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "task not found".to_string())?;
    engine
        .resume_run(&mut task, from_step, run_type, Some(&token))
        .await
        .map_err(|e| e.to_string())
}

/// Approve a pending operation: clear the approval, optionally grant a whitelist entry, then resume.
pub async fn approve_operation(
    engine: &TaskEngine,
    auth: &AuthContext,
    task_id: Uuid,
    auto_approve_future: bool,
) -> Result<admiral_types::task::TaskRun, String> {
    let db_path = db::models::db_path();
    let conn = db::models::open_rw(&db_path).map_err(|e| e.to_string())?;

    let pending = db::models::get_pending_approval(&conn, task_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no pending approval found".to_string())?;

    if auto_approve_future {
        let task = engine
            .get_task(task_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "task not found".to_string())?;

        let perm = OperationPermission {
            id: Uuid::new_v4().to_string(),
            scope_type: PermissionScope::Task,
            scope_id: task.id.to_string(),
            operation: pending.operation.clone(),
            auto_approve: true,
            granted_at: Utc::now(),
            expires_at: None,
        };
        db::models::upsert_permission(&conn, &perm).map_err(|e| e.to_string())?;
    }

    db::models::clear_pending_approvals(&conn, task_id).map_err(|e| e.to_string())?;

    // Resume execution from the step that was waiting
    resume_task(engine, auth, task_id, Some(pending.step_seq), RunType::Production).await
}

/// Reject a pending operation → cancel the task.
pub fn reject_operation(engine: &TaskEngine, task_id: Uuid) -> Result<Task, String> {
    let db_path = db::models::db_path();
    let conn = db::models::open_rw(&db_path).map_err(|e| e.to_string())?;
    db::models::clear_pending_approvals(&conn, task_id).map_err(|e| e.to_string())?;

    let mut task = engine
        .get_task(task_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "task not found".to_string())?;
    engine
        .transition_to(&mut task, TaskStatus::Cancelled)
        .map_err(|e| e.to_string())?;
    Ok(task)
}

/// Get the pending approval for a task (if any).
pub fn get_pending_approval(task_id: Uuid) -> Result<Option<PendingApproval>, String> {
    let db_path = db::models::db_path();
    let conn = db::models::open_ro(&db_path).map_err(|e| e.to_string())?;
    db::models::get_pending_approval(&conn, task_id).map_err(|e| e.to_string())
}

/// List all operation permissions.
pub fn list_permissions() -> Result<Vec<OperationPermission>, String> {
    let db_path = db::models::db_path();
    let conn = db::models::open_ro(&db_path).map_err(|e| e.to_string())?;
    db::models::list_permissions(&conn).map_err(|e| e.to_string())
}

/// Revoke an operation permission by id.
pub fn revoke_permission(id: String) -> Result<(), String> {
    let db_path = db::models::db_path();
    let conn = db::models::open_rw(&db_path).map_err(|e| e.to_string())?;
    db::models::revoke_permission(&conn, &id).map_err(|e| e.to_string())
}
