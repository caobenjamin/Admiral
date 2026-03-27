//! SQLite data models and query helpers.

use admiral_types::task::{Task, TaskRun, TaskStatus, TaskType};
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
    conn.execute(
        r#"
        INSERT INTO tasks (id, name, description, task_type, status, plan, platform, account_ids,
            trigger_type, cron_expression, heartbeat_condition, next_run_at, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        ON CONFLICT(id) DO UPDATE SET
            name=excluded.name, description=excluded.description, task_type=excluded.task_type,
            status=excluded.status, plan=excluded.plan, platform=excluded.platform,
            account_ids=excluded.account_ids, trigger_type=excluded.trigger_type,
            cron_expression=excluded.cron_expression, heartbeat_condition=excluded.heartbeat_condition,
            next_run_at=excluded.next_run_at, updated_at=excluded.updated_at
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
        ],
    )?;
    Ok(())
}

/// Load a task by id.
pub fn get_task(conn: &rusqlite::Connection, id: Uuid) -> anyhow::Result<Option<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, task_type, status, plan, platform, account_ids, trigger_type, cron_expression, heartbeat_condition, next_run_at, created_at, updated_at FROM tasks WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id.to_string()])?;
    let row = match rows.next()? {
        Some(r) => r,
        None => return Ok(None),
    };
    let task = row_to_task(&row).map_err(|e| anyhow::anyhow!("db: {}", e))?;
    Ok(Some(task))
}

/// List all tasks, optionally filtered by status.
pub fn list_tasks(
    conn: &rusqlite::Connection,
    status_filter: Option<TaskStatus>,
) -> anyhow::Result<Vec<Task>> {
    let sql = "SELECT id, name, description, task_type, status, plan, platform, account_ids, trigger_type, cron_expression, heartbeat_condition, next_run_at, created_at, updated_at FROM tasks";
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

/// List tasks that are heartbeat type and deployed (for heartbeat runner).
pub fn list_heartbeat_tasks(conn: &rusqlite::Connection) -> anyhow::Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, task_type, status, plan, platform, account_ids, trigger_type, cron_expression, heartbeat_condition, next_run_at, created_at, updated_at FROM tasks WHERE task_type = 'heartbeat' AND status IN ('scheduled', 'deployed')",
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

    let task_type = serde_json::from_str::<TaskType>(&format!("\"{}\"", task_type_str)).unwrap_or(TaskType::Instant);
    let status = serde_json::from_str::<TaskStatus>(&format!("\"{}\"", status_str)).unwrap_or(TaskStatus::Draft);
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

/// Insert a task run.
pub fn insert_task_run(conn: &rusqlite::Connection, run: &TaskRun) -> anyhow::Result<()> {
    let step_results_json = serde_json::to_string(&run.step_results).ok();
    conn.execute(
        r#"
        INSERT INTO task_runs (id, task_id, run_type, status, started_at, finished_at, step_results, error)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
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
        ],
    )?;
    Ok(())
}