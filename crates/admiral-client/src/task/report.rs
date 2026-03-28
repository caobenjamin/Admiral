//! Execution report generator: produces a Markdown report after each task run.

use admiral_types::task::{TaskRun, RunType, RunStatus, StepResult};
use chrono::Utc;
use uuid::Uuid;

use crate::agent::memory::MemoryStore;
use crate::task::spec;

/// Generate a Markdown report from a TaskRun and optionally persist it.
pub fn generate_report(_task_id: Uuid, task_name: &str, run: &TaskRun) -> String {
    let now = Utc::now().format("%Y-%m-%d %H:%M").to_string();
    let run_type_label = match run.run_type {
        RunType::DryRun => "预检",
        RunType::TestRun => "试运行",
        RunType::Production => "正式运行",
    };
    let status_label = match run.status {
        RunStatus::Succeeded => "成功",
        RunStatus::Failed => "失败",
        RunStatus::PartiallySucceeded => "部分成功",
        RunStatus::WaitingForApproval => "等待审批",
        RunStatus::Cancelled => "已取消",
        RunStatus::Running => "运行中",
    };

    let total_duration_ms: u64 = run.step_results.iter()
        .filter_map(|s| s.duration_ms)
        .sum();
    let duration_str = if total_duration_ms >= 1000 {
        format!("{:.1}s", total_duration_ms as f64 / 1000.0)
    } else {
        format!("{}ms", total_duration_ms)
    };

    let mut md = format!(
        "## 执行报告 — {}\n\n### 概览\n- 任务：{}\n- 运行类型：{}\n- 总耗时：{}\n- 结果：**{}**\n",
        now, task_name, run_type_label, duration_str, status_label
    );

    if !run.step_results.is_empty() {
        md.push_str("\n### 步骤详情\n\n");
        md.push_str("| 步骤 | 技能 | 状态 | 耗时 |\n");
        md.push_str("|------|------|------|------|\n");
        for step in &run.step_results {
            let status = if step.success { "成功" } else { "失败" };
            let dur = step.duration_ms
                .map(|d| if d >= 1000 { format!("{:.1}s", d as f64 / 1000.0) } else { format!("{}ms", d) })
                .unwrap_or_else(|| "-".to_string());
            md.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                step.step_seq, step.skill_id, status, dur
            ));
        }
    }

    if let Some(ref error) = run.error {
        md.push_str(&format!("\n### 错误信息\n\n{}\n", error));
    }

    // Summarise key results from step outputs
    let key_results = extract_key_results(&run.step_results);
    if !key_results.is_empty() {
        md.push_str("\n### 关键结果\n\n");
        for result in &key_results {
            md.push_str(&format!("- {}\n", result));
        }
    }

    md
}

/// Save report to disk and append a summary to the task spec.
pub fn save_report(task_id: Uuid, task_name: &str, run: &TaskRun) -> Option<String> {
    let report = generate_report(task_id, task_name, run);

    let memory = MemoryStore::new().ok()?;
    let _ = memory.save_run_report(task_id, &run.id.to_string(), &report);

    // Also append a one-line summary to the task spec
    let mut task_spec = spec::load_spec(task_id);
    let run_type_label = match run.run_type {
        RunType::DryRun => "预检",
        RunType::TestRun => "试运行",
        RunType::Production => "正式运行",
    };
    let status_label = match run.status {
        RunStatus::Succeeded => "成功",
        RunStatus::Failed => "失败",
        RunStatus::PartiallySucceeded => "部分成功",
        _ => "其他",
    };
    task_spec.run_summaries.push(spec::RunSummary {
        date: Utc::now().format("%Y-%m-%d %H:%M").to_string(),
        run_type: run_type_label.to_string(),
        result: status_label.to_string(),
        notes: run.error.clone().unwrap_or_default(),
    });
    let _ = spec::save_spec(task_id, task_name, &task_spec);

    Some(report)
}

fn extract_key_results(step_results: &[StepResult]) -> Vec<String> {
    let mut results = Vec::new();
    for step in step_results {
        if !step.success {
            continue;
        }
        if let Some(ref data) = step.result {
            if let Some(total) = data.get("total").and_then(|v| v.as_u64()) {
                results.push(format!("步骤 {} 返回 {} 条记录", step.step_seq, total));
            }
            if let Some(batch_total) = data.get("batch_total").and_then(|v| v.as_u64()) {
                let succeeded = data.get("batch_succeeded").and_then(|v| v.as_u64()).unwrap_or(0);
                results.push(format!("步骤 {} 批量处理 {}/{} 成功", step.step_seq, succeeded, batch_total));
            }
            if let Some(records) = data.get("records").and_then(|v| v.as_array()) {
                results.push(format!("步骤 {} 获取到 {} 条数据", step.step_seq, records.len()));
            }
        }
    }
    results
}
