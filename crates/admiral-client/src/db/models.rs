//! SQLite data models and query helpers.

use admiral_types::task::{
    Task, TaskCategory, TaskRun, TaskStatus, TaskType,
    OperationPermission, PermissionScope, PendingApproval,
};
use chrono::{DateTime, Utc};
use rusqlite::params;
use std::path::PathBuf;
use uuid::Uuid;

/// Serialize a serde-based enum to its snake_case string representation.
fn enum_to_str<T: serde::Serialize>(val: &T) -> String {
    serde_json::to_string(val)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string()
}

fn enum_from_str<T: serde::de::DeserializeOwned>(s: &str, default: T) -> T {
    let quoted = format!("\"{}\"", s);
    serde_json::from_str::<T>(&quoted).unwrap_or(default)
}

/// Base directory for Admiral local data (e.g. ~/.local/share/admiral).
pub fn admiral_data_dir() -> PathBuf {
    let dir = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let admiral = dir.join("admiral");
    let _ = std::fs::create_dir_all(&admiral);
    admiral
}

/// Returns the path to the local SQLite database file (e.g. ~/.local/share/admiral/admiral.db).
pub fn db_path() -> PathBuf {
    admiral_data_dir().join("admiral.db")
}

/// Open a read-only connection (for queries that don't need write).
pub fn open_ro(path: &std::path::Path) -> anyhow::Result<rusqlite::Connection> {
    Ok(rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?)
}

/// Open a read-write connection.
pub fn open_rw(path: &std::path::Path) -> anyhow::Result<rusqlite::Connection> {
    Ok(rusqlite::Connection::open(path)?)
}

// ========================== Tasks ==========================

/// Insert or replace a task.
pub fn upsert_task(conn: &rusqlite::Connection, task: &Task) -> anyhow::Result<()> {
    let plan_json = task
        .plan
        .as_ref()
        .and_then(|p| serde_json::to_string(p).ok());
    let account_ids_json = serde_json::to_string(&task.account_ids).unwrap_or_else(|_| "[]".to_string());
    let next_run_at = task.trigger.next_run_at.map(|t| t.to_rfc3339());

    let task_type_str = enum_to_str(&task.task_type);
    let status_str = enum_to_str(&task.status);
    let category_str = enum_to_str(&task.task_category);
    conn.execute(
        r#"
        INSERT INTO tasks (id, name, description, task_type, status, plan, platform, account_ids,
            trigger_type, cron_expression, heartbeat_condition, next_run_at, created_at, updated_at, task_category)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
        ON CONFLICT(id) DO UPDATE SET
            name=excluded.name, description=excluded.description, task_type=excluded.task_type,
            status=excluded.status, plan=excluded.plan, platform=excluded.platform,
            account_ids=excluded.account_ids, trigger_type=excluded.trigger_type,
            cron_expression=excluded.cron_expression, heartbeat_condition=excluded.heartbeat_condition,
            next_run_at=excluded.next_run_at, updated_at=excluded.updated_at, task_category=excluded.task_category
        "#,
        params![
            task.id.to_string(),
            task.name,
            task.description,
            task_type_str,
            status_str,
            plan_json,
            task.platform,
            account_ids_json,
            task_type_str,
            task.trigger.cron_expression,
            task.trigger.heartbeat_condition,
            next_run_at,
            task.created_at.to_rfc3339(),
            task.updated_at.to_rfc3339(),
            category_str,
        ],
    )?;
    Ok(())
}

/// Load a task by id.
pub fn get_task(conn: &rusqlite::Connection, id: Uuid) -> anyhow::Result<Option<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, task_type, status, plan, platform, account_ids, trigger_type, cron_expression, heartbeat_condition, next_run_at, created_at, updated_at, task_category FROM tasks WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id.to_string()])?;
    let row = match rows.next()? {
        Some(r) => r,
        None => return Ok(None),
    };
    let task = row_to_task(row).map_err(|e| anyhow::anyhow!("db: {}", e))?;
    Ok(Some(task))
}

/// List all tasks, optionally filtered by status.
pub fn list_tasks(
    conn: &rusqlite::Connection,
    status_filter: Option<TaskStatus>,
) -> anyhow::Result<Vec<Task>> {
    let sql = "SELECT id, name, description, task_type, status, plan, platform, account_ids, trigger_type, cron_expression, heartbeat_condition, next_run_at, created_at, updated_at, task_category FROM tasks";
    let out: Vec<Task> = match status_filter {
        Some(s) => {
            let status_str = enum_to_str(&s);
            let mut stmt = conn.prepare(&format!("{} WHERE status = ?1", sql))?;
            stmt.query_map(params![status_str], row_to_task)?
                .filter_map(|r| r.ok())
                .collect()
        }
        None => {
            let mut stmt = conn.prepare(sql)?;
            stmt.query_map([], row_to_task)?
                .filter_map(|r| r.ok())
                .collect()
        }
    };
    Ok(out)
}

/// List tasks that are heartbeat type and scheduled (for heartbeat runner).
pub fn list_heartbeat_tasks(conn: &rusqlite::Connection) -> anyhow::Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, task_type, status, plan, platform, account_ids, trigger_type, cron_expression, heartbeat_condition, next_run_at, created_at, updated_at, task_category FROM tasks WHERE task_type = 'heartbeat' AND status = 'scheduled'",
    )?;
    let out: Vec<Task> = stmt
        .query_map([], row_to_task)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(out)
}

fn row_to_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
    let id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let description: Option<String> = row.get(2)?;
    let task_type_str: String = row.get(3)?;
    let status_str: String = row.get(4)?;
    let plan_json: Option<String> = row.get(5)?;
    let platform: String = row.get(6).unwrap_or_else(|_| "bilibili".to_string());
    let account_ids_json: String = row.get(7).unwrap_or_else(|_| "[]".to_string());
    let _trigger_type_str: String = row.get(8).unwrap_or_else(|_| "instant".to_string());
    let cron_expression: Option<String> = row.get(9)?;
    let heartbeat_condition: Option<String> = row.get(10)?;
    let next_run_at_str: Option<String> = row.get(11)?;
    let created_at_str: String = row.get(12)?;
    let updated_at_str: String = row.get(13)?;
    let category_str: String = row.get(14).unwrap_or_else(|_| "mixed".to_string());

    let task_type: TaskType = enum_from_str(&task_type_str, TaskType::Instant);
    let status: TaskStatus = enum_from_str(&status_str, TaskStatus::Draft);
    let task_category: TaskCategory = enum_from_str(&category_str, TaskCategory::Mixed);
    let plan = plan_json.and_then(|s| serde_json::from_str(&s).ok());
    let account_ids: Vec<i64> = serde_json::from_str(&account_ids_json).unwrap_or_default();
    let next_run_at = next_run_at_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|t| t.with_timezone(&Utc)));
    let created_at = DateTime::parse_from_rfc3339(&created_at_str).map(|t| t.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());
    let updated_at = DateTime::parse_from_rfc3339(&updated_at_str).map(|t| t.with_timezone(&Utc)).unwrap_or_else(|_| Utc::now());

    Ok(Task {
        id: Uuid::parse_str(&id).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?,
        name,
        description,
        task_type,
        task_category,
        status,
        plan,
        platform,
        account_ids,
        trigger: admiral_types::task::TaskTrigger {
            cron_expression,
            heartbeat_condition,
            next_run_at,
        },
        created_at,
        updated_at,
    })
}

// ========================== Task Runs ==========================

/// Insert a task run.
pub fn insert_task_run(conn: &rusqlite::Connection, run: &TaskRun) -> anyhow::Result<()> {
    let step_results_json = serde_json::to_string(&run.step_results).ok();
    let completed_steps_json = serde_json::to_string(&run.completed_steps).ok();
    conn.execute(
        r#"
        INSERT INTO task_runs (id, task_id, run_type, status, started_at, finished_at, step_results, error, resume_from_step, completed_steps)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#,
        params![
            run.id.to_string(),
            run.task_id.to_string(),
            enum_to_str(&run.run_type),
            enum_to_str(&run.status),
            run.started_at.to_rfc3339(),
            run.finished_at.as_ref().map(|t| t.to_rfc3339()),
            step_results_json,
            run.error,
            run.resume_from_step,
            completed_steps_json,
        ],
    )?;
    Ok(())
}

/// List runs for a task, ordered by started_at descending.
pub fn list_task_runs(conn: &rusqlite::Connection, task_id: Uuid) -> anyhow::Result<Vec<TaskRun>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, run_type, status, started_at, finished_at, step_results, error, resume_from_step, completed_steps FROM task_runs WHERE task_id = ?1 ORDER BY started_at DESC",
    )?;
    let out: Vec<TaskRun> = stmt
        .query_map(params![task_id.to_string()], row_to_task_run)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(out)
}

fn row_to_task_run(row: &rusqlite::Row) -> rusqlite::Result<TaskRun> {
    let id: String = row.get(0)?;
    let task_id: String = row.get(1)?;
    let run_type_str: String = row.get(2)?;
    let status_str: String = row.get(3)?;
    let started_at_str: String = row.get(4)?;
    let finished_at_str: Option<String> = row.get(5)?;
    let step_results_json: Option<String> = row.get(6)?;
    let error: Option<String> = row.get(7)?;
    let resume_from_step: Option<u32> = row.get(8)?;
    let completed_steps_json: Option<String> = row.get(9)?;

    use admiral_types::task::{RunType, RunStatus};
    let run_type: RunType = enum_from_str(&run_type_str, RunType::Production);
    let status: RunStatus = enum_from_str(&status_str, RunStatus::Failed);
    let started_at = DateTime::parse_from_rfc3339(&started_at_str)
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let finished_at = finished_at_str
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|t| t.with_timezone(&Utc)));
    let step_results = step_results_json
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let completed_steps = completed_steps_json
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    Ok(TaskRun {
        id: Uuid::parse_str(&id).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?,
        task_id: Uuid::parse_str(&task_id).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?,
        run_type,
        status,
        started_at,
        finished_at,
        step_results,
        error,
        resume_from_step,
        completed_steps,
    })
}

// ========================== Conversations ==========================

/// A single persisted conversation message.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ConversationRow {
    pub id: String,
    pub task_id: Option<String>,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<String>,
    pub tool_result: Option<String>,
    pub timestamp: String,
}

/// Save a single conversation message to DB.
pub fn save_conversation_message(conn: &rusqlite::Connection, msg: &ConversationRow) -> anyhow::Result<()> {
    conn.execute(
        r#"
        INSERT INTO conversations (id, task_id, session_id, role, content, tool_calls, tool_result, timestamp)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![
            msg.id,
            msg.task_id,
            msg.session_id,
            msg.role,
            msg.content,
            msg.tool_calls,
            msg.tool_result,
            msg.timestamp,
        ],
    )?;
    Ok(())
}

/// Load conversation messages for a task, ordered chronologically.
pub fn load_conversations_by_task(
    conn: &rusqlite::Connection,
    task_id: &str,
) -> anyhow::Result<Vec<ConversationRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, session_id, role, content, tool_calls, tool_result, timestamp FROM conversations WHERE task_id = ?1 ORDER BY timestamp ASC",
    )?;
    let out: Vec<ConversationRow> = stmt
        .query_map(params![task_id], |row| {
            Ok(ConversationRow {
                id: row.get(0)?,
                task_id: row.get(1)?,
                session_id: row.get(2)?,
                role: row.get(3)?,
                content: row.get(4)?,
                tool_calls: row.get(5)?,
                tool_result: row.get(6)?,
                timestamp: row.get(7)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(out)
}

// ========================== Operation Permissions ==========================

/// Insert or update an operation permission.
pub fn upsert_permission(conn: &rusqlite::Connection, perm: &OperationPermission) -> anyhow::Result<()> {
    conn.execute(
        r#"
        INSERT INTO operation_permissions (id, scope_type, scope_id, operation, auto_approve, granted_at, expires_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(id) DO UPDATE SET
            auto_approve=excluded.auto_approve, granted_at=excluded.granted_at, expires_at=excluded.expires_at
        "#,
        params![
            perm.id,
            enum_to_str(&perm.scope_type),
            perm.scope_id,
            perm.operation,
            perm.auto_approve as i32,
            perm.granted_at.to_rfc3339(),
            perm.expires_at.map(|t| t.to_rfc3339()),
        ],
    )?;
    Ok(())
}

/// Check if an operation is auto-approved. Checks task-scope first, then platform-scope.
pub fn check_permission(
    conn: &rusqlite::Connection,
    platform: &str,
    task_id: Option<&str>,
    operation: &str,
) -> anyhow::Result<bool> {
    let now = Utc::now().to_rfc3339();

    // Task-scope first
    if let Some(tid) = task_id {
        let approved: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM operation_permissions WHERE scope_type = 'task' AND scope_id = ?1 AND operation = ?2 AND auto_approve = 1 AND (expires_at IS NULL OR expires_at > ?3)",
                params![tid, operation, now],
                |row| row.get(0),
            )
            .unwrap_or(false);
        if approved {
            return Ok(true);
        }
    }

    // Platform-scope
    let approved: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM operation_permissions WHERE scope_type = 'platform' AND scope_id = ?1 AND operation = ?2 AND auto_approve = 1 AND (expires_at IS NULL OR expires_at > ?3)",
            params![platform, operation, now],
            |row| row.get(0),
        )
        .unwrap_or(false);
    Ok(approved)
}

/// List all operation permissions.
pub fn list_permissions(conn: &rusqlite::Connection) -> anyhow::Result<Vec<OperationPermission>> {
    let mut stmt = conn.prepare(
        "SELECT id, scope_type, scope_id, operation, auto_approve, granted_at, expires_at FROM operation_permissions ORDER BY granted_at DESC",
    )?;
    let out: Vec<OperationPermission> = stmt
        .query_map([], |row| {
            let scope_type_str: String = row.get(1)?;
            let granted_at_str: String = row.get(5)?;
            let expires_at_str: Option<String> = row.get(6)?;
            Ok(OperationPermission {
                id: row.get(0)?,
                scope_type: enum_from_str(&scope_type_str, PermissionScope::Platform),
                scope_id: row.get(2)?,
                operation: row.get(3)?,
                auto_approve: row.get::<_, i32>(4).unwrap_or(0) != 0,
                granted_at: DateTime::parse_from_rfc3339(&granted_at_str)
                    .map(|t| t.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                expires_at: expires_at_str
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|t| t.with_timezone(&Utc))),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(out)
}

/// Revoke (delete) an operation permission by id.
pub fn revoke_permission(conn: &rusqlite::Connection, id: &str) -> anyhow::Result<()> {
    conn.execute("DELETE FROM operation_permissions WHERE id = ?1", params![id])?;
    Ok(())
}

// ========================== Pending Approvals ==========================

/// Insert a pending approval.
pub fn insert_pending_approval(conn: &rusqlite::Connection, pa: &PendingApproval) -> anyhow::Result<()> {
    conn.execute(
        r#"
        INSERT INTO pending_approvals (id, task_id, run_id, step_seq, skill_id, operation, params_summary, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![
            Uuid::new_v4().to_string(),
            pa.task_id.to_string(),
            pa.run_id.to_string(),
            pa.step_seq,
            pa.skill_id,
            pa.operation,
            pa.params_summary,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Get the pending approval for a task (if any).
pub fn get_pending_approval(conn: &rusqlite::Connection, task_id: Uuid) -> anyhow::Result<Option<PendingApproval>> {
    let mut stmt = conn.prepare(
        "SELECT task_id, run_id, step_seq, skill_id, operation, params_summary FROM pending_approvals WHERE task_id = ?1 ORDER BY created_at DESC LIMIT 1",
    )?;
    let mut rows = stmt.query(params![task_id.to_string()])?;
    match rows.next()? {
        Some(row) => {
            let task_id_str: String = row.get(0)?;
            let run_id_str: String = row.get(1)?;
            Ok(Some(PendingApproval {
                task_id: Uuid::parse_str(&task_id_str).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?,
                run_id: Uuid::parse_str(&run_id_str).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?,
                step_seq: row.get(2)?,
                skill_id: row.get(3)?,
                operation: row.get(4)?,
                params_summary: row.get(5).unwrap_or_default(),
            }))
        }
        None => Ok(None),
    }
}

/// Delete pending approvals for a task.
pub fn clear_pending_approvals(conn: &rusqlite::Connection, task_id: Uuid) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM pending_approvals WHERE task_id = ?1",
        params![task_id.to_string()],
    )?;
    Ok(())
}
