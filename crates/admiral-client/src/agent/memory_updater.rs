//! Automatic memory updater: analyses each conversation turn and decides whether
//! to update the task-level or user-level memory files.

use crate::agent::memory::MemoryStore;
use crate::network::api_client::ApiClient;
use admiral_types::api::LlmChatRequest;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

const TASK_MEMORY_PROMPT: &str = r#"You are a memory extraction assistant. Analyse the conversation below between a user and an AI assistant working on an advertising task.

Extract ONLY information worth remembering for future sessions about THIS SPECIFIC TASK:
- Changes to task parameters (accounts, budgets, schedules, targeting, etc.)
- User corrections or modifications to the plan
- Important decisions or confirmations
- Acceptance criteria updates
- Key outputs or results from task runs

Return a JSON object: {"updates": [{"section": "...", "content": "..."}]} where section is one of: "基本信息", "目标与验收标准", "关键参数", "对话摘要".
If nothing worth remembering, return {"updates": []}.
Respond ONLY with valid JSON, no markdown fences."#;

const USER_MEMORY_PROMPT: &str = r#"You are a memory extraction assistant. Analyse the conversation below between a user and an AI assistant.

Extract ONLY globally relevant user preferences or knowledge that should persist across ALL tasks:
- Personal preferences ("I always want...", "remember that...", "from now on...")
- Cross-task decisions (budget limits, preferred platforms, default accounts)
- Workflow preferences (approval style, report format, etc.)

Return a JSON object: {"updates": [{"section": "...", "content": "..."}]} where section is one of: "个人偏好", "常用参数", "跨任务重要记录".
If nothing worth remembering globally, return {"updates": []}.
Respond ONLY with valid JSON, no markdown fences."#;

/// Analyse the conversation and conditionally update the task memory file.
pub async fn maybe_update_task_memory(
    task_id: Uuid,
    task_name: &str,
    user_msg: &str,
    ai_response: &str,
    memory: &MemoryStore,
    api_client: &Arc<ApiClient>,
    access_token: &str,
) {
    let existing = memory.load_task_memory(task_id).unwrap_or_default();
    let conversation_snippet = format!(
        "User: {}\n\nAssistant: {}",
        truncate(user_msg, 1000),
        truncate(ai_response, 2000),
    );

    let prompt = format!(
        "{}\n\n--- Existing Task Memory ---\n{}\n\n--- Current Conversation ---\n{}",
        TASK_MEMORY_PROMPT,
        truncate(&existing, 1500),
        conversation_snippet,
    );

    let updates = call_llm_for_updates(api_client, access_token, &prompt).await;
    if updates.is_empty() {
        return;
    }

    let mut content = if existing.is_empty() {
        format!("# 任务记忆：{}\n", task_name)
    } else {
        existing
    };

    for (section, text) in &updates {
        let header = format!("## {}", section);
        if !content.contains(&header) {
            content.push_str(&format!("\n{}\n", header));
        }
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        content.push_str(&format!("- [{}] {}\n", today, text));
    }

    if let Err(e) = memory.write_task_memory(task_id, &content) {
        tracing::warn!(error = %e, "failed to write task memory");
    }
}

/// Analyse the conversation and conditionally update the user global memory file.
pub async fn maybe_update_user_memory(
    user_msg: &str,
    ai_response: &str,
    memory: &MemoryStore,
    api_client: &Arc<ApiClient>,
    access_token: &str,
) {
    let existing = memory.load_user_memory().unwrap_or_default();
    let conversation_snippet = format!(
        "User: {}\n\nAssistant: {}",
        truncate(user_msg, 1000),
        truncate(ai_response, 2000),
    );

    let prompt = format!(
        "{}\n\n--- Existing User Memory ---\n{}\n\n--- Current Conversation ---\n{}",
        USER_MEMORY_PROMPT,
        truncate(&existing, 1500),
        conversation_snippet,
    );

    let updates = call_llm_for_updates(api_client, access_token, &prompt).await;
    if updates.is_empty() {
        return;
    }

    let mut content = if existing.is_empty() {
        "# 用户全局记忆\n".to_string()
    } else {
        existing
    };

    for (section, text) in &updates {
        let header = format!("## {}", section);
        if !content.contains(&header) {
            content.push_str(&format!("\n{}\n", header));
        }
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        content.push_str(&format!("- [{}] {}\n", today, text));
    }

    if let Err(e) = memory.write_user_memory(&content) {
        tracing::warn!(error = %e, "failed to write user memory");
    }
}

/// Call LLM with a short prompt and parse the JSON response into (section, content) pairs.
async fn call_llm_for_updates(
    api_client: &Arc<ApiClient>,
    access_token: &str,
    prompt: &str,
) -> Vec<(String, String)> {
    let req = LlmChatRequest {
        messages: json!([
            {"role": "system", "content": prompt}
        ]),
        stream: false,
        model: Some("fast".to_string()),
        max_tokens: Some(512),
        tools: None,
        tool_choice: None,
        response_format: None,
    };

    let bytes = match api_client.llm_chat(access_token, &req).await {
        Ok(b) => b,
        Err(e) => {
            tracing::debug!(error = %e, "memory updater LLM call failed");
            return Vec::new();
        }
    };

    let resp: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let text = resp
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("");

    // Strip potential markdown code fences
    let clean = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: serde_json::Value = match serde_json::from_str(clean) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    parsed
        .get("updates")
        .and_then(|u| u.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let section = item.get("section")?.as_str()?;
                    let content = item.get("content")?.as_str()?;
                    if content.is_empty() {
                        None
                    } else {
                        Some((section.to_string(), content.to_string()))
                    }
                })
                .collect()
        })
        .unwrap_or_default()
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
