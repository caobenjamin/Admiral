//! Skill definition, SkillCall, SkillResult and LLM tool definitions.

use serde::{Deserialize, Serialize};

/// Skill category: semantic (LLM-only), tools (local), mapi (remote via server).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillCategory {
    Semantic,
    Tools,
    Mapi,
}

/// Risk level for a skill (affects dry-run / user confirm requirements).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// API definition for MAPI skills (method + path).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillApi {
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub rate_limit: Option<String>,
}

/// Input parameter definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillInputDef {
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub optional: Vec<String>,
}

/// LLM tool definition (name + description for function calling).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillLlmTool {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub parameters_schema: Option<serde_json::Value>,
}

/// Full skill definition loaded from skill.toml.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub id: String,
    pub name: String,
    pub category: SkillCategory,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub version: Option<String>,

    #[serde(default)]
    pub description_short: Option<String>,
    #[serde(default)]
    pub description_long: Option<String>,

    #[serde(default)]
    pub risk_level: Option<RiskLevel>,
    #[serde(default)]
    pub requires_dry_run: bool,
    #[serde(default)]
    pub requires_user_confirm: bool,
    #[serde(default)]
    pub is_reversible: Option<bool>,

    #[serde(default)]
    pub api: Option<SkillApi>,
    #[serde(default)]
    pub input: Option<SkillInputDef>,
    #[serde(default)]
    pub llm_tool: Option<SkillLlmTool>,

    /// Path to optional prompt.md content (loaded separately).
    #[serde(skip)]
    pub prompt_md: Option<String>,
}

/// A single skill invocation (from LLM or task step).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillCall {
    pub skill_id: String,
    pub params: serde_json::Value,
    #[serde(default)]
    pub account_id: Option<i64>,
    #[serde(default)]
    pub dry_run: bool,
}

/// Result of executing a skill.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillResult {
    pub success: bool,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

impl SkillResult {
    pub fn ok(data: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            duration_ms: Some(duration_ms),
        }
    }

    pub fn err(message: String, duration_ms: Option<u64>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            duration_ms,
        }
    }
}

/// OpenAI/Anthropic compatible function definition for LLM context.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmToolDefinition {
    pub r#type: String, // "function"
    pub function: LlmFunctionDef,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmFunctionDef {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}
