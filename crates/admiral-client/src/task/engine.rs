//! Task engine: orchestrates task lifecycle, state transitions, persistence.

use admiral_types::task::{
    Task, TaskCategory, TaskStatus, TaskType, TaskTrigger, TaskProgress,
    RunType, RunStatus,
};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::db;
use crate::task::state_machine;
use crate::task::executor;

/// Task engine: load/save tasks, run state machine, trigger execution.
pub struct TaskEngine {
    skill_registry: Arc<crate::skill::registry::SkillRegistry>,
    skill_executor: Arc<crate::skill::executor::SkillExecutor>,
}

impl TaskEngine {
    pub fn new(
        skill_registry: Arc<crate::skill::registry::SkillRegistry>,
        skill_executor: Arc<crate::skill::executor::SkillExecutor>,
    ) -> Self {
        Self {
            skill_registry,
            skill_executor,
        }
    }

    pub fn create_task(
        &self,
        name: String,
        description: Option<String>,
        task_type: TaskType,
        task_category: TaskCategory,
    ) -> anyhow::Result<Task> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let task = Task {
            id,
            name,
            description,
            task_type,
            task_category,
            status: TaskStatus::Draft,
            plan: None,
            platform: "bilibili".to_string(),
            account_ids: Vec::new(),
            trigger: TaskTrigger {
                cron_expression: None,
                heartbeat_condition: None,
                next_run_at: None,
            },
            created_at: now,
            updated_at: now,
        };
        self.save_task(&task)?;
        Ok(task)
    }

    pub fn save_task(&self, task: &Task) -> anyhow::Result<()> {
        let path = db::models::db_path();
        let conn = db::models::open_rw(&path)?;
        db::models::upsert_task(&conn, task)?;
        Ok(())
    }

    pub fn get_task(&self, id: Uuid) -> anyhow::Result<Option<Task>> {
        let path = db::models::db_path();
        let conn = db::models::open_ro(&path)?;
        db::models::get_task(&conn, id)
    }

    pub fn delete_task(&self, id: Uuid) -> anyhow::Result<()> {
        let path = db::models::db_path();
        let conn = db::models::open_rw(&path)?;
        db::models::delete_task(&conn, id)
    }

    pub fn list_tasks(&self, status_filter: Option<TaskStatus>) -> anyhow::Result<Vec<Task>> {
        let path = db::models::db_path();
        let conn = db::models::open_ro(&path)?;
        db::models::list_tasks(&conn, status_filter)
    }

    pub fn transition_to(&self, task: &mut Task, new_status: TaskStatus) -> anyhow::Result<()> {
        state_machine::transition(&task.status, &new_status, task.task_type)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        task.status = new_status;
        task.updated_at = Utc::now();
        self.save_task(task)?;
        Ok(())
    }

    /// Map RunStatus to the appropriate TaskStatus, considering task_type for re-scheduling.
    fn run_status_to_task_status(&self, run_status: &RunStatus, task_type: TaskType) -> TaskStatus {
        match run_status {
            RunStatus::Succeeded => match task_type {
                TaskType::Cron | TaskType::Heartbeat => TaskStatus::Scheduled,
                TaskType::Instant => TaskStatus::Succeeded,
            },
            RunStatus::Failed => TaskStatus::Failed,
            RunStatus::PartiallySucceeded => TaskStatus::PartiallySucceeded,
            RunStatus::WaitingForApproval => TaskStatus::WaitingForApproval,
            RunStatus::Cancelled => TaskStatus::Cancelled,
            RunStatus::Running => TaskStatus::Running,
        }
    }

    /// Execute a task plan and auto-transition the task status based on the result.
    /// DryRun only validates the plan without changing task lifecycle status.
    pub async fn run(
        &self,
        task: &mut Task,
        run_type: RunType,
        access_token: Option<&str>,
        progress_tx: Option<mpsc::Sender<TaskProgress>>,
    ) -> anyhow::Result<admiral_types::task::TaskRun> {
        let is_dry_run = run_type == RunType::DryRun;

        if !is_dry_run && task.status != TaskStatus::Running {
            self.transition_to(task, TaskStatus::Running)?;
        }

        self.execute_and_finalize(task, run_type, None, access_token, progress_tx, is_dry_run).await
    }

    /// Resume a previously partially-succeeded task from a specific step.
    pub async fn resume_run(
        &self,
        task: &mut Task,
        from_step: Option<u32>,
        run_type: RunType,
        access_token: Option<&str>,
        progress_tx: Option<mpsc::Sender<TaskProgress>>,
    ) -> anyhow::Result<admiral_types::task::TaskRun> {
        self.transition_to(task, TaskStatus::Running)?;
        self.execute_and_finalize(task, run_type, from_step, access_token, progress_tx, false).await
    }

    /// Shared execution + post-run lifecycle: persist TaskRun and transition status.
    async fn execute_and_finalize(
        &self,
        task: &mut Task,
        run_type: RunType,
        from_step: Option<u32>,
        access_token: Option<&str>,
        progress_tx: Option<mpsc::Sender<TaskProgress>>,
        is_dry_run: bool,
    ) -> anyhow::Result<admiral_types::task::TaskRun> {
        let run = match executor::execute_plan_from_step(
            task,
            run_type,
            from_step,
            self.skill_registry.as_ref(),
            Arc::clone(&self.skill_executor),
            access_token,
            progress_tx,
        )
        .await
        {
            Ok(run) => run,
            Err(e) => {
                if !is_dry_run {
                    let _ = self.transition_to(task, TaskStatus::Failed);
                }
                return Err(e);
            }
        };

        let path = db::models::db_path();
        let conn = db::models::open_rw(&path)?;
        db::models::insert_task_run(&conn, &run)?;

        if is_dry_run {
            return Ok(run);
        }

        let new_status = self.run_status_to_task_status(&run.status, task.task_type);
        if task.status != new_status {
            self.transition_to(task, new_status)?;
        }

        Ok(run)
    }
}
