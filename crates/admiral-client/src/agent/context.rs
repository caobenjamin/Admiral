//! Context assembler: agent profile + session history + memory + task spec + available skills.

use crate::agent::memory::MemoryStore;
use crate::agent::profile::AgentProfile;
use crate::skill::registry::SkillRegistry;
use crate::task::completeness;
use crate::task::spec;
use chrono::Local;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// One message in the conversation (OpenAI-compatible shape).
#[derive(Clone, Debug, serde::Serialize)]
pub struct Message {
    pub role: String,
    pub content: Option<String>,
    pub tool_calls: Option<serde_json::Value>,
    pub name: Option<String>,
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn to_openai_value(&self) -> serde_json::Value {
        let mut m = serde_json::Map::new();
        m.insert("role".to_string(), json!(self.role));
        if let Some(ref c) = self.content {
            m.insert("content".to_string(), json!(c));
        }
        if let Some(ref t) = self.tool_calls {
            m.insert("tool_calls".to_string(), t.clone());
        }
        if let Some(ref n) = self.name {
            m.insert("name".to_string(), json!(n));
        }
        if let Some(ref id) = self.tool_call_id {
            m.insert("tool_call_id".to_string(), json!(id));
        }
        json!(m)
    }
}

/// Session holds conversation history for one task (or main session).
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Session {
    pub id: String,
    pub task_id: Option<Uuid>,
    pub messages: Vec<Message>,
}

/// Builds the full message list for an LLM call: system + user memory + task spec + tools + history.
pub struct ContextAssembler {
    pub profile: AgentProfile,
    pub memory: MemoryStore,
    pub skill_registry: Arc<SkillRegistry>,
    pub max_history: usize,
}

impl ContextAssembler {
    pub fn new(
        profile: AgentProfile,
        memory: MemoryStore,
        skill_registry: Arc<SkillRegistry>,
    ) -> Self {
        Self {
            profile,
            memory,
            skill_registry,
            max_history: 20,
        }
    }

    /// Build the messages array for the LLM API.
    /// The current user message should already be in `session.messages`.
    pub fn build(
        &self,
        session: &Session,
        knowledge_snippets: &[String],
        runtime_notes: &[String],
    ) -> serde_json::Value {
        let mut system_parts = vec![self.profile.system_prompt.clone()];

        let today = Local::now().format("%Y-%m-%d").to_string();
        system_parts.push(format!(
            "## Current date (use for report queries, start_time/end_time)\n\nToday is {} (YYYY-MM-DD).",
            today
        ));
        system_parts.push(
            "## Report Query Rules\n\n\
             - For report analysis requests, first resolve account_id. If user did not provide one, call `bili_account_list_local`.\n\
             - After account_id is known, use `bili_report_query` to fetch actual data instead of answering from guesswork.\n\
             - `bili_report_query` can only query one day at a time because `start_time` and `end_time` must be the same date.\n\
             - For a month-by-day analysis, call the report tool repeatedly for each day needed, then summarize the results.".to_string(),
        );

        if let Ok(user_mem) = self.memory.load_user_memory() {
            if !user_mem.is_empty() {
                system_parts.push(format!("## User Memory\n\n{}", user_mem));
            }
        }

        // Inject task spec and completeness context
        if let Some(ref task_id) = session.task_id {
            if let Ok(task_mem) = self.memory.load_task_memory(*task_id) {
                if !task_mem.is_empty() {
                    system_parts.push(format!("## Task Memory\n\n{}", task_mem));
                }
            }

            let task_spec = spec::load_spec(*task_id);
            let spec_md = self.memory.load_task_spec(*task_id).unwrap_or_default();
            if !spec_md.is_empty() {
                system_parts.push(format!("## 当前任务规格文档\n\n{}", spec_md));
            }

            let level = completeness::evaluate(&task_spec);
            let missing = completeness::missing_items(&task_spec);
            let level_label = match level {
                admiral_types::task::CompletenessLevel::Incomplete => "不完整",
                admiral_types::task::CompletenessLevel::Partial => "部分完整",
                admiral_types::task::CompletenessLevel::Sufficient => "基本完整",
                admiral_types::task::CompletenessLevel::Complete => "完整",
            };

            let mut completeness_section = format!(
                "## 任务信息完整度\n\n当前完整度：**{}**\n",
                level_label
            );
            if !missing.is_empty() {
                completeness_section.push_str("\n缺失信息：\n");
                for item in &missing {
                    completeness_section.push_str(&format!("- {}\n", item));
                }
            }
            completeness_section.push_str(
                "\n**指引**：\n\
                 - 根据用户描述，逐步完善任务规格\n\
                 - 主动询问上述缺失的关键信息\n\
                 - 当完整度达到「基本完整」时，建议用户进行试运行\n\
                 - 每次对话都要帮助推进任务配置\n"
            );
            system_parts.push(completeness_section);
        }

        if !knowledge_snippets.is_empty() {
            system_parts.push(
                "## Relevant Knowledge\n\n".to_string()
                    + &knowledge_snippets.join("\n\n"),
            );
        }

        let llm_tools = self.skill_registry.build_llm_tools();
        if !llm_tools.is_empty() {
            let mut tool_section = String::from(
                "## Available Tools\n\n\
                You can only choose tools from this list. When returning `call_tool`, use the exact tool name and JSON arguments.\n",
            );
            for tool in llm_tools {
                let schema = tool
                    .function
                    .parameters
                    .as_ref()
                    .map(|v| serde_json::to_string_pretty(v).unwrap_or_else(|_| "{}".to_string()))
                    .unwrap_or_else(|| "{}".to_string());
                tool_section.push_str(&format!(
                    "\n### {}\n{}\nParameters schema:\n```json\n{}\n```\n",
                    tool.function.name,
                    tool.function.description,
                    schema
                ));
            }
            system_parts.push(tool_section);
        }

        system_parts.push(
            "## Action Protocol\n\n\
            In every turn, respond with exactly one JSON object and nothing else.\n\
            Allowed actions:\n\
            - {\"type\":\"call_tool\",\"tool_name\":\"exact_name\",\"arguments\":{...},\"reason\":\"short reason\"}\n\
            - {\"type\":\"respond\",\"message\":\"final answer to user\"}\n\
            - {\"type\":\"ask_user\",\"message\":\"specific question for missing info\"}\n\
            Rules:\n\
            - Never output markdown fences.\n\
            - Never mix prose outside JSON.\n\
            - If the task can be completed with tools, prefer `call_tool`.\n\
            - For multi-day report tasks, fetch one day per tool call and stop calling tools after all required dates are finished."
                .to_string(),
        );

        if !runtime_notes.is_empty() {
            system_parts.push(format!(
                "## Runtime Notes\n\n{}",
                runtime_notes.join("\n\n")
            ));
        }

        let system_content = system_parts.join("\n\n");

        let mut messages: Vec<serde_json::Value> = Vec::new();

        messages.push(json!({
            "role": "system",
            "content": system_content
        }));

        // For new tasks with empty conversation, inject onboarding guidance
        if let Some(ref task_id) = session.task_id {
            let user_msg_count = session.messages.iter().filter(|m| m.role == "user").count();
            if user_msg_count <= 1 {
                let task_spec = spec::load_spec(*task_id);
                if task_spec.goal.is_empty() {
                    system_parts.push(
                        "## 新任务引导\n\n\
                        这是一个新创建的任务，用户可能还不清楚如何描述需求。\n\
                        请主动引导用户，例如询问：\n\
                        - 这个任务想要实现什么功能？\n\
                        - 涉及哪些广告账户？\n\
                        - 是否需要定期执行？\n\
                        - 具体想要查看/操作什么数据？\n\n\
                        给出几个常见场景供用户选择会更友好。".to_string()
                    );
                }
            }
        }

        let history_start = session
            .messages
            .len()
            .saturating_sub(self.max_history * 2);
        for msg in session.messages[history_start..].iter() {
            messages.push(msg.to_openai_value());
        }

        json!(messages)
    }
}
