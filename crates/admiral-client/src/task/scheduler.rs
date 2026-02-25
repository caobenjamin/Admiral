//! Cron and Heartbeat scheduler for scheduled tasks (local execution).

use admiral_types::task::Task;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::db;

/// Callback when a scheduled task should run: (task_id, task).
pub type ScheduleCallback = Arc<dyn Fn(Uuid, Task) + Send + Sync>;

/// Scheduler: loads scheduled/heartbeat tasks and invokes callback at interval or cron time.
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

    /// Start the scheduler loop (heartbeat every 60s; cron would be added via tokio-cron-scheduler).
    pub fn start(&self) {
        let callback = Arc::clone(&self.callback);
        let running = Arc::clone(&self.running);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let guard = running.read().await;
                if !*guard {
                    drop(guard);
                    continue;
                }
                drop(guard);
                let path = db::models::db_path();
                if let Ok(conn) = db::models::open_rw(&path) {
                        if let Ok(tasks) = db::models::list_heartbeat_tasks(&conn) {
                            let now = chrono::Utc::now();
                            for task in tasks {
                                let next = task.trigger.next_run_at;
                                if next.map(|t| t <= now).unwrap_or(true) {
                                    callback(task.id, task);
                                }
                            }
                        }
                }
            }
        });
    }

    pub async fn set_running(&self, value: bool) {
        let mut g = self.running.write().await;
        *g = value;
    }
}
