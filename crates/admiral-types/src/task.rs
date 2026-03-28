//! Task, TaskRun, TaskPlan, TaskStep and related types.

use std::collections::HashMap;

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

/// Business category of a task — influences partial-success recovery strategy in the UI.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskCategory {
    DataFetch,
    Create,
    Update,
    #[default]
    Mixed,
}

/// Task lifecycle status (conversation-driven model).
///
/// Lifecycle states: Draft → Exploring → Configuring → Testing → Ready → Active
/// Execution states: Running, WaitingForApproval (transient during a run)
/// Terminal/pause:   Completed, Failed, Paused, Cancelled
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Draft,
    Exploring,
    Configuring,
    Testing,
    Ready,
    Active,
    Running,
    WaitingForApproval,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// How complete the task specification is (evaluated by rules + LLM).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletenessLevel {
    #[default]
    Incomplete,
    Partial,
    Sufficient,
    Complete,
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

/// Retry policy for a step (exponential backoff).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetryPolicy {
    #[serde(default = "RetryPolicy::default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "RetryPolicy::default_backoff_ms")]
    pub backoff_ms: u64,
    #[serde(default = "RetryPolicy::default_backoff_multiplier")]
    pub backoff_multiplier: f32,
}

impl RetryPolicy {
    fn default_max_retries() -> u32 { 3 }
    fn default_backoff_ms() -> u64 { 1000 }
    fn default_backoff_multiplier() -> f32 { 2.0 }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: Self::default_max_retries(),
            backoff_ms: Self::default_backoff_ms(),
            backoff_multiplier: Self::default_backoff_multiplier(),
        }
    }
}

/// Execution mode for a step: single execution or iterating over a list.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StepMode {
    Single,
    ForEach {
        /// JSONPath reference to a list in step context, e.g. "$.step_1.records"
        source: String,
        /// Variable name for the current item, e.g. "current_account"
        item_binding: String,
        /// Max concurrent executions (0 = unlimited)
        #[serde(default = "StepMode::default_concurrency")]
        max_concurrency: u32,
    },
}

impl StepMode {
    fn default_concurrency() -> u32 { 5 }
}

impl Default for StepMode {
    fn default() -> Self { Self::Single }
}

/// Action to take when a step condition evaluates to false.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepActionOnFalse {
    #[default]
    Skip,
    Fail,
}

/// A single step in a task plan (skill_id + params + dependencies + data flow).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskStep {
    pub seq: u32,
    pub skill_id: String,
    pub params: serde_json::Value,
    #[serde(default)]
    pub depends_on: Vec<u32>,
    #[serde(default)]
    pub description: Option<String>,

    /// Store this step's output into context under this key (e.g. "account_list").
    #[serde(default)]
    pub output_key: Option<String>,
    /// Inject values from context into params before execution.
    /// Key = param field name, Value = JSONPath expression (e.g. "$.step_1.records[0].account_id").
    #[serde(default)]
    pub param_bindings: HashMap<String, String>,

    /// Execution mode: single or iterate over a list.
    #[serde(default)]
    pub mode: StepMode,

    /// Condition expression evaluated against step context; step runs only when true.
    /// Supports simple comparisons: "$.step_2.total > 1000", "$.step_1.count == 0".
    #[serde(default)]
    pub condition: Option<String>,
    /// What to do when condition is false.
    #[serde(default)]
    pub on_condition_false: StepActionOnFalse,

    /// Optional retry policy for transient failures.
    #[serde(default)]
    pub retry_policy: Option<RetryPolicy>,
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
    pub task_category: TaskCategory,
    pub status: TaskStatus,
    #[serde(default)]
    pub completeness: CompletenessLevel,
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
    WaitingForApproval,
    PartiallySucceeded,
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
    #[serde(default)]
    pub resume_from_step: Option<u32>,
    #[serde(default)]
    pub completed_steps: Vec<u32>,
}

/// Pending approval info attached to a run that is waiting for user confirmation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingApproval {
    pub task_id: Uuid,
    pub run_id: Uuid,
    pub step_seq: u32,
    pub skill_id: String,
    pub operation: String,
    pub params_summary: String,
}

// --------------- Task Progress Events ---------------

/// Real-time progress events emitted during task execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskProgress {
    StepStarted { step_seq: u32, total_steps: u32 },
    StepCompleted { step_seq: u32, success: bool },
    StepSkipped { step_seq: u32, reason: String },
    BatchProgress { step_seq: u32, completed: u32, total: u32 },
    TaskCompleted { status: RunStatus },
}

// --------------- Operation Permissions ---------------

/// Scope for an operation permission entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionScope {
    Platform,
    Task,
}

/// A persisted auto-approve rule for write operations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperationPermission {
    pub id: String,
    pub scope_type: PermissionScope,
    pub scope_id: String,
    pub operation: String,
    pub auto_approve: bool,
    pub granted_at: DateTime<Utc>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
}
