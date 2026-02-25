//! Task state machine: Draft → DryRun → TestRun → Deployed → Running → Succeeded/Failed.

use admiral_types::task::{TaskStatus, TaskType};

/// Returns true if transition from `from` to `to` is allowed.
pub fn can_transition(from: TaskStatus, to: TaskStatus, task_type: TaskType) -> bool {
    use admiral_types::task::TaskStatus::*;

    match (from, to) {
        (Draft, Planning) => true,
        (Draft, PlanReady) => true,
        (Planning, PlanReady) => true,
        (Planning, Draft) => true,
        (PlanReady, DryRunning) => true,
        (PlanReady, Draft) => true,
        (DryRunning, DryRunPassed) => true,
        (DryRunning, Draft) => true,
        (DryRunPassed, TestRunning) => true,
        (DryRunPassed, Draft) => true,
        (TestRunning, TestRunPassed) => true,
        (TestRunning, Draft) => true,
        (TestRunPassed, Deployed) => true,
        (TestRunPassed, Draft) => true,
        (Deployed, Running) => task_type == TaskType::Instant,
        (Deployed, Scheduled) => matches!(task_type, TaskType::Cron | TaskType::Heartbeat),
        (Scheduled, Running) => true,
        (Running, Succeeded) => true,
        (Running, Failed) => true,
        (Running, Paused) => true,
        (Paused, Running) => true,
        _ => false,
    }
}

/// Transition task to new status if allowed. Returns new status or error.
pub fn transition(
    from: &TaskStatus,
    to: &TaskStatus,
    task_type: TaskType,
) -> Result<TaskStatus, String> {
    if can_transition(*from, *to, task_type) {
        Ok(*to)
    } else {
        Err(format!(
            "invalid transition from {:?} to {:?} for task_type {:?}",
            from, to, task_type
        ))
    }
}
