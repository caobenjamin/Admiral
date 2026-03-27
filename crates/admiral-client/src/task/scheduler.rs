//! Cron and Heartbeat scheduler for scheduled tasks (local execution).

use admiral_types::task::Task;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db;

/// Callback invoked when a scheduled task should run: (task_id, task).
pub type ScheduleCallback = Arc<dyn Fn(Uuid, Task) + Send + Sync>;

/// Scheduler: loads scheduled heartbeat/cron tasks and invokes callback at the right time.
pub struct Scheduler {
    callback: ScheduleCallback,
    running: Arc<RwLock<bool>>,
}

impl Scheduler {
    pub fn new(callback: ScheduleCallback) -> Self {
        Self {
            callback,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the scheduler loop (checks every 30s for heartbeat and cron tasks).
    pub fn start(&self) {
        let callback = Arc::clone(&self.callback);
        let running = Arc::clone(&self.running);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            info!("task scheduler started (30s interval)");
            loop {
                interval.tick().await;
                let guard = running.read().await;
                if !*guard {
                    drop(guard);
                    continue;
                }
                drop(guard);

                let path = db::models::db_path();
                let conn = match db::models::open_rw(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        warn!(error = %e, "scheduler: failed to open DB");
                        continue;
                    }
                };

                // Heartbeat tasks: check next_run_at
                if let Ok(tasks) = db::models::list_heartbeat_tasks(&conn) {
                    let now = chrono::Utc::now();
                    for task in tasks {
                        let should_run = task.trigger.next_run_at
                            .map(|t| t <= now)
                            .unwrap_or(true);
                        if should_run {
                            info!(task_id = %task.id, "scheduler: triggering heartbeat task");
                            callback(task.id, task);
                        }
                    }
                }

                // Cron tasks: parse cron expression and check if it's time
                if let Ok(tasks) = db::models::list_cron_tasks(&conn) {
                    let now = chrono::Utc::now();
                    for task in tasks {
                        if let Some(ref cron_expr) = task.trigger.cron_expression {
                            match Self::should_cron_run(cron_expr, &now, task.trigger.next_run_at) {
                                Ok(true) => {
                                    info!(task_id = %task.id, cron = cron_expr, "scheduler: triggering cron task");
                                    callback(task.id, task);
                                }
                                Ok(false) => {}
                                Err(e) => {
                                    warn!(task_id = %task.id, error = %e, "scheduler: invalid cron expression");
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    /// Check if a cron task should run now.
    /// Currently relies on pre-computed `next_run_at`; cron expression parsing via
    /// `tokio-cron-scheduler` is planned but not yet wired.
    fn should_cron_run(
        cron_expr: &str,
        now: &chrono::DateTime<chrono::Utc>,
        next_run_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<bool, String> {
        if let Some(next) = next_run_at {
            return Ok(next <= *now);
        }
        warn!(cron_expr, "cron expression parsing not yet implemented, task will not fire without next_run_at");
        Ok(false)
    }

    pub async fn set_running(&self, value: bool) {
        let mut g = self.running.write().await;
        *g = value;
        info!(running = value, "scheduler running state updated");
    }
}
