//! Task executor: runs task steps (dry_run / test_run / production) via SkillExecutor.

use admiral_types::task::{RunType, Task, TaskRun, RunStatus, StepResult};
use chrono::Utc;
use uuid::Uuid;

use crate::skill::executor::SkillExecutor;
use crate::skill::registry::SkillRegistry;
use crate::skill::validator;
use admiral_types::skill::SkillCall;

/// Execute a task plan: dry_run only validates; test_run runs one step; production runs all.
pub async fn execute_plan(
    task: &Task,
    run_type: RunType,
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

    if run_type == RunType::DryRun {
        for step in plan.steps.iter() {
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
        });
    }

    // 若第一步是 bili.account.list 且仅返回 1 个账户，后续步无 account_id 时用该值
    let mut resolved_account_id: Option<i64> = None;

    for step in plan.steps.iter() {
        let account_id = step
            .params
            .get("account_id")
            .and_then(|v| v.as_i64())
            .or_else(|| task.account_ids.first().and_then(|s| s.parse().ok()))
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

        if !success && run_type != RunType::DryRun {
            let err = step_results.last().and_then(|s| s.error.clone());
            return Ok(TaskRun {
                id: run_id,
                task_id: task.id,
                run_type,
                status: RunStatus::Failed,
                started_at,
                finished_at: Some(Utc::now()),
                step_results,
                error: err,
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
    })
}
