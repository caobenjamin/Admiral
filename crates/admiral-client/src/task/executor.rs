//! Task executor: runs task steps (dry_run / test_run / production) via SkillExecutor,
//! with step context pipeline, ForEach batch execution, conditional steps,
//! retry with backoff, and permission checks for write operations.

use std::collections::HashMap;
use std::sync::Arc;

use admiral_types::task::{
    RunType, Task, TaskRun, RunStatus, StepResult, PendingApproval,
    StepMode, StepActionOnFalse, TaskProgress,
};
use chrono::Utc;
use uuid::Uuid;
use tokio::sync::mpsc;
use tracing::warn;

use crate::db;
use crate::skill::executor::SkillExecutor;
use crate::skill::registry::SkillRegistry;
use crate::skill::validator;
use admiral_types::skill::SkillCall;

// --------------- Step Context ---------------

/// Runtime context that accumulates step outputs and allows cross-step data references.
#[derive(Debug, Default)]
pub struct StepContext {
    data: HashMap<String, serde_json::Value>,
}

impl StepContext {
    pub fn new() -> Self { Self::default() }

    pub fn insert(&mut self, key: String, value: serde_json::Value) {
        self.data.insert(key, value);
    }

    /// Resolve a JSONPath-like expression against the context.
    /// Supports: "$.key.field.nested" and "$.key.array[0].field"
    pub fn resolve(&self, path: &str) -> Option<serde_json::Value> {
        let path = path.strip_prefix("$.")?;
        let mut segments: Vec<&str> = Vec::new();
        for part in path.split('.') {
            if let Some(bracket_pos) = part.find('[') {
                segments.push(&part[..bracket_pos]);
                let idx_str = part[bracket_pos + 1..].strip_suffix(']')?;
                segments.push(idx_str);
            } else {
                segments.push(part);
            }
        }

        if segments.is_empty() {
            return None;
        }

        let root_key = segments[0];
        let mut current = self.data.get(root_key)?.clone();

        for segment in &segments[1..] {
            if let Ok(index) = segment.parse::<usize>() {
                current = current.as_array()?.get(index)?.clone();
            } else {
                current = current.get(segment)?.clone();
            }
        }
        Some(current)
    }

    /// Apply param_bindings: resolve each binding and merge into params.
    pub fn apply_bindings(
        &self,
        params: &serde_json::Value,
        bindings: &HashMap<String, String>,
    ) -> serde_json::Value {
        let mut result = params.clone();
        let obj = match result.as_object_mut() {
            Some(o) => o,
            None => return result,
        };
        for (param_key, path_expr) in bindings {
            if let Some(value) = self.resolve(path_expr) {
                obj.insert(param_key.clone(), value);
            }
        }
        result
    }
}

// --------------- Condition Evaluation ---------------

/// Evaluate a simple condition expression against step context.
/// Supports: "$.key.field > 1000", "$.key.count == 0", "$.key.name != \"foo\""
fn evaluate_condition(expr: &str, ctx: &StepContext) -> bool {
    let operators = [">=", "<=", "!=", "==", ">", "<"];
    for op in &operators {
        if let Some(pos) = expr.find(op) {
            let lhs_str = expr[..pos].trim();
            let rhs_str = expr[pos + op.len()..].trim();

            let lhs = match ctx.resolve(lhs_str) {
                Some(v) => v,
                None => return false,
            };

            return compare_values(&lhs, rhs_str, op);
        }
    }
    // No operator found — treat as truthy check on the resolved value
    ctx.resolve(expr.trim())
        .map(|v| !v.is_null() && v.as_bool() != Some(false))
        .unwrap_or(false)
}

fn compare_values(lhs: &serde_json::Value, rhs_str: &str, op: &str) -> bool {
    if let Some(lhs_num) = lhs.as_f64() {
        if let Ok(rhs_num) = rhs_str.parse::<f64>() {
            return match op {
                ">" => lhs_num > rhs_num,
                "<" => lhs_num < rhs_num,
                ">=" => lhs_num >= rhs_num,
                "<=" => lhs_num <= rhs_num,
                "==" => (lhs_num - rhs_num).abs() < f64::EPSILON,
                "!=" => (lhs_num - rhs_num).abs() >= f64::EPSILON,
                _ => false,
            };
        }
    }
    let lhs_str = match lhs.as_str() {
        Some(s) => s.to_string(),
        None => lhs.to_string(),
    };
    let rhs_clean = rhs_str.trim_matches('"');
    match op {
        "==" => lhs_str == rhs_clean,
        "!=" => lhs_str != rhs_clean,
        _ => false,
    }
}

// --------------- Operation Classification ---------------

fn classify_operation(skill_id: &str) -> OperationType {
    let verb = skill_id.split('.').last().unwrap_or("");
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
    db::models::check_permission(&conn, &task.platform, Some(&task.id.to_string()), verb)
        .unwrap_or(false)
}

// --------------- Retry Helper ---------------

async fn execute_with_retry(
    skill_executor: &SkillExecutor,
    call: &SkillCall,
    access_token: Option<&str>,
    retry_policy: Option<&admiral_types::task::RetryPolicy>,
) -> Result<admiral_types::skill::SkillResult, anyhow::Error> {
    let max_retries = retry_policy.map(|r| r.max_retries).unwrap_or(0);
    let backoff_ms = retry_policy.map(|r| r.backoff_ms).unwrap_or(1000);
    let multiplier = retry_policy.map(|r| r.backoff_multiplier).unwrap_or(2.0);

    let mut attempt = 0;
    loop {
        let result = skill_executor.execute(call.clone(), access_token).await;
        match &result {
            Ok(sr) if sr.success => return result,
            Ok(_) | Err(_) if attempt < max_retries => {
                let delay = backoff_ms as f64 * multiplier.powi(attempt as i32) as f64;
                warn!(attempt, max_retries, delay_ms = delay as u64, "step failed, retrying");
                tokio::time::sleep(std::time::Duration::from_millis(delay as u64)).await;
                attempt += 1;
            }
            _ => return result,
        }
    }
}

// --------------- TaskRun Builder ---------------

fn make_task_run(
    run_id: Uuid,
    task_id: Uuid,
    run_type: RunType,
    status: RunStatus,
    started_at: chrono::DateTime<Utc>,
    finished: bool,
    step_results: Vec<StepResult>,
    error: Option<String>,
    resume_from_step: Option<u32>,
    completed_steps: Vec<u32>,
) -> TaskRun {
    TaskRun {
        id: run_id,
        task_id,
        run_type,
        status,
        started_at,
        finished_at: if finished { Some(Utc::now()) } else { None },
        step_results,
        error,
        resume_from_step,
        completed_steps,
    }
}

// --------------- Main Executor ---------------

pub async fn execute_plan(
    task: &Task,
    run_type: RunType,
    skill_registry: &SkillRegistry,
    skill_executor: Arc<SkillExecutor>,
    access_token: Option<&str>,
    progress_tx: Option<mpsc::Sender<TaskProgress>>,
) -> Result<TaskRun, anyhow::Error> {
    execute_plan_from_step(task, run_type, None, skill_registry, skill_executor, access_token, progress_tx).await
}

pub async fn execute_plan_from_step(
    task: &Task,
    run_type: RunType,
    resume_from: Option<u32>,
    skill_registry: &SkillRegistry,
    skill_executor: Arc<SkillExecutor>,
    access_token: Option<&str>,
    progress_tx: Option<mpsc::Sender<TaskProgress>>,
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
    let total_steps = plan.steps.len() as u32;
    let mut ctx = StepContext::new();

    // --- DryRun: validate only ---
    if run_type == RunType::DryRun {
        for step in plan.steps.iter() {
            if step.seq < start_seq {
                continue;
            }
            let skill = skill_registry
                .get(&step.skill_id)
                .ok_or_else(|| anyhow::anyhow!("skill not found: {}", step.skill_id))?;
            let resolved_params = ctx.apply_bindings(&step.params, &step.param_bindings);
            let account_id = resolved_params.get("account_id").and_then(|v| v.as_i64());
            let call = SkillCall {
                skill_id: step.skill_id.clone(),
                params: resolved_params,
                account_id,
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
        return Ok(make_task_run(
            run_id, task.id, run_type, status, started_at, true,
            step_results, error, resume_from, completed_steps,
        ));
    }

    // --- TestRun / Production ---
    for step in plan.steps.iter() {
        if step.seq < start_seq {
            completed_steps.push(step.seq);
            continue;
        }

        if run_type == RunType::TestRun && !step_results.is_empty() {
            break;
        }

        // Condition evaluation
        if let Some(ref condition_expr) = step.condition {
            if !evaluate_condition(condition_expr, &ctx) {
                match step.on_condition_false {
                    StepActionOnFalse::Skip => {
                        send_progress(&progress_tx, TaskProgress::StepSkipped {
                            step_seq: step.seq,
                            reason: format!("condition not met: {}", condition_expr),
                        }).await;
                        step_results.push(StepResult {
                            step_seq: step.seq,
                            skill_id: step.skill_id.clone(),
                            success: true,
                            result: Some(serde_json::json!({"skipped": true, "reason": "condition not met"})),
                            error: None,
                            duration_ms: Some(0),
                        });
                        completed_steps.push(step.seq);
                        continue;
                    }
                    StepActionOnFalse::Fail => {
                        let err = format!("condition not met: {}", condition_expr);
                        step_results.push(StepResult {
                            step_seq: step.seq,
                            skill_id: step.skill_id.clone(),
                            success: false,
                            result: None,
                            error: Some(err.clone()),
                            duration_ms: Some(0),
                        });
                        let status = if completed_steps.is_empty() { RunStatus::Failed } else { RunStatus::PartiallySucceeded };
                        return Ok(make_task_run(
                            run_id, task.id, run_type, status, started_at, true,
                            step_results, Some(err), Some(step.seq), completed_steps,
                        ));
                    }
                }
            }
        }

        send_progress(&progress_tx, TaskProgress::StepStarted {
            step_seq: step.seq,
            total_steps,
        }).await;

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
            return Ok(make_task_run(
                run_id, task.id, run_type, RunStatus::WaitingForApproval, started_at, false,
                step_results,
                Some(format!("waiting for approval on step {} ({})", step.seq, step.skill_id)),
                Some(step.seq), completed_steps,
            ));
        }

        // Resolve params with context bindings
        let resolved_params = ctx.apply_bindings(&step.params, &step.param_bindings);

        match &step.mode {
            StepMode::Single => {
                let (success, result_data, error, duration_ms) = execute_single_step(
                    task, &step.skill_id, &resolved_params, &skill_executor, access_token,
                    step.retry_policy.as_ref(),
                ).await;

                if success {
                    if let Some(ref key) = step.output_key {
                        if let Some(ref data) = result_data {
                            ctx.insert(key.clone(), data.clone());
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

                send_progress(&progress_tx, TaskProgress::StepCompleted {
                    step_seq: step.seq,
                    success,
                }).await;

                if success {
                    completed_steps.push(step.seq);
                } else {
                    let status = if !completed_steps.is_empty() { RunStatus::PartiallySucceeded } else { RunStatus::Failed };
                    let err = step_results.last().and_then(|s| s.error.clone());
                    return Ok(make_task_run(
                        run_id, task.id, run_type, status, started_at, true,
                        step_results, err, Some(step.seq), completed_steps,
                    ));
                }
            }
            StepMode::ForEach { source, item_binding, max_concurrency } => {
                let items = ctx.resolve(source)
                    .and_then(|v| v.as_array().cloned())
                    .unwrap_or_default();
                let total_items = items.len() as u32;

                if items.is_empty() {
                    step_results.push(StepResult {
                        step_seq: step.seq,
                        skill_id: step.skill_id.clone(),
                        success: true,
                        result: Some(serde_json::json!({"batch_total": 0, "batch_succeeded": 0})),
                        error: None,
                        duration_ms: Some(0),
                    });
                    completed_steps.push(step.seq);
                    send_progress(&progress_tx, TaskProgress::StepCompleted {
                        step_seq: step.seq,
                        success: true,
                    }).await;
                    continue;
                }

                let concurrency = if *max_concurrency == 0 { items.len() } else { *max_concurrency as usize };
                let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
                let mut handles = Vec::new();

                for (idx, item) in items.into_iter().enumerate() {
                    let mut item_params = resolved_params.clone();
                    if let Some(obj) = item_params.as_object_mut() {
                        obj.insert(item_binding.clone(), item.clone());
                        if let Some(account_id) = item.get("account_id") {
                            obj.insert("account_id".to_string(), account_id.clone());
                        }
                    }

                    let sem = semaphore.clone();
                    let skill_id = step.skill_id.clone();
                    let retry_policy = step.retry_policy.clone();
                    let account_ids = task.account_ids.clone();
                    let progress_tx_clone = progress_tx.clone();
                    let step_seq = step.seq;
                    let executor = Arc::clone(&skill_executor);
                    let token = access_token.map(|s| s.to_string());

                    handles.push(tokio::spawn(async move {
                        let _permit = sem.acquire().await.unwrap();
                        let account_id = item_params.get("account_id").and_then(|v| v.as_i64())
                            .or_else(|| account_ids.first().copied());
                        let call = SkillCall {
                            skill_id: skill_id.clone(),
                            params: item_params,
                            account_id,
                            dry_run: false,
                        };
                        let start = std::time::Instant::now();
                        let result = execute_with_retry(
                            &executor, &call, token.as_deref(), retry_policy.as_ref(),
                        ).await;
                        let duration_ms = start.elapsed().as_millis() as u64;
                        let (success, data, error) = match result {
                            Ok(sr) => (sr.success, sr.data, sr.error),
                            Err(e) => (false, None, Some(e.to_string())),
                        };

                        send_progress(&progress_tx_clone, TaskProgress::BatchProgress {
                            step_seq,
                            completed: (idx + 1) as u32,
                            total: total_items,
                        }).await;

                        (success, data, error, duration_ms)
                    }));
                }

                let mut batch_succeeded = 0u32;
                let mut batch_failed = 0u32;
                let mut batch_results = Vec::new();
                let mut first_error = None;

                for handle in handles {
                    let (success, data, error, duration) = handle.await
                        .unwrap_or((false, None, Some("task join error".to_string()), 0));
                    if success {
                        batch_succeeded += 1;
                    } else {
                        batch_failed += 1;
                        if first_error.is_none() {
                            first_error = error.clone();
                        }
                    }
                    batch_results.push(serde_json::json!({
                        "success": success,
                        "data": data,
                        "error": error,
                        "duration_ms": duration,
                    }));
                }

                let batch_success = batch_failed == 0;
                let result_data = serde_json::json!({
                    "batch_total": batch_succeeded + batch_failed,
                    "batch_succeeded": batch_succeeded,
                    "batch_failed": batch_failed,
                    "results": batch_results,
                });

                if let Some(ref key) = step.output_key {
                    ctx.insert(key.clone(), result_data.clone());
                }

                step_results.push(StepResult {
                    step_seq: step.seq,
                    skill_id: step.skill_id.clone(),
                    success: batch_success,
                    result: Some(result_data),
                    error: first_error.clone(),
                    duration_ms: None,
                });

                send_progress(&progress_tx, TaskProgress::StepCompleted {
                    step_seq: step.seq,
                    success: batch_success,
                }).await;

                if batch_success {
                    completed_steps.push(step.seq);
                } else if batch_succeeded > 0 {
                    completed_steps.push(step.seq);
                    // Partial batch success: continue but record the failures
                } else {
                    let status = if !completed_steps.is_empty() { RunStatus::PartiallySucceeded } else { RunStatus::Failed };
                    return Ok(make_task_run(
                        run_id, task.id, run_type, status, started_at, true,
                        step_results, first_error, Some(step.seq), completed_steps,
                    ));
                }
            }
        }
    }

    send_progress(&progress_tx, TaskProgress::TaskCompleted {
        status: RunStatus::Succeeded,
    }).await;

    Ok(make_task_run(
        run_id, task.id, run_type, RunStatus::Succeeded, started_at, true,
        step_results, None, resume_from, completed_steps,
    ))
}

/// Execute a single step with retry support.
async fn execute_single_step(
    task: &Task,
    skill_id: &str,
    params: &serde_json::Value,
    skill_executor: &SkillExecutor,
    access_token: Option<&str>,
    retry_policy: Option<&admiral_types::task::RetryPolicy>,
) -> (bool, Option<serde_json::Value>, Option<String>, u64) {
    let account_id = params.get("account_id").and_then(|v| v.as_i64())
        .or_else(|| task.account_ids.first().copied());

    let call = SkillCall {
        skill_id: skill_id.to_string(),
        params: params.clone(),
        account_id,
        dry_run: false,
    };

    let start = std::time::Instant::now();
    let result = execute_with_retry(skill_executor, &call, access_token, retry_policy).await;
    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(sr) => (sr.success, sr.data, sr.error, duration_ms),
        Err(e) => (false, None, Some(e.to_string()), duration_ms),
    }
}

async fn send_progress(tx: &Option<mpsc::Sender<TaskProgress>>, event: TaskProgress) {
    if let Some(sender) = tx {
        let _ = sender.send(event).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_context_resolve() {
        let mut ctx = StepContext::new();
        ctx.insert("step_1".to_string(), serde_json::json!({
            "records": [
                {"account_id": 12345, "name": "test_account"},
                {"account_id": 67890, "name": "test_account_2"},
            ],
            "total": 2
        }));

        assert_eq!(ctx.resolve("$.step_1.total"), Some(serde_json::json!(2)));
        assert_eq!(
            ctx.resolve("$.step_1.records[0].account_id"),
            Some(serde_json::json!(12345))
        );
        assert_eq!(
            ctx.resolve("$.step_1.records[1].name"),
            Some(serde_json::json!("test_account_2"))
        );
        assert_eq!(ctx.resolve("$.nonexistent"), None);
    }

    #[test]
    fn test_condition_evaluation() {
        let mut ctx = StepContext::new();
        ctx.insert("step_1".to_string(), serde_json::json!({
            "total": 1500,
            "count": 0,
            "name": "foo"
        }));

        assert!(evaluate_condition("$.step_1.total > 1000", &ctx));
        assert!(!evaluate_condition("$.step_1.total < 1000", &ctx));
        assert!(evaluate_condition("$.step_1.count == 0", &ctx));
        assert!(evaluate_condition("$.step_1.name == \"foo\"", &ctx));
        assert!(!evaluate_condition("$.step_1.name == \"bar\"", &ctx));
        assert!(evaluate_condition("$.step_1.total >= 1500", &ctx));
        assert!(evaluate_condition("$.step_1.total <= 1500", &ctx));
    }

    #[test]
    fn test_apply_bindings() {
        let mut ctx = StepContext::new();
        ctx.insert("step_1".to_string(), serde_json::json!({
            "records": [{"account_id": 42}]
        }));

        let params = serde_json::json!({"other_param": "value"});
        let mut bindings = HashMap::new();
        bindings.insert("account_id".to_string(), "$.step_1.records[0].account_id".to_string());

        let result = ctx.apply_bindings(&params, &bindings);
        assert_eq!(result.get("account_id"), Some(&serde_json::json!(42)));
        assert_eq!(result.get("other_param"), Some(&serde_json::json!("value")));
    }
}
