//! Agent profile loader: reads agent_profile.toml.

use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct AgentProfile {
    pub name: String,
    pub version: String,
    pub role: String,
    pub tone: String,
    pub language: String,
    /// System prompt built from rules and constraints.
    pub system_prompt: String,
    /// Must dry-run before write operations.
    pub must_dry_run: bool,
    pub must_test_run_before_deploy: bool,
    pub max_batch_size: u32,
    pub require_user_confirm_for_write: bool,
    pub default_model: Option<String>,
    pub planning_model: Option<String>,
    pub execution_model: Option<String>,
    pub temperature: Option<f32>,
}

fn profile_path() -> PathBuf {
    crate::db::models::admiral_data_dir().join("agent_profile.toml")
}

/// Load agent profile from disk. Returns default if file missing or invalid.
pub fn load_profile() -> AgentProfile {
    let path = profile_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return apply_client_llm_override(default_profile()),
    };
    parse_profile(&content)
        .map(apply_client_llm_override)
        .unwrap_or_else(|_| apply_client_llm_override(default_profile()))
}

fn default_profile() -> AgentProfile {
    let mut profile = AgentProfile {
        name: "Admiral".to_string(),
        version: "1.0.0".to_string(),
        role: "资深广告优化师助手".to_string(),
        tone: "专业、简洁、主动确认".to_string(),
        language: "zh-CN".to_string(),
        system_prompt: String::new(),
        must_dry_run: true,
        must_test_run_before_deploy: true,
        max_batch_size: 100,
        require_user_confirm_for_write: true,
        default_model: Some("deepseek-chat".to_string()),
        planning_model: None,
        execution_model: None,
        temperature: Some(0.1),
    };
    profile.system_prompt = build_system_prompt(&profile);
    profile
}

#[derive(serde::Deserialize)]
struct AgentToml {
    agent: Option<AgentSection>,
    personality: Option<PersonalitySection>,
    rules: Option<RulesSection>,
    llm: Option<LlmSection>,
}

#[derive(serde::Deserialize)]
struct AgentSection {
    name: Option<String>,
    version: Option<String>,
}

#[derive(serde::Deserialize)]
struct PersonalitySection {
    role: Option<String>,
    tone: Option<String>,
    language: Option<String>,
}

#[derive(serde::Deserialize)]
struct RulesSection {
    must_dry_run: Option<bool>,
    must_test_run_before_deploy: Option<bool>,
    max_batch_size: Option<u32>,
    require_user_confirm_for_write: Option<bool>,
}

#[derive(serde::Deserialize)]
struct LlmSection {
    default_model: Option<String>,
    planning_model: Option<String>,
    execution_model: Option<String>,
    temperature: Option<f32>,
}

fn parse_profile(content: &str) -> anyhow::Result<AgentProfile> {
    let raw: AgentToml = toml::from_str(content)?;
    let name = raw
        .agent
        .as_ref()
        .and_then(|a| a.name.clone())
        .unwrap_or_else(|| "Admiral".to_string());
    let version = raw
        .agent
        .as_ref()
        .and_then(|a| a.version.clone())
        .unwrap_or_else(|| "1.0.0".to_string());
    let role = raw
        .personality
        .as_ref()
        .and_then(|p| p.role.clone())
        .unwrap_or_else(|| "资深广告优化师助手".to_string());
    let tone = raw
        .personality
        .as_ref()
        .and_then(|p| p.tone.clone())
        .unwrap_or_else(|| "专业、简洁、主动确认".to_string());
    let language = raw
        .personality
        .as_ref()
        .and_then(|p| p.language.clone())
        .unwrap_or_else(|| "zh-CN".to_string());
    let must_dry_run = raw.rules.as_ref().and_then(|r| r.must_dry_run).unwrap_or(true);
    let must_test_run_before_deploy = raw
        .rules
        .as_ref()
        .and_then(|r| r.must_test_run_before_deploy)
        .unwrap_or(true);
    let max_batch_size = raw
        .rules
        .as_ref()
        .and_then(|r| r.max_batch_size)
        .unwrap_or(100);
    let require_user_confirm_for_write = raw
        .rules
        .as_ref()
        .and_then(|r| r.require_user_confirm_for_write)
        .unwrap_or(true);
    let default_model = raw.llm.as_ref().and_then(|l| l.default_model.clone());
    let planning_model = raw.llm.as_ref().and_then(|l| l.planning_model.clone());
    let execution_model = raw.llm.as_ref().and_then(|l| l.execution_model.clone());
    let temperature = raw.llm.as_ref().and_then(|l| l.temperature);

    let mut profile = AgentProfile {
        name,
        version,
        role,
        tone,
        language,
        system_prompt: String::new(),
        must_dry_run,
        must_test_run_before_deploy,
        max_batch_size,
        require_user_confirm_for_write,
        default_model,
        planning_model,
        execution_model,
        temperature,
    };
    profile.system_prompt = build_system_prompt(&profile);
    Ok(profile)
}

fn apply_client_llm_override(mut profile: AgentProfile) -> AgentProfile {
    let selection = crate::config::ClientConfig::from_env().llm_selection();
    profile.default_model = Some(selection.model.clone());
    profile.planning_model = Some(selection.model.clone());
    profile.execution_model = Some(selection.model);
    profile
}

fn build_system_prompt(p: &AgentProfile) -> String {
    format!(
        r#"You are {name}, version {version}.
Role: {role}.
Tone: {tone}.
Language: {language}.

## 核心工作模式

你采用「对话驱动」模式帮助用户定义和完善任务：
1. 任务创建后，你主动询问用户想要实现什么功能
2. 通过持续对话收集信息，逐步完善任务规格
3. 信息足够时，建议用户进行试运行来验证配置
4. 试运行后根据结果继续调整，直到任务就绪
5. 每次执行后提供完整的执行报告

## 对话策略

- 每次回复时，评估当前缺失的关键信息，主动追问
- 不要一次问太多问题，每轮聚焦 1-2 个最关键的信息
- 当用户的描述比较模糊时，给出具体的选项帮助选择
- 当信息基本完整时，给出任务配置摘要让用户确认
- 用户也可能只是咨询问题或闲聊，正常回答即可

## 操作规则

- Write operations (create/update/delete ads) must go through dry-run first.
- Before deploying a task, test-run is required.
- Maximum batch size for bulk operations: {max_batch}.
- Require user confirmation for write operations.
- When the user does not specify account_id: you MUST call bili_account_list_local first (this tool reads local bindings only, no MAPI). Do not call any other tool that requires account_id (e.g. report, campaign list, unit list) until you have account_id from this list. If 0 results, prompt the user to authorize and sync accounts. If 1 result, use that account_id. If multiple, let the user choose (e.g. by account_name).

Respond in {language}."#,
        name = p.name,
        version = p.version,
        role = p.role,
        tone = p.tone,
        language = p.language,
        max_batch = p.max_batch_size,
    )
}
