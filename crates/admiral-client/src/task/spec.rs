//! Task specification document: structured Markdown representing the task's goals,
//! context, implementation path, and run history. Persisted as TASK_SPEC.md.

use crate::agent::memory;
use uuid::Uuid;

/// In-memory representation of a task specification document.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TaskSpec {
    pub goal: String,
    pub context: SpecContext,
    pub steps: Vec<SpecStep>,
    pub config_params: String,
    pub checklist: Vec<ChecklistItem>,
    pub run_summaries: Vec<RunSummary>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SpecContext {
    pub platform: String,
    pub accounts: Vec<String>,
    pub trigger_description: String,
    pub extra: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SpecStep {
    pub seq: u32,
    pub description: String,
    pub skill_id: Option<String>,
    pub params_hint: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ChecklistItem {
    pub label: String,
    pub done: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RunSummary {
    pub date: String,
    pub run_type: String,
    pub result: String,
    pub notes: String,
}

impl TaskSpec {
    pub fn to_markdown(&self, task_name: &str) -> String {
        let mut md = format!("# 任务：{}\n\n", task_name);

        md.push_str("## 目标\n\n");
        if self.goal.is_empty() {
            md.push_str("（待定义）\n");
        } else {
            md.push_str(&self.goal);
            md.push('\n');
        }

        md.push_str("\n## 上下文\n\n");
        if !self.context.platform.is_empty() {
            md.push_str(&format!("- 平台：{}\n", self.context.platform));
        }
        if !self.context.accounts.is_empty() {
            md.push_str(&format!("- 关联账户：{}\n", self.context.accounts.join(", ")));
        }
        if !self.context.trigger_description.is_empty() {
            md.push_str(&format!("- 触发方式：{}\n", self.context.trigger_description));
        }
        if !self.context.extra.is_empty() {
            md.push_str(&format!("- 补充信息：{}\n", self.context.extra));
        }

        if !self.steps.is_empty() {
            md.push_str("\n## 实现路径\n\n");
            for step in &self.steps {
                md.push_str(&format!("### 步骤 {}: {}\n", step.seq, step.description));
                if let Some(ref sid) = step.skill_id {
                    md.push_str(&format!("- 技能：{}\n", sid));
                }
                if !step.params_hint.is_empty() {
                    md.push_str(&format!("- 参数：{}\n", step.params_hint));
                }
                md.push('\n');
            }
        }

        if !self.config_params.is_empty() {
            md.push_str("## 配置参数\n\n");
            md.push_str(&self.config_params);
            md.push('\n');
        }

        if !self.checklist.is_empty() {
            md.push_str("\n## 信息完整度评估\n\n");
            for item in &self.checklist {
                let check = if item.done { "x" } else { " " };
                md.push_str(&format!("- [{}] {}\n", check, item.label));
            }
        }

        if !self.run_summaries.is_empty() {
            md.push_str("\n## 运行历史摘要\n\n");
            for run in &self.run_summaries {
                md.push_str(&format!("### {} {}\n", run.date, run.run_type));
                md.push_str(&format!("- 结果：{}\n", run.result));
                if !run.notes.is_empty() {
                    md.push_str(&format!("- 备注：{}\n", run.notes));
                }
                md.push('\n');
            }
        }

        md
    }

    /// Parse a TASK_SPEC.md back into a TaskSpec (best-effort, tolerant of manual edits).
    pub fn from_markdown(md: &str) -> Self {
        let mut spec = TaskSpec::default();
        let mut current_section = "";
        let mut current_step_seq = 0u32;
        let mut buf = String::new();

        for line in md.lines() {
            if line.starts_with("## 目标") {
                flush_section(&mut spec, current_section, &buf);
                current_section = "goal";
                buf.clear();
            } else if line.starts_with("## 上下文") {
                flush_section(&mut spec, current_section, &buf);
                current_section = "context";
                buf.clear();
            } else if line.starts_with("## 实现路径") {
                flush_section(&mut spec, current_section, &buf);
                current_section = "steps";
                buf.clear();
            } else if line.starts_with("## 配置参数") {
                flush_section(&mut spec, current_section, &buf);
                current_section = "config";
                buf.clear();
            } else if line.starts_with("## 信息完整度评估") {
                flush_section(&mut spec, current_section, &buf);
                current_section = "checklist";
                buf.clear();
            } else if line.starts_with("## 运行历史摘要") {
                flush_section(&mut spec, current_section, &buf);
                current_section = "history";
                buf.clear();
            } else if current_section == "steps" && line.starts_with("### 步骤") {
                current_step_seq += 1;
                let desc = line.trim_start_matches("### 步骤")
                    .trim_start_matches(|c: char| c.is_ascii_digit() || c == ':' || c == ' ');
                spec.steps.push(SpecStep {
                    seq: current_step_seq,
                    description: desc.trim().to_string(),
                    skill_id: None,
                    params_hint: String::new(),
                });
            } else if current_section == "steps" && line.starts_with("- 技能：") {
                if let Some(step) = spec.steps.last_mut() {
                    step.skill_id = Some(line.trim_start_matches("- 技能：").trim().to_string());
                }
            } else if current_section == "steps" && line.starts_with("- 参数：") {
                if let Some(step) = spec.steps.last_mut() {
                    step.params_hint = line.trim_start_matches("- 参数：").trim().to_string();
                }
            } else if current_section == "checklist" && line.starts_with("- [") {
                let done = line.contains("[x]");
                let label = line.trim_start_matches("- [x] ")
                    .trim_start_matches("- [ ] ")
                    .trim()
                    .to_string();
                spec.checklist.push(ChecklistItem { label, done });
            } else if current_section == "context" && line.starts_with("- 平台：") {
                spec.context.platform = line.trim_start_matches("- 平台：").trim().to_string();
            } else if current_section == "context" && line.starts_with("- 关联账户：") {
                spec.context.accounts = line.trim_start_matches("- 关联账户：")
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
            } else if current_section == "context" && line.starts_with("- 触发方式：") {
                spec.context.trigger_description = line.trim_start_matches("- 触发方式：").trim().to_string();
            } else {
                buf.push_str(line);
                buf.push('\n');
            }
        }
        flush_section(&mut spec, current_section, &buf);
        spec
    }
}

fn flush_section(spec: &mut TaskSpec, section: &str, buf: &str) {
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        return;
    }
    match section {
        "goal" => spec.goal = trimmed.to_string(),
        "config" => spec.config_params = trimmed.to_string(),
        "context" => {
            if !spec.context.extra.is_empty() {
                spec.context.extra.push('\n');
            }
            spec.context.extra.push_str(trimmed);
        }
        _ => {}
    }
}

/// Load the task spec from disk. Returns a default empty spec if the file doesn't exist.
pub fn load_spec(task_id: Uuid) -> TaskSpec {
    let store = match memory::MemoryStore::new() {
        Ok(s) => s,
        Err(_) => return TaskSpec::default(),
    };
    match store.load_task_spec(task_id) {
        Ok(content) if !content.is_empty() => TaskSpec::from_markdown(&content),
        _ => TaskSpec::default(),
    }
}

/// Save the task spec to disk as Markdown.
pub fn save_spec(task_id: Uuid, task_name: &str, spec: &TaskSpec) -> std::io::Result<()> {
    let store = memory::MemoryStore::new()?;
    let md = spec.to_markdown(task_name);
    store.write_task_spec(task_id, &md)
}
