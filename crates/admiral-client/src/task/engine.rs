//! Task engine: orchestrates task lifecycle, state transitions, persistence.

use admiral_types::task::{
    Task, TaskStatus, TaskType, TaskTrigger, TriggerType, RunType,
};
use chrono::Utc;
use std::sync::Arc;
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
    ) -> anyhow::Result<Task> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let trigger_type = match task_type {
            TaskType::Instant => TriggerType::Instant,
            TaskType::Cron => TriggerType::Cron,
            TaskType::Heartbeat => TriggerType::Heartbeat,
        };
        let task = Task {
            id,
            name,
            description,
            task_type,
            status: TaskStatus::Draft,
            plan: None,
            platform: "bilibili".to_string(),
            account_ids: Vec::new(),
            trigger: TaskTrigger {
                trigger_type,
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
        let conn = db::models::open_rw(&path)?;
        db::models::get_task(&conn, id)
    }

    pub fn list_tasks(&self, status_filter: Option<TaskStatus>) -> anyhow::Result<Vec<Task>> {
        let path = db::models::db_path();
        let conn = db::models::open_rw(&path)?;
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

    pub async fn run_dry_run(
        &self,
        task: &Task,
        access_token: Option<&str>,
    ) -> anyhow::Result<admiral_types::task::TaskRun> {
        let run = executor::execute_plan(
            task,
            RunType::DryRun,
            self.skill_registry.as_ref(),
            self.skill_executor.as_ref(),
            access_token,
        )
        .await?;
        let path = db::models::db_path();
        let conn = db::models::open_rw(&path)?;
        db::models::insert_task_run(&conn, &run)?;
        Ok(run)
    }

    pub async fn run_test_run(
        &self,
        task: &Task,
        access_token: Option<&str>,
    ) -> anyhow::Result<admiral_types::task::TaskRun> {
        let run = executor::execute_plan(
            task,
            RunType::TestRun,
            self.skill_registry.as_ref(),
            self.skill_executor.as_ref(),
            access_token,
        )
        .await?;
        let path = db::models::db_path();
        let conn = db::models::open_rw(&path)?;
        db::models::insert_task_run(&conn, &run)?;
        Ok(run)
    }

    pub async fn run_production(
        &self,
        task: &Task,
        access_token: Option<&str>,
    ) -> anyhow::Result<admiral_types::task::TaskRun> {
        let run = executor::execute_plan(
            task,
            RunType::Production,
            self.skill_registry.as_ref(),
            self.skill_executor.as_ref(),
            access_token,
        )
        .await?;
        let path = db::models::db_path();
        let conn = db::models::open_rw(&path)?;
        db::models::insert_task_run(&conn, &run)?;
        Ok(run)
    }
}
