//! Task state machine: validates lifecycle transitions for conversation-driven model.
//!
//! ```text
//! Draft → Exploring → Configuring → Testing → Ready → Active
//!                          ↑            |                |
//!                          └────────────┘                |
//!                          ↑                             |
//!                          └─────────────────────────────┘
//! Execution: Running ↔ WaitingForApproval
//! Terminal:  Completed, Failed, Paused, Cancelled
//! ```

use admiral_types::task::{TaskStatus, TaskType};

/// Returns true if transition from `from` to `to` is allowed.
pub fn can_transition(from: TaskStatus, to: TaskStatus, task_type: TaskType) -> bool {
    use TaskStatus::*;

    match (from, to) {
        // Lifecycle progression (conversation-driven)
        (Draft, Exploring) => true,
        (Exploring, Configuring) => true,
        (Configuring, Testing) => true,
        (Testing, Ready) => true,
        (Ready, Active) => matches!(task_type, TaskType::Cron | TaskType::Heartbeat),

        // Backward transitions (refine after test or adjust active task)
        (Testing, Configuring) => true,
        (Ready, Configuring) => true,
        (Active, Configuring) => true,
        (Failed, Configuring) => true,
        (Completed, Configuring) => true,

        // Skip steps when Agent determines sufficient info
        (Draft, Configuring) => true,
        (Exploring, Testing) => true,

        // Execution transitions (transient during a run)
        (Testing, Running) => true,
        (Ready, Running) => true,
        (Active, Running) => true,
        (Running, WaitingForApproval) => true,
        (WaitingForApproval, Running) => true,

        // Run outcomes
        (Running, Completed) => true,
        (Running, Failed) => true,
        (Running, Testing) => true,
        (Running, Active) => matches!(task_type, TaskType::Cron | TaskType::Heartbeat),
        (Running, Ready) => true,

        // Pause/resume
        (Active, Paused) => true,
        (Running, Paused) => true,
        (Paused, Active) => matches!(task_type, TaskType::Cron | TaskType::Heartbeat),
        (Paused, Running) => true,
        (Paused, Configuring) => true,

        // Cancel from most states
        (WaitingForApproval, Cancelled) => true,
        (Paused, Cancelled) => true,
        (Failed, Cancelled) => true,
        (Cancelled, Draft) => true,

        // Re-activate after completion (cron/heartbeat)
        (Completed, Active) => matches!(task_type, TaskType::Cron | TaskType::Heartbeat),
        (Completed, Ready) => true,

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
