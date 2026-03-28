//! Rule-based completeness evaluator for task specifications.
//! Checks required, recommended, and optional fields to determine a CompletenessLevel.

use admiral_types::task::CompletenessLevel;
use crate::task::spec::TaskSpec;

/// Evaluate the completeness level of a task spec using deterministic rules.
pub fn evaluate(spec: &TaskSpec) -> CompletenessLevel {
    let mut required_met = 0u32;
    let mut required_total = 0u32;
    let mut recommended_met = 0u32;
    let mut recommended_total = 0u32;

    // --- Required items ---
    required_total += 1;
    if !spec.goal.is_empty() && spec.goal != "（待定义）" {
        required_met += 1;
    }

    required_total += 1;
    if !spec.steps.is_empty() {
        required_met += 1;
    }

    // --- Recommended items ---
    recommended_total += 1;
    if !spec.context.platform.is_empty() {
        recommended_met += 1;
    }

    recommended_total += 1;
    if !spec.context.accounts.is_empty() && !spec.context.accounts.iter().all(|a| a.is_empty()) {
        recommended_met += 1;
    }

    recommended_total += 1;
    if !spec.context.trigger_description.is_empty() {
        recommended_met += 1;
    }

    recommended_total += 1;
    if spec.steps.iter().any(|s| s.skill_id.is_some()) {
        recommended_met += 1;
    }

    // --- Determine level ---
    if required_met == 0 {
        CompletenessLevel::Incomplete
    } else if required_met < required_total {
        CompletenessLevel::Partial
    } else if recommended_met < recommended_total / 2 {
        CompletenessLevel::Partial
    } else if recommended_met < recommended_total {
        CompletenessLevel::Sufficient
    } else {
        CompletenessLevel::Sufficient
    }
}

/// After a successful test run, promote to Complete.
pub fn promote_after_test_run(current: CompletenessLevel) -> CompletenessLevel {
    match current {
        CompletenessLevel::Sufficient => CompletenessLevel::Complete,
        CompletenessLevel::Complete => CompletenessLevel::Complete,
        other => other,
    }
}

/// Build a list of human-readable missing items for the current spec.
pub fn missing_items(spec: &TaskSpec) -> Vec<String> {
    let mut items = Vec::new();

    if spec.goal.is_empty() || spec.goal == "（待定义）" {
        items.push("任务目标尚未描述".to_string());
    }
    if spec.steps.is_empty() {
        items.push("尚未确定实现步骤".to_string());
    }
    if spec.context.platform.is_empty() {
        items.push("平台信息未确认".to_string());
    }
    if spec.context.accounts.is_empty() || spec.context.accounts.iter().all(|a| a.is_empty()) {
        items.push("关联账户未选择".to_string());
    }
    if spec.context.trigger_description.is_empty() {
        items.push("触发方式/频率未确认".to_string());
    }
    if !spec.steps.is_empty() && spec.steps.iter().all(|s| s.skill_id.is_none()) {
        items.push("步骤尚未关联到具体技能".to_string());
    }

    items
}
