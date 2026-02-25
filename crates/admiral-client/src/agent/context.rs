//! Context assembler: agent profile + session history + memory + available skills.

use crate::agent::memory::MemoryStore;
use crate::agent::profile::AgentProfile;
use crate::skill::registry::SkillRegistry;
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

/// Builds the full message list for an LLM call: system + user memory + task memory + tools + history + current input.
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

    /// Build the messages array for the LLM API (system content is merged into first user message or a system message).
    pub fn build(
        &self,
        session: &Session,
        user_message: &str,
        knowledge_snippets: &[String],
    ) -> serde_json::Value {
        let mut system_parts = vec![self.profile.system_prompt.clone()];

        if let Ok(user_mem) = self.memory.load_user_memory() {
            if !user_mem.is_empty() {
                system_parts.push(format!("## User Memory\n\n{}", user_mem));
            }
        }

        if let Some(ref task_id) = session.task_id {
            if let Ok(task_mem) = self.memory.load_task_memory(*task_id) {
                if !task_mem.is_empty() {
                    system_parts.push(format!("## Task Memory\n\n{}", task_mem));
                }
            }
        }

        if !knowledge_snippets.is_empty() {
            system_parts.push(
                "## Relevant Knowledge\n\n".to_string()
                    + &knowledge_snippets.join("\n\n"),
            );
        }

        let tools = self.skill_registry.build_llm_tools();
        if !tools.is_empty() {
            system_parts.push(
                "## Available Tools\n\nYou may call these functions when needed. Use the exact function names."
                    .to_string(),
            );
        }

        let system_content = system_parts.join("\n\n");

        let mut messages: Vec<serde_json::Value> = Vec::new();

        messages.push(json!({
            "role": "system",
            "content": system_content
        }));

        if !tools.is_empty() {
            messages.push(json!({
                "role": "system",
                "content": serde_json::to_string(&tools).unwrap_or_default()
            }));
        }

        let history_start = session
            .messages
            .len()
            .saturating_sub(self.max_history * 2);
        for msg in session.messages[history_start..].iter() {
            messages.push(msg.to_openai_value());
        }

        messages.push(json!({
            "role": "user",
            "content": user_message
        }));

        json!(messages)
    }
}
