//! SQLite schema migrations: tasks, task_runs, logs, memory_entries, conversations, config,
//! operation_permissions.

use rusqlite::Connection;
use std::path::Path;
use tracing::info;

const SCHEMA_VERSION: u32 = 2;

/// Run all migrations. Creates DB file and tables if they don't exist.
pub fn run_migrations(db_path: &Path) -> anyhow::Result<()> {
    let conn = Connection::open(db_path)?;
    let current = conn.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))? as u32;
    if current >= SCHEMA_VERSION {
        info!(current, SCHEMA_VERSION, "SQLite schema already up to date");
        return Ok(());
    }
    if current == 0 {
        create_schema_v1(&conn)?;
    }
    if current <= 1 {
        migrate_v1_to_v2(&conn)?;
    }
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

fn create_schema_v1(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        r#"
        -- Tasks
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            task_type TEXT NOT NULL DEFAULT 'instant',
            status TEXT NOT NULL DEFAULT 'draft',
            plan TEXT,
            platform TEXT DEFAULT 'bilibili',
            account_ids TEXT,
            trigger_type TEXT DEFAULT 'instant',
            cron_expression TEXT,
            heartbeat_condition TEXT,
            next_run_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        -- Task runs
        CREATE TABLE IF NOT EXISTS task_runs (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            run_type TEXT NOT NULL,
            status TEXT NOT NULL,
            started_at TEXT NOT NULL,
            finished_at TEXT,
            step_results TEXT,
            error TEXT,
            FOREIGN KEY (task_id) REFERENCES tasks(id)
        );
        CREATE INDEX IF NOT EXISTS idx_task_runs_task_id ON task_runs(task_id);

        -- Conversations (agent chat history)
        CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            task_id TEXT,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            tool_calls TEXT,
            tool_result TEXT,
            timestamp TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_conversations_task_id ON conversations(task_id);
        CREATE INDEX IF NOT EXISTS idx_conversations_session_ts ON conversations(session_id, timestamp);

        -- Memory index (for FTS5 / hybrid search - content stored in files, this is index)
        CREATE TABLE IF NOT EXISTS memory_entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            key TEXT NOT NULL UNIQUE,
            content TEXT NOT NULL,
            category TEXT,
            source_file TEXT,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_memory_entries_category ON memory_entries(category);

        -- FTS5 virtual table for full-text search on memory
        CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
            key,
            content,
            category,
            content='memory_entries',
            content_rowid='id'
        );
        CREATE TRIGGER IF NOT EXISTS memory_fts_insert AFTER INSERT ON memory_entries BEGIN
            INSERT INTO memory_fts(rowid, key, content, category) VALUES (new.id, new.key, new.content, new.category);
        END;
        CREATE TRIGGER IF NOT EXISTS memory_fts_delete AFTER DELETE ON memory_entries BEGIN
            INSERT INTO memory_fts(memory_fts, rowid, key, content, category) VALUES ('delete', old.id, old.key, old.content, old.category);
        END;
        CREATE TRIGGER IF NOT EXISTS memory_fts_update AFTER UPDATE ON memory_entries BEGIN
            INSERT INTO memory_fts(memory_fts, rowid, key, content, category) VALUES ('delete', old.id, old.key, old.content, old.category);
            INSERT INTO memory_fts(rowid, key, content, category) VALUES (new.id, new.key, new.content, new.category);
        END;

        -- Logs (execution logs, synced to server optionally)
        CREATE TABLE IF NOT EXISTS logs (
            id TEXT PRIMARY KEY,
            request_id TEXT,
            task_id TEXT,
            task_run_id TEXT,
            skill_id TEXT,
            level TEXT NOT NULL,
            category TEXT,
            message TEXT NOT NULL,
            details TEXT,
            timestamp TEXT NOT NULL,
            duration_ms INTEGER,
            synced INTEGER DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_logs_task_id ON logs(task_id);
        CREATE INDEX IF NOT EXISTS idx_logs_synced ON logs(synced, timestamp);

        -- Config (key-value)
        CREATE TABLE IF NOT EXISTS config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    info!("SQLite schema v1 created");
    Ok(())
}

fn migrate_v1_to_v2(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        r#"
        -- Operation permissions (auto-approve whitelist for write operations)
        CREATE TABLE IF NOT EXISTS operation_permissions (
            id           TEXT PRIMARY KEY,
            scope_type   TEXT NOT NULL,
            scope_id     TEXT NOT NULL,
            operation    TEXT NOT NULL,
            auto_approve INTEGER NOT NULL DEFAULT 0,
            granted_at   TEXT NOT NULL,
            expires_at   TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_op_perm_scope ON operation_permissions(scope_type, scope_id, operation);

        -- Pending approvals (tracks which step is waiting for user confirmation)
        CREATE TABLE IF NOT EXISTS pending_approvals (
            id        TEXT PRIMARY KEY,
            task_id   TEXT NOT NULL,
            run_id    TEXT NOT NULL,
            step_seq  INTEGER NOT NULL,
            skill_id  TEXT NOT NULL,
            operation TEXT NOT NULL,
            params_summary TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (task_id) REFERENCES tasks(id)
        );
        CREATE INDEX IF NOT EXISTS idx_pending_approvals_task ON pending_approvals(task_id);
        "#,
    )?;

    // Add new columns to tasks (SQLite ALTER TABLE only supports ADD COLUMN one at a time)
    let add_cols = [
        "ALTER TABLE tasks ADD COLUMN task_category TEXT DEFAULT 'mixed'",
        "ALTER TABLE tasks ADD COLUMN last_run_id TEXT",
    ];
    for sql in &add_cols {
        if let Err(e) = conn.execute(sql, []) {
            if !e.to_string().contains("duplicate column") {
                return Err(e.into());
            }
        }
    }

    let add_run_cols = [
        "ALTER TABLE task_runs ADD COLUMN resume_from_step INTEGER",
        "ALTER TABLE task_runs ADD COLUMN completed_steps TEXT",
    ];
    for sql in &add_run_cols {
        if let Err(e) = conn.execute(sql, []) {
            if !e.to_string().contains("duplicate column") {
                return Err(e.into());
            }
        }
    }

    info!("SQLite schema migrated to v2");
    Ok(())
}
