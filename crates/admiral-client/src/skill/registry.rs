//! Skill registry: stores loaded skills, provides lookup by ID and LLM tool list.

use admiral_types::skill::{LlmFunctionDef, LlmToolDefinition, SkillDefinition};
use std::collections::HashMap;
use crate::skill::loader;
use std::sync::RwLock;

/// Registry of all loaded skills (thread-safe).
pub struct SkillRegistry {
    by_id: RwLock<HashMap<String, SkillDefinition>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            by_id: RwLock::new(HashMap::new()),
        }
    }

    /// Load all skills from the default skills directory and register them.
    pub fn load_from_default_dir(&self) -> anyhow::Result<usize> {
        let base = loader::skills_base_dir();
        let skills = loader::load_all_skills(&base);
        let count = skills.len();
        let mut map = self.by_id.write().map_err(|e| anyhow::anyhow!("lock: {}", e))?;
        for def in skills {
            map.insert(def.id.clone(), def);
        }
        Ok(count)
    }

    /// Get a skill by id.
    pub fn get(&self, id: &str) -> Option<SkillDefinition> {
        self.by_id.read().ok()?.get(id).cloned()
    }

    /// List all registered skill ids.
    pub fn list_ids(&self) -> Vec<String> {
        self.by_id
            .read()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// List all registered skills.
    pub fn list(&self) -> Vec<SkillDefinition> {
        self.by_id
            .read()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Build OpenAI/Anthropic compatible tool definitions for LLM context.
    pub fn build_llm_tools(&self) -> Vec<LlmToolDefinition> {
        self.by_id
            .read()
            .map(|m| {
                m.values()
                    .filter_map(|def| {
                        def.llm_tool.as_ref().map(|t| LlmToolDefinition {
                            r#type: "function".to_string(),
                            function: LlmFunctionDef {
                                name: t.name.clone(),
                                description: t.description.clone(),
                                parameters: t.parameters_schema.clone(),
                            },
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}
