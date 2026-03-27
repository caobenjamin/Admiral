//! Task executor: runs task steps (dry_run / test_run / production) via SkillExecutor,
//! with permission checks for write operations.

use admiral_types::task::{RunType, Task, TaskRun, RunStatus, StepResult, PendingApproval};
use chrono::Utc;
use uuid::Uuid;

use crate::db;
use crate::skill::executor::SkillExecutor;
use crate::skill::registry::SkillRegistry;
use crate::skill::validator;
use admiral_types::skill::SkillCall;

/// Derive the operation type from a skill_id suffix.
fn classify_operation(skill_id: &str) -> OperationType {
    let parts: Vec<&str> = skill_id.split('.').collect();
    let verb = parts.last().copied().unwrap_or("");
    match verb {
        "get" | "list" | "query" | "search" | "report" | "stats" | "info" => OperationType::Read,
        "create" | "add" | "copy" | "clone" | "duplicate" => OperationType::Write,
        "update" | "edit" | "modify" | "set" | "enable" | "disable" | "pause" | "resume" => OperationType::Write,
        "delete" | "remove" | "destroy" | "archive" => OperationType::Dangerous,
        _ => OperationType::Read,
    }
}

#[derive(Debug, PartialEq)]
enum OperationType {
    Read,
    Write,
    Dangerous,
}

/// Check whether a write operation is auto-approved.
fn is_auto_approved(task: &Task, skill_id: &str) -> bool {
    let op = classify_operation(skill_id);
    if op == OperationType::Read {
        return true;
    }

    let verb = skill_id.split('.').last().unwrap_or("");
    let db_path = db::models::db_path();
    let conn = match db::models::open_ro(&db_path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    db::models::check_permission(
        &conn,
        &task.platform,
        Some(&task.id.to_string()),
        verb,
    )
    .unwrap_or(false)
}

/// Execute a task plan: dry_run only validates; test_run runs one step; production runs all.
/// Write operations are checked against the permission whitelist.
pub async fn execute_plan(
    task: &Task,
    run_type: RunType,
    skill_registry: &SkillRegistry,
    skill_executor: &SkillExecutor,
    access_token: Option<&str>,
) -> Result<TaskRun, anyhow::Error> {
    execute_plan_from_step(task, run_type, None, skill_registry, skill_executor, access_token).await
}

/// Execute a task plan starting from a specific step (for resume).
pub async fn execute_plan_from_step(
    task: &Task,
    run_type: RunType,
    resume_from: Option<u32>,
    skill_registry: &SkillRegistry,
    skill_executor: &SkillExecutor,
    access_token: Option<&str>,
) -> Result<TaskRun, anyhow::Error> {
    let plan = task
        .plan
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("task has no plan"))?;

    let run_id = Uuid::new_v4();
    let started_at = Utc::now();
    let mut step_results = Vec::new();
    let mut completed_steps: Vec<u32> = Vec::new();
    let start_seq = resume_from.unwrap_or(0);

    // --- DryRun: validate only ---
    if run_type == RunType::DryRun {
        for step in plan.steps.iter() {
            if step.seq < start_seq {
                continue;
            }
            let skill = skill_registry
                .get(&step.skill_id)
                .ok_or_else(|| anyhow::anyhow!("skill not found: {}", step.skill_id))?;
            let call = SkillCall {
                skill_id: step.skill_id.clone(),
                params: step.params.clone(),
                account_id: step.params.get("account_id").and_then(|v| v.as_i64()),
                dry_run: true,
            };
            let err_msg = validator::validate_call(&skill, &call).err();
            let success = err_msg.is_none();
            step_results.push(StepResult {
                step_seq: step.seq,
                skill_id: step.skill_id.clone(),
                success,
                result: None,
                error: err_msg,
                duration_ms: Some(0),
            });
            if success {
                completed_steps.push(step.seq);
            }
            if !success {
                break;
            }
        }
        let status = if step_results.iter().all(|s| s.success) {
            RunStatus::Succeeded
        } else {
            RunStatus::Failed
        };
        let error = step_results.iter().find(|s| !s.success).and_then(|s| s.error.clone());
        return Ok(TaskRun {
            id: run_id,
            task_id: task.id,
            run_type,
            status,
            started_at,
            finished_at: Some(Utc::now()),
            step_results,
            error,
            resume_from_step: resume_from,
            completed_steps,
        });
    }

    // --- TestRun / Production ---
    let mut resolved_account_id: Option<i64> = None;

    for step in plan.steps.iter() {
        if step.seq < start_seq {
            completed_steps.push(step.seq);
            continue;
        }

        // Permission check for write operations
        if !is_auto_approved(task, &step.skill_id) {
            let approval = PendingApproval {
                task_id: task.id,
                run_id,
                step_seq: step.seq,
                skill_id: step.skill_id.clone(),
                operation: step.skill_id.split('.').last().unwrap_or("").to_string(),
                params_summary: serde_json::to_string(&step.params).unwrap_or_default(),
            };

            let db_path = db::models::db_path();
            if let Ok(conn) = db::models::open_rw(&db_path) {
                let _ = db::models::insert_pending_approval(&conn, &approval);
            }

            let partial_status = if completed_steps.is_empty() {
                RunStatus::WaitingForApproval
            } else {
                RunStatus::WaitingForApproval
            };

            return Ok(TaskRun {
                id: run_id,
                task_id: task.id,
                run_type,
                status: partial_status,
                started_at,
                finished_at: None,
                step_results,
                error: Some(format!("waiting for approval on step {} ({})", step.seq, step.skill_id)),
                resume_from_step: Some(step.seq),
                completed_steps,
            });
        }

        let account_id = step
            .params
            .get("account_id")
            .and_then(|v| v.as_i64())
            .or_else(|| task.account_ids.first().copied())
            .or(resolved_account_id);

        if run_type == RunType::TestRun && !step_results.is_empty() {
            break;
        }

        let call = SkillCall {
            skill_id: step.skill_id.clone(),
            params: step.params.clone(),
            account_id,
            dry_run: false,
        };

        let start = std::time::Instant::now();
        let result = skill_executor.execute(call, access_token).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        let (success, result_data, error) = match result {
            Ok(sr) => (sr.success, sr.data, sr.error),
            Err(e) => (false, None, Some(e.to_string())),
        };

        if step.skill_id == "bili.account.list" && success {
            if let Some(ref data) = result_data {
                if let Some(arr) = data.get("records").and_then(|r| r.as_array()) {
                    if arr.len() == 1 {
                        if let Some(id) = arr[0].get("account_id").and_then(|v| v.as_i64()) {
                            resolved_account_id = Some(id);
                        }
                    }
                }
            }
        }

        step_results.push(StepResult {
            step_seq: step.seq,
            skill_id: step.skill_id.clone(),
            success,
            result: result_data,
            error,
            duration_ms: Some(duration_ms),
        });

        if success {
            completed_steps.push(step.seq);
        }

        if !success {
            let has_completed = !completed_steps.is_empty();
            let err = step_results.last().and_then(|s| s.error.clone());
            let status = if has_completed {
                RunStatus::PartiallySucceeded
            } else {
                RunStatus::Failed
            };
            return Ok(TaskRun {
                id: run_id,
                task_id: task.id,
                run_type,
                status,
                started_at,
                finished_at: Some(Utc::now()),
                step_results,
                error: err,
                resume_from_step: Some(step.seq),
                completed_steps,
            });
        }
    }

    let status = if step_results.iter().all(|s| s.success) {
        RunStatus::Succeeded
    } else {
        RunStatus::Failed
    };

    Ok(TaskRun {
        id: run_id,
        task_id: task.id,
        run_type,
        status,
        started_at,
        finished_at: Some(Utc::now()),
        step_results,
        error: None,
        resume_from_step: resume_from,
        completed_steps,
    })
}
