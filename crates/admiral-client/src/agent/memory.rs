//! Long-term memory store: user preferences, operation patterns.
//! Markdown files under ~/.admiral/memory/ with optional SQLite FTS5 index for search.

use crate::db;
use std::path::PathBuf;
use uuid::Uuid;

/// Base paths for memory files (~/.admiral/memory/).
fn memory_base_dir() -> PathBuf {
    db::models::admiral_data_dir().join("memory")
}

/// Path to USER_MEMORY.md.
fn user_memory_path() -> PathBuf {
    memory_base_dir().join("USER_MEMORY.md")
}

/// Path to daily note for date (e.g. 2026-02-24.md).
fn daily_path(date: &str) -> PathBuf {
    memory_base_dir().join("daily").join(format!("{}.md", date))
}

/// Path to task memory directory and file for a task.
fn task_memory_dir(task_id: Uuid) -> PathBuf {
    memory_base_dir().join("tasks").join(task_id.to_string())
}

fn task_memory_path(task_id: Uuid) -> PathBuf {
    task_memory_dir(task_id).join("TASK_MEMORY.md")
}

fn task_spec_path(task_id: Uuid) -> PathBuf {
    task_memory_dir(task_id).join("TASK_SPEC.md")
}

fn task_run_report_dir(task_id: Uuid) -> PathBuf {
    task_memory_dir(task_id).join("runs")
}

/// Ensures memory directory structure exists.
pub fn ensure_memory_dirs() -> std::io::Result<()> {
    let base = memory_base_dir();
    std::fs::create_dir_all(base.join("daily"))?;
    std::fs::create_dir_all(base.join("tasks"))?;
    Ok(())
}

/// Memory store: read/write Markdown files and optional FTS search.
#[derive(Clone)]
pub struct MemoryStore {
    /// Path to SQLite DB for FTS5 (same as main app DB).
    db_path: PathBuf,
}

impl MemoryStore {
    pub fn new() -> std::io::Result<Self> {
        ensure_memory_dirs()?;
        Ok(Self {
            db_path: db::models::db_path(),
        })
    }

    /// Load full user-level memory (USER_MEMORY.md). Returns empty string if file missing.
    pub fn load_user_memory(&self) -> std::io::Result<String> {
        let p = user_memory_path();
        if p.is_file() {
            std::fs::read_to_string(p)
        } else {
            Ok(String::new())
        }
    }

    /// Load task-level memory for a task. Returns empty string if missing.
    pub fn load_task_memory(&self, task_id: Uuid) -> std::io::Result<String> {
        let p = task_memory_path(task_id);
        if p.is_file() {
            std::fs::read_to_string(p)
        } else {
            Ok(String::new())
        }
    }

    /// Load daily note for a date (YYYY-MM-DD). Returns empty string if missing.
    pub fn load_daily_memory(&self, date: &str) -> std::io::Result<String> {
        let p = daily_path(date);
        if p.is_file() {
            std::fs::read_to_string(p)
        } else {
            Ok(String::new())
        }
    }

    /// Write user memory file (full replace).
    pub fn write_user_memory(&self, content: &str) -> std::io::Result<()> {
        ensure_memory_dirs()?;
        std::fs::write(user_memory_path(), content)
    }

    /// Append a line to user memory (e.g. new preference). Creates file if missing.
    pub fn append_user_memory_line(&self, section: &str, line: &str) -> std::io::Result<()> {
        ensure_memory_dirs()?;
        let path = user_memory_path();
        let mut content = if path.is_file() {
            std::fs::read_to_string(&path)?
        } else {
            "# User Memory\n\n".to_string()
        };
        let section_header = format!("## {}", section);
        if !content.contains(&section_header) {
            content.push_str(&format!("\n{}\n", section_header));
        }
        content.push_str(&format!("- {}\n", line));
        std::fs::write(path, content)
    }

    /// Write task memory file (full replace). Creates task dir if needed.
    pub fn write_task_memory(&self, task_id: Uuid, content: &str) -> std::io::Result<()> {
        let dir = task_memory_dir(task_id);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(task_memory_path(task_id), content)
    }

    /// Load task spec (TASK_SPEC.md) for a task. Returns empty string if missing.
    pub fn load_task_spec(&self, task_id: Uuid) -> std::io::Result<String> {
        let p = task_spec_path(task_id);
        if p.is_file() {
            std::fs::read_to_string(p)
        } else {
            Ok(String::new())
        }
    }

    /// Write task spec file (full replace). Creates task dir if needed.
    pub fn write_task_spec(&self, task_id: Uuid, content: &str) -> std::io::Result<()> {
        let dir = task_memory_dir(task_id);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(task_spec_path(task_id), content)
    }

    /// Save a run report as `runs/{run_id}.md` under the task directory.
    pub fn save_run_report(&self, task_id: Uuid, run_id: &str, content: &str) -> std::io::Result<()> {
        let dir = task_run_report_dir(task_id);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(format!("{}.md", run_id)), content)
    }

    /// Load all run report filenames for a task.
    pub fn list_run_reports(&self, task_id: Uuid) -> std::io::Result<Vec<String>> {
        let dir = task_run_report_dir(task_id);
        if !dir.is_dir() {
            return Ok(Vec::new());
        }
        let mut reports = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            if let Ok(e) = entry {
                if let Some(name) = e.file_name().to_str() {
                    if name.ends_with(".md") {
                        reports.push(name.to_string());
                    }
                }
            }
        }
        reports.sort();
        Ok(reports)
    }

    /// Append to daily note for today.
    pub fn append_daily(&self, line: &str) -> std::io::Result<()> {
        ensure_memory_dirs()?;
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let path = daily_path(&date);
        let mut content = if path.is_file() {
            std::fs::read_to_string(&path)?
        } else {
            format!("# Daily note {}\n\n", date)
        };
        content.push_str(&format!("- {}\n", line));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
    }

    /// Keyword search across memory (uses SQLite FTS5 if memory_entries is populated).
    /// Returns empty list if DB/tables not ready or query fails.
    pub fn search(&self, query: &str) -> Vec<String> {
        let conn = match rusqlite::Connection::open(&self.db_path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut stmt = match conn.prepare("SELECT content FROM memory_fts WHERE memory_fts MATCH ?1 LIMIT 20") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = match stmt.query_map([query], |row| row.get::<_, String>(0)) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        let mut out = Vec::new();
        for r in rows {
            if let Ok(s) = r {
                out.push(s);
            }
        }
        out
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            tracing::warn!(error = %e, "MemoryStore init failed, using in-memory only");
            Self {
                db_path: std::path::PathBuf::from(":memory:"),
            }
        })
    }
}
