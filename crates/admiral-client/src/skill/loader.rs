//! Skill loader: scans skills/ directory, parses skill.toml, builds SkillDefinition.

use admiral_types::skill::{
    SkillApi, SkillCategory, SkillDefinition, SkillInputDef, SkillLlmTool, RiskLevel,
};
use std::path::{Path, PathBuf};
use tracing::warn;

/// TOML file format (matches existing skill.toml files).
#[derive(serde::Deserialize)]
struct SkillToml {
    skill: SkillSection,
}

#[derive(serde::Deserialize)]
struct SkillSection {
    id: String,
    name: String,
    category: String,
    #[serde(default)]
    platform: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    description: Option<SkillDescSection>,
    #[serde(default)]
    risk: Option<SkillRiskSection>,
    #[serde(default)]
    input: Option<SkillInputSection>,
    #[serde(default)]
    api: Option<SkillApiSection>,
    #[serde(default)]
    llm_tool: Option<SkillLlmToolSection>,
}

#[derive(serde::Deserialize)]
struct SkillDescSection {
    #[serde(default)]
    short: Option<String>,
    #[serde(default)]
    long: Option<String>,
}

#[derive(serde::Deserialize)]
struct SkillRiskSection {
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    requires_dry_run: bool,
    #[serde(default)]
    requires_test_run: bool,
    #[serde(default)]
    requires_user_confirm: bool,
    #[serde(default)]
    is_reversible: Option<bool>,
}

#[derive(serde::Deserialize)]
struct SkillInputSection {
    #[serde(default)]
    required: Vec<String>,
    #[serde(default)]
    optional: Vec<String>,
}

#[derive(serde::Deserialize)]
struct SkillApiSection {
    method: String,
    #[serde(default)]
    endpoint: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    rate_limit: Option<String>,
}

#[derive(serde::Deserialize)]
struct SkillLlmToolSection {
    name: String,
    description: String,
    #[serde(default)]
    parameters_schema: Option<serde_json::Value>,
}

fn parse_risk_level(s: &str) -> RiskLevel {
    match s.to_lowercase().as_str() {
        "low" => RiskLevel::Low,
        "medium" => RiskLevel::Medium,
        "high" => RiskLevel::High,
        "critical" => RiskLevel::Critical,
        _ => RiskLevel::Low,
    }
}

fn parse_category(s: &str) -> SkillCategory {
    match s.to_lowercase().as_str() {
        "semantic" => SkillCategory::Semantic,
        "tools" => SkillCategory::Tools,
        "mapi" => SkillCategory::Mapi,
        _ => SkillCategory::Tools,
    }
}

/// Returns the directory containing skill definitions (bundled with the app or in data dir).
pub fn skills_base_dir() -> PathBuf {
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(manifest).join("skills");
        if p.is_dir() {
            return p;
        }
    }
    crate::db::models::admiral_data_dir().join("skills")
}

/// Load a single skill from a directory containing skill.toml.
pub fn load_skill_from_dir(dir: &Path) -> anyhow::Result<SkillDefinition> {
    let toml_path = dir.join("skill.toml");
    let content = std::fs::read_to_string(&toml_path)
        .map_err(|e| anyhow::anyhow!("read {}: {}", toml_path.display(), e))?;
    let raw: SkillToml = toml::from_str(&content).map_err(|e| anyhow::anyhow!("parse {}: {}", toml_path.display(), e))?;

    let s = raw.skill;
    let category = parse_category(&s.category);
    let risk_level = s
        .risk
        .as_ref()
        .and_then(|r| r.level.as_deref())
        .map(parse_risk_level);
    let requires_dry_run = s.risk.as_ref().map(|r| r.requires_dry_run).unwrap_or(false);
    let requires_user_confirm = s.risk.as_ref().map(|r| r.requires_user_confirm).unwrap_or(false);
    let is_reversible = s.risk.as_ref().and_then(|r| r.is_reversible);

    let api = s.api.map(|a| SkillApi {
        method: a.method,
        path: a.endpoint.or(a.path).unwrap_or_else(|| "/".to_string()),
        rate_limit: a.rate_limit,
    });

    let input = s.input.map(|i| SkillInputDef {
        required: i.required,
        optional: i.optional,
    });

    let llm_tool = s.llm_tool.map(|t| SkillLlmTool {
        name: t.name,
        description: t.description,
        parameters_schema: t.parameters_schema,
    }).or_else(|| {
        let name = s.id.replace('.', "_");
        let desc = s.description.as_ref().and_then(|d| d.short.clone()).unwrap_or_else(|| s.name.clone());
        Some(SkillLlmTool {
            name,
            description: desc,
            parameters_schema: None,
        })
    });

    let prompt_md = std::fs::read_to_string(dir.join("prompt.md")).ok();

    Ok(SkillDefinition {
        id: s.id,
        name: s.name,
        category,
        platform: s.platform,
        version: s.version,
        description_short: s.description.as_ref().and_then(|d| d.short.clone()),
        description_long: s.description.as_ref().and_then(|d| d.long.clone()),
        risk_level,
        requires_dry_run,
        requires_user_confirm,
        is_reversible,
        api,
        input,
        output_description: None,
        llm_tool,
        prompt_md,
    })
}

/// Recursively find all directories that contain skill.toml under base.
fn find_skill_dirs(base: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.join("skill.toml").is_file() {
                    out.push(path);
                } else {
                    out.extend(find_skill_dirs(&path));
                }
            }
        }
    }
    out
}

/// Load all skills from the skills base directory.
pub fn load_all_skills(base: &Path) -> Vec<SkillDefinition> {
    let dirs = find_skill_dirs(base);
    let mut skills = Vec::with_capacity(dirs.len());
    for dir in dirs {
        match load_skill_from_dir(&dir) {
            Ok(def) => skills.push(def),
            Err(e) => warn!(dir = %dir.display(), error = %e, "skip skill"),
        }
    }
    skills
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapi_skills_have_open_api_paths() {
        let base = skills_base_dir();
        let skills = load_all_skills(&base);
        let by_id: std::collections::HashMap<_, _> = skills.iter().map(|s| (s.id.as_str(), s)).collect();
        let expected = [
            ("bili.campaign.list", "open_api/cpc/v3/campaign/list_campaigns"),
            ("bili.adgroup.list", "open_api/cpc/v3/unit/list_units"),
            ("bili.creative.list", "open_api/cpc/v3/creative/list_creatives"),
            ("bili.report.query", "open_api/report/v3/custom/query"),
        ];
        for (id, path) in &expected {
            let s = by_id.get(id).expect(&format!("skill {} should be loaded", id));
            let api_path = s.api.as_ref().map(|a| a.path.as_str()).unwrap_or("");
            assert_eq!(api_path, *path, "skill {} path", id);
        }
    }
}
