//! Post-conversation task spec updater: analyses each conversation turn and updates
//! the TASK_SPEC.md with newly gathered information, then evaluates completeness.

use crate::agent::memory::MemoryStore;
use crate::network::api_client::ApiClient;
use crate::task::completeness;
use crate::task::engine::TaskEngine;
use crate::task::spec::{self, TaskSpec, SpecStep, ChecklistItem};
use admiral_types::api::LlmChatRequest;
use admiral_types::task::{CompletenessLevel, TaskStatus};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

const SPEC_EXTRACTION_PROMPT: &str = r#"你是一个任务规格提取助手。分析下面用户与AI助手的对话，从中提取与任务配置相关的结构化信息。

请根据对话内容，返回一个 JSON 对象，包含以下可选字段（仅包含本轮对话中明确提到或确认的信息，不要编造）：

{
  "goal": "任务目标描述（如果用户在本轮中描述或更新了目标）",
  "platform": "平台名称",
  "accounts": ["账户名或ID"],
  "trigger_description": "触发方式描述（如：每天10:00执行）",
  "steps": [
    {"description": "步骤描述", "skill_id": "技能ID（如已明确）", "params_hint": "参数提示"}
  ],
  "checklist_updates": [
    {"label": "检查项描述", "done": true}
  ]
}

如果本轮对话没有任何与任务配置相关的新信息（例如用户只是闲聊或咨询），返回空对象 {}。
仅返回有效 JSON，不要使用 markdown 代码块。"#;

/// Analyse conversation and update the task spec. Also evaluates completeness and may advance status.
pub async fn update_task_spec(
    task_id: Uuid,
    task_name: &str,
    user_msg: &str,
    ai_response: &str,
    task_engine: &Arc<TaskEngine>,
    api_client: &Arc<ApiClient>,
    access_token: &str,
) {
    let memory = match MemoryStore::new() {
        Ok(m) => m,
        Err(_) => return,
    };

    let existing_spec = spec::load_spec(task_id);
    let existing_md = memory.load_task_spec(task_id).unwrap_or_default();

    let conversation_snippet = format!(
        "User: {}\n\nAssistant: {}",
        truncate(user_msg, 1500),
        truncate(ai_response, 2500),
    );

    let prompt = format!(
        "{}\n\n--- 当前任务规格 ---\n{}\n\n--- 本轮对话 ---\n{}",
        SPEC_EXTRACTION_PROMPT,
        truncate(&existing_md, 2000),
        conversation_snippet,
    );

    let req = LlmChatRequest {
        messages: json!([
            {"role": "system", "content": prompt}
        ]),
        stream: false,
        model: Some("fast".to_string()),
        max_tokens: Some(1024),
        tools: None,
        tool_choice: None,
        response_format: None,
    };

    let bytes = match api_client.llm_chat(access_token, &req).await {
        Ok(b) => b,
        Err(e) => {
            tracing::debug!(error = %e, "spec updater LLM call failed");
            return;
        }
    };

    let resp: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return,
    };

    let text = resp
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("");

    let clean = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: serde_json::Value = match serde_json::from_str(clean) {
        Ok(v) => v,
        Err(_) => return,
    };

    // If empty object, nothing to update
    if parsed.as_object().map(|o| o.is_empty()).unwrap_or(true) {
        // Still evaluate completeness in case spec was updated externally
        evaluate_and_advance(task_id, &existing_spec, task_engine);
        return;
    }

    let updated_spec = merge_extraction(existing_spec, &parsed);
    if let Err(e) = spec::save_spec(task_id, task_name, &updated_spec) {
        tracing::warn!(error = %e, "failed to save updated task spec");
        return;
    }

    evaluate_and_advance(task_id, &updated_spec, task_engine);
}

/// Merge LLM-extracted fields into the existing spec (additive, not destructive).
fn merge_extraction(mut spec: TaskSpec, extracted: &serde_json::Value) -> TaskSpec {
    if let Some(goal) = extracted.get("goal").and_then(|v| v.as_str()) {
        if !goal.is_empty() {
            spec.goal = goal.to_string();
        }
    }

    if let Some(platform) = extracted.get("platform").and_then(|v| v.as_str()) {
        if !platform.is_empty() {
            spec.context.platform = platform.to_string();
        }
    }

    if let Some(accounts) = extracted.get("accounts").and_then(|v| v.as_array()) {
        let new_accounts: Vec<String> = accounts
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        if !new_accounts.is_empty() {
            spec.context.accounts = new_accounts;
        }
    }

    if let Some(trigger) = extracted.get("trigger_description").and_then(|v| v.as_str()) {
        if !trigger.is_empty() {
            spec.context.trigger_description = trigger.to_string();
        }
    }

    if let Some(steps) = extracted.get("steps").and_then(|v| v.as_array()) {
        let existing_count = spec.steps.len() as u32;
        for (i, step_val) in steps.iter().enumerate() {
            let desc = step_val.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let skill_id = step_val.get("skill_id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let params_hint = step_val.get("params_hint").and_then(|v| v.as_str()).unwrap_or("").to_string();

            if desc.is_empty() {
                continue;
            }

            // Try to match by description to update existing step, otherwise append
            if let Some(existing) = spec.steps.iter_mut().find(|s| s.description == desc) {
                if skill_id.is_some() {
                    existing.skill_id = skill_id;
                }
                if !params_hint.is_empty() {
                    existing.params_hint = params_hint;
                }
            } else {
                spec.steps.push(SpecStep {
                    seq: existing_count + i as u32 + 1,
                    description: desc,
                    skill_id,
                    params_hint,
                });
            }
        }
    }

    if let Some(checklist) = extracted.get("checklist_updates").and_then(|v| v.as_array()) {
        for item_val in checklist {
            let label = item_val.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let done = item_val.get("done").and_then(|v| v.as_bool()).unwrap_or(false);
            if label.is_empty() {
                continue;
            }
            if let Some(existing) = spec.checklist.iter_mut().find(|c| c.label == label) {
                existing.done = done;
            } else {
                spec.checklist.push(ChecklistItem { label, done });
            }
        }
    }

    spec
}

/// Evaluate completeness and possibly advance task status.
fn evaluate_and_advance(task_id: Uuid, spec: &TaskSpec, task_engine: &Arc<TaskEngine>) {
    let level = completeness::evaluate(spec);

    let mut task = match task_engine.get_task(task_id) {
        Ok(Some(t)) => t,
        _ => return,
    };

    // Update completeness
    if task.completeness != level {
        let _ = task_engine.set_completeness(&mut task, level);
    }

    // Auto-advance status based on completeness
    match (task.status, level) {
        (TaskStatus::Draft, _) if level != CompletenessLevel::Incomplete => {
            let _ = task_engine.transition_to(&mut task, TaskStatus::Exploring);
        }
        (TaskStatus::Exploring, CompletenessLevel::Partial)
        | (TaskStatus::Exploring, CompletenessLevel::Sufficient)
        | (TaskStatus::Exploring, CompletenessLevel::Complete) => {
            let _ = task_engine.transition_to(&mut task, TaskStatus::Configuring);
        }
        _ => {}
    }
}

fn truncate(s: &str, max_chars: usize) -> &str {
    if s.len() <= max_chars {
        s
    } else {
        let mut end = max_chars;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        &s[..end]
    }
}
