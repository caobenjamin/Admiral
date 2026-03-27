//! Task state machine: validates lifecycle transitions.
//!
//! ```text
//! Draft → Planning → PlanReady → Running / Scheduled
//!                                  ↓
//!               Succeeded / Failed / PartiallySucceeded / WaitingForApproval / Paused
//! ```

use admiral_types::task::{TaskStatus, TaskType};

/// Returns true if transition from `from` to `to` is allowed.
pub fn can_transition(from: TaskStatus, to: TaskStatus, task_type: TaskType) -> bool {
    use TaskStatus::*;

    match (from, to) {
        // Draft phase
        (Draft, Planning) => true,
        (Draft, PlanReady) => true,

        // Planning phase
        (Planning, PlanReady) => true,
        (Planning, Draft) => true,

        // PlanReady → execution
        (PlanReady, Running) => true,
        (PlanReady, Scheduled) => matches!(task_type, TaskType::Cron | TaskType::Heartbeat),
        (PlanReady, Draft) => true,

        // Scheduled (cron / heartbeat)
        (Scheduled, Running) => true,
        (Scheduled, Paused) => true,

        // Running outcomes
        (Running, Succeeded) => true,
        (Running, Failed) => true,
        (Running, PartiallySucceeded) => true,
        (Running, WaitingForApproval) => true,
        (Running, Paused) => true,

        // WaitingForApproval
        (WaitingForApproval, Running) => true,
        (WaitingForApproval, Cancelled) => true,

        // PartiallySucceeded recovery (user chooses in the UI)
        (PartiallySucceeded, Running) => true,
        (PartiallySucceeded, PlanReady) => true,

        // Paused
        (Paused, Running) => true,
        (Paused, Scheduled) => matches!(task_type, TaskType::Cron | TaskType::Heartbeat),

        // Terminal → retry / re-schedule
        (Succeeded, Scheduled) => matches!(task_type, TaskType::Cron | TaskType::Heartbeat),
        (Failed, PlanReady) => true,
        (Cancelled, Draft) => true,

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
