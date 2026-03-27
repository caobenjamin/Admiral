//! Task, TaskRun, TaskPlan, TaskStep and related types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::skill::RiskLevel;

/// Task type / trigger type: instant (user-triggered), cron (scheduled), heartbeat (periodic check).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    #[default]
    Instant,
    Cron,
    Heartbeat,
}

/// Task lifecycle status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Draft,
    Planning,
    PlanReady,
    DryRunning,
    DryRunPassed,
    TestRunning,
    TestRunPassed,
    Deployed,
    Scheduled,
    Running,
    Paused,
    Succeeded,
    Failed,
}

/// Trigger configuration (cron expression, heartbeat condition, next run time).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TaskTrigger {
    #[serde(default)]
    pub cron_expression: Option<String>,
    #[serde(default)]
    pub heartbeat_condition: Option<String>,
    #[serde(default)]
    pub next_run_at: Option<DateTime<Utc>>,
}

/// A single step in a task plan (skill_id + params + dependencies).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskStep {
    pub seq: u32,
    pub skill_id: String,
    pub params: serde_json::Value,
    #[serde(default)]
    pub depends_on: Vec<u32>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Full execution plan (ordered steps + metadata).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskPlan {
    pub steps: Vec<TaskStep>,
    #[serde(default)]
    pub risk_level: Option<RiskLevel>,
}

/// Main task entity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub task_type: TaskType,
    pub status: TaskStatus,
    #[serde(default)]
    pub plan: Option<TaskPlan>,
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub account_ids: Vec<i64>,
    #[serde(default)]
    pub trigger: TaskTrigger,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Type of a single task run (dry_run, test_run, production).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunType {
    DryRun,
    TestRun,
    Production,
}

/// Status of a task run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

/// Result of a single step execution within a run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StepResult {
    pub step_seq: u32,
    pub skill_id: String,
    pub success: bool,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

/// A single task execution record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskRun {
    pub id: Uuid,
    pub task_id: Uuid,
    pub run_type: RunType,
    pub status: RunStatus,
    pub started_at: DateTime<Utc>,
    #[serde(default)]
    pub finished_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub step_results: Vec<StepResult>,
    #[serde(default)]
    pub error: Option<String>,
}
