//! Agent core interaction loop with explicit observe -> decide -> act state.

use crate::agent::context::{ContextAssembler, Message, Session};
use crate::agent::memory::MemoryStore;
use crate::agent::profile;
use crate::db;
use crate::network::api_client::ApiClient;
use admiral_types::api::LlmChatRequest;
use admiral_types::skill::{SkillCall, SkillResult};
use chrono::{Datelike, NaiveDate};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunStatus {
    Queued,
    Running,
    WaitingTool,
    WaitingUser,
    Retrying,
    Paused,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStepStatus {
    Pending,
    Running,
    WaitingTool,
    Retrying,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentStepState {
    pub step: u32,
    pub title: String,
    pub status: AgentStepStatus,
    pub action_type: Option<String>,
    pub tool_name: Option<String>,
    pub tool_arguments: Option<Value>,
    pub tool_success: Option<bool>,
    pub tool_result: Option<Value>,
    pub message: Option<String>,
    pub retry_count: u32,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentRunState {
    pub run_id: String,
    pub task_id: Option<String>,
    pub session_id: String,
    pub status: AgentRunStatus,
    pub current_step: u32,
    pub total_retries: u32,
    pub max_steps: u32,
    pub steps: Vec<AgentStepState>,
    pub final_message: Option<String>,
    pub waiting_for_user: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "delta")]
    Delta { content: String },
    #[serde(rename = "status")]
    Status {
        stage: String,
        message: String,
        level: Option<String>,
    },
    #[serde(rename = "tool_use")]
    ToolUse { name: String },
    #[serde(rename = "tool_result")]
    ToolResult { name: String, success: bool },
    #[serde(rename = "run_state")]
    RunState { run: AgentRunState },
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "error")]
    Error { message: String },
}

#[derive(Clone, Debug, Serialize)]
pub struct AgentResponse {
    pub content: String,
    pub tool_calls_used: Vec<(String, SkillResult)>,
    pub session: Session,
    pub run: AgentRunState,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AgentAction {
    CallTool {
        tool_name: String,
        #[serde(default)]
        arguments: Value,
        #[serde(default)]
        reason: Option<String>,
    },
    Respond {
        message: String,
    },
    AskUser {
        message: String,
    },
}

#[derive(Clone, Debug)]
struct SseOutcome {
    content: String,
    first_chunk_seen: bool,
}

#[derive(Clone, Debug)]
struct MonthlyReportPlan {
    expected_dates: Vec<String>,
    completed_dates: BTreeSet<String>,
}

impl MonthlyReportPlan {
    fn describe_progress(&self) -> String {
        format!(
            "Explicit stop condition for daily report fetch: completed {}/{} required dates. Remaining dates: {}.",
            self.completed_dates.len(),
            self.expected_dates.len(),
            self.expected_dates
                .iter()
                .filter(|d| !self.completed_dates.contains(*d))
                .take(8)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn is_complete(&self) -> bool {
        self.completed_dates.len() >= self.expected_dates.len()
    }
}

async fn emit_status(tx: &mpsc::Sender<StreamEvent>, stage: &str, message: impl Into<String>, level: Option<&str>) {
    let _ = tx
        .send(StreamEvent::Status {
            stage: stage.to_string(),
            message: message.into(),
            level: level.map(|s| s.to_string()),
        })
        .await;
}

pub struct AgentRuntime {
    context: ContextAssembler,
    api_client: Arc<ApiClient>,
    skill_executor: Arc<crate::skill::executor::SkillExecutor>,
    run_store: Arc<RwLock<HashMap<String, AgentRunState>>>,
}

impl AgentRuntime {
    pub fn new(
        api_client: Arc<ApiClient>,
        skill_registry: Arc<crate::skill::registry::SkillRegistry>,
    ) -> anyhow::Result<Self> {
        let profile = profile::load_profile();
        let memory = MemoryStore::new().map_err(|e| anyhow::anyhow!("memory: {}", e))?;
        let context = ContextAssembler::new(profile, memory, skill_registry.clone());
        let skill_executor = Arc::new(crate::skill::executor::SkillExecutor::new(
            skill_registry,
            api_client.clone(),
        ));
        Ok(Self {
            context,
            api_client,
            skill_executor,
            run_store: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn process_message(
        &self,
        session: Session,
        user_message: &str,
        access_token: Option<&str>,
    ) -> Result<AgentResponse, anyhow::Error> {
        let (tx, _rx) = mpsc::channel(1);
        self.process_message_stream(session, user_message, access_token, tx).await
    }

    pub async fn process_message_stream(
        &self,
        mut session: Session,
        user_message: &str,
        access_token: Option<&str>,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<AgentResponse, anyhow::Error> {
        session.messages.push(Message {
            role: "user".to_string(),
            content: Some(user_message.to_string()),
            tool_calls: None,
            name: None,
            tool_call_id: None,
        });

        let token = access_token.ok_or_else(|| anyhow::anyhow!("access_token required for LLM"))?;
        let knowledge_snippets: Vec<String> = self
            .api_client
            .knowledge_query(token, user_message, 5)
            .await
            .map(|s| s.into_iter().map(|k| k.content).collect())
            .unwrap_or_default();

        let run_id = Uuid::new_v4().to_string();
        let max_steps = 12_u32;
        let mut current_session = session;
        let mut tool_calls_used: Vec<(String, SkillResult)> = Vec::new();
        let mut runtime_notes: Vec<String> = Vec::new();
        let mut monthly_plan = infer_monthly_report_plan(user_message);
        let mut run = AgentRunState {
            run_id: run_id.clone(),
            task_id: current_session.task_id.map(|id| id.to_string()),
            session_id: current_session.id.clone(),
            status: AgentRunStatus::Running,
            current_step: 0,
            total_retries: 0,
            max_steps,
            steps: Vec::new(),
            final_message: None,
            waiting_for_user: None,
        };
        self.publish_run_state(&tx, &run, Some("run_started"), Some(json!({"user_message": user_message}))).await;

        let mut repeated_tool_signatures: HashMap<String, u32> = HashMap::new();

        for step_no in 1..=max_steps {
            run.current_step = step_no;
            run.steps.push(AgentStepState {
                step: step_no,
                title: format!("Step {}", step_no),
                status: AgentStepStatus::Running,
                action_type: None,
                tool_name: None,
                tool_arguments: None,
                tool_success: None,
                tool_result: None,
                message: Some("observe -> decide".to_string()),
                retry_count: 0,
                error: None,
            });
            self.publish_run_state(
                &tx,
                &run,
                Some("step_started"),
                Some(json!({"step": step_no})),
            )
            .await;

            if let Some(plan) = monthly_plan.as_ref() {
                runtime_notes.push(plan.describe_progress());
            }

            let action = match self
                .decide_next_action(
                    &current_session,
                    &knowledge_snippets,
                    &runtime_notes,
                    token,
                    &tx,
                    &mut run,
                )
                .await
            {
                Ok(action) => action,
                Err(err) => {
                    mark_step_failed(&mut run, step_no, err.to_string());
                    run.status = AgentRunStatus::Failed;
                    run.final_message = Some(err.to_string());
                    self.publish_run_state(
                        &tx,
                        &run,
                        Some("decision_failed"),
                        Some(json!({"step": step_no, "error": err.to_string()})),
                    )
                    .await;
                    let _ = tx.send(StreamEvent::Error { message: err.to_string() }).await;
                    let _ = tx.send(StreamEvent::Done).await;
                    return Ok(AgentResponse {
                        content: err.to_string(),
                        tool_calls_used,
                        session: current_session,
                        run,
                    });
                }
            };

            match action {
                AgentAction::Respond { message } => {
                    current_session.messages.push(Message {
                        role: "assistant".to_string(),
                        content: Some(message.clone()),
                        tool_calls: None,
                        name: None,
                        tool_call_id: None,
                    });
                    mark_step_terminal(&mut run, step_no, "respond", Some(message.clone()));
                    run.status = AgentRunStatus::Succeeded;
                    run.final_message = Some(message.clone());
                    self.publish_run_state(
                        &tx,
                        &run,
                        Some("run_succeeded"),
                        Some(json!({"step": step_no, "message": message.clone()})),
                    )
                    .await;
                    let _ = tx.send(StreamEvent::Done).await;
                    return Ok(AgentResponse {
                        content: message,
                        tool_calls_used,
                        session: current_session,
                        run,
                    });
                }
                AgentAction::AskUser { message } => {
                    current_session.messages.push(Message {
                        role: "assistant".to_string(),
                        content: Some(message.clone()),
                        tool_calls: None,
                        name: None,
                        tool_call_id: None,
                    });
                    mark_step_terminal(&mut run, step_no, "ask_user", Some(message.clone()));
                    run.status = AgentRunStatus::WaitingUser;
                    run.waiting_for_user = Some(message.clone());
                    self.publish_run_state(
                        &tx,
                        &run,
                        Some("waiting_for_user"),
                        Some(json!({"step": step_no, "message": message.clone()})),
                    )
                    .await;
                    let _ = tx.send(StreamEvent::Done).await;
                    return Ok(AgentResponse {
                        content: message,
                        tool_calls_used,
                        session: current_session,
                        run,
                    });
                }
                AgentAction::CallTool {
                    tool_name,
                    arguments,
                    reason,
                } => {
                    if let Some(step) = run.steps.last_mut() {
                        step.action_type = Some("call_tool".to_string());
                        step.tool_name = Some(tool_name.clone());
                        step.tool_arguments = Some(arguments.clone());
                        step.message = reason.clone();
                    }
                    run.status = AgentRunStatus::WaitingTool;
                    self.publish_run_state(
                        &tx,
                        &run,
                        Some("tool_requested"),
                        Some(json!({"step": step_no, "tool_name": tool_name.clone(), "arguments": arguments.clone()})),
                    )
                    .await;
                    run.status = AgentRunStatus::Running;

                    let tool_signature = format!("{}:{}", tool_name, canonical_json(&arguments));
                    let signature_count = repeated_tool_signatures.entry(tool_signature.clone()).or_insert(0);
                    *signature_count += 1;
                    if *signature_count > 2 {
                        let message = format!("tool loop stopped: repeated identical call `{}`", tool_signature);
                        mark_step_failed(&mut run, step_no, message.clone());
                        run.status = AgentRunStatus::Failed;
                        run.final_message = Some(message.clone());
                        self.publish_run_state(
                            &tx,
                            &run,
                            Some("termination_triggered"),
                            Some(json!({"step": step_no, "reason": message.clone()})),
                        )
                        .await;
                        let _ = tx.send(StreamEvent::Error { message: message.clone() }).await;
                        let _ = tx.send(StreamEvent::Done).await;
                        return Ok(AgentResponse {
                            content: message,
                            tool_calls_used,
                            session: current_session,
                            run,
                        });
                    }

                    let tool_call_id = format!("call-{}", Uuid::new_v4());
                    current_session.messages.push(Message {
                        role: "assistant".to_string(),
                        content: None,
                        tool_calls: Some(json!([{
                            "id": tool_call_id,
                            "type": "function",
                            "function": {
                                "name": tool_name,
                                "arguments": canonical_json(&arguments),
                            }
                        }])),
                        name: None,
                        tool_call_id: None,
                    });

                    let result = self
                        .execute_tool_with_retry(
                            &tool_name,
                            &arguments,
                            token,
                            &tx,
                            &mut run,
                            step_no,
                        )
                        .await;

                    let result_content = if result.success {
                        serde_json::to_string(&result.data).unwrap_or_else(|_| "{}".to_string())
                    } else {
                        result.error.clone().unwrap_or_else(|| "unknown error".to_string())
                    };
                    current_session.messages.push(Message {
                        role: "tool".to_string(),
                        content: Some(result_content),
                        tool_calls: None,
                        name: Some(tool_name.clone()),
                        tool_call_id: Some(tool_call_id),
                    });

                    if let Some(step) = run.steps.last_mut() {
                        step.tool_success = Some(result.success);
                        step.tool_result = result.data.clone();
                    }

                    if result.success {
                        let skill_id = resolve_tool_name_to_skill_id(&tool_name, self.context.skill_registry.as_ref())
                            .unwrap_or_else(|| tool_name.clone());
                        tool_calls_used.push((skill_id, result.clone()));
                        if let Some(plan) = monthly_plan.as_mut() {
                            maybe_mark_report_date_completed(plan, &tool_name, &arguments);
                            runtime_notes.push(plan.describe_progress());
                            if plan.is_complete() {
                                runtime_notes.push(
                                    "Explicit stop condition reached: all required daily report dates are fetched. Next action must be `respond`, not another tool call."
                                        .to_string(),
                                );
                            }
                        }
                        if let Some(step) = run.steps.last_mut() {
                            step.status = AgentStepStatus::Succeeded;
                        }
                    } else {
                        let error_message = result
                            .error
                            .clone()
                            .unwrap_or_else(|| "tool execution failed".to_string());
                        if let Some(step) = run.steps.last_mut() {
                            step.status = AgentStepStatus::Failed;
                            step.error = Some(error_message.clone());
                        }
                        runtime_notes.push(format!(
                            "Latest tool failure for recovery: tool `{}` failed with `{}`. Either retry with corrected arguments or ask the user for missing information.",
                            tool_name, error_message
                        ));
                    }

                    let _ = tx
                        .send(StreamEvent::ToolResult {
                            name: tool_name.clone(),
                            success: result.success,
                        })
                        .await;
                    self.publish_run_state(
                        &tx,
                        &run,
                        Some(if result.success { "tool_succeeded" } else { "tool_failed" }),
                        Some(json!({
                            "step": step_no,
                            "tool_name": tool_name,
                            "success": result.success
                        })),
                    )
                    .await;
                }
            }
        }

        let message = "达到最大步骤数，已停止自动执行。".to_string();
        run.status = AgentRunStatus::Failed;
        run.final_message = Some(message.clone());
        self.publish_run_state(
            &tx,
            &run,
            Some("run_failed"),
            Some(json!({"reason": message.clone()})),
        )
        .await;
        let _ = tx.send(StreamEvent::Done).await;
        Ok(AgentResponse {
            content: message,
            tool_calls_used,
            session: current_session,
            run,
        })
    }

    async fn publish_run_state(
        &self,
        tx: &mpsc::Sender<StreamEvent>,
        run: &AgentRunState,
        event_type: Option<&str>,
        payload: Option<Value>,
    ) {
        self.run_store
            .write()
            .await
            .insert(run.run_id.clone(), run.clone());
        persist_run_state(run);
        if let Some(event_type) = event_type {
            persist_run_event(run, event_type, payload);
        }
        let _ = tx.send(StreamEvent::RunState { run: run.clone() }).await;
    }

    async fn decide_next_action(
        &self,
        session: &Session,
        knowledge_snippets: &[String],
        runtime_notes: &[String],
        access_token: &str,
        tx: &mpsc::Sender<StreamEvent>,
        run: &mut AgentRunState,
    ) -> anyhow::Result<AgentAction> {
        let max_retries = 3_u32;

        for attempt in 0..max_retries {
            if attempt > 0 {
                run.total_retries += 1;
                if let Some(step) = run.steps.last_mut() {
                    step.status = AgentStepStatus::Retrying;
                    step.retry_count = attempt;
                    step.error = Some("retrying LLM decision".to_string());
                }
                run.status = AgentRunStatus::Retrying;
                self.publish_run_state(
                    tx,
                    run,
                    Some("decision_retry"),
                    Some(json!({"step": run.current_step, "attempt": attempt})),
                )
                .await;
                run.status = AgentRunStatus::Running;
            }

            emit_status(
                tx,
                "llm_request_start",
                format!("开始第 {} 步决策，请求模型结构化动作。", run.current_step),
                Some("info"),
            )
            .await;

            let messages = self.context.build(session, knowledge_snippets, runtime_notes);
            let req = LlmChatRequest {
                messages,
                stream: true,
                model: self.context.profile.default_model.clone(),
                max_tokens: Some(2048),
                tools: None,
                tool_choice: None,
            };

            let resp = self
                .api_client
                .llm_chat_stream(access_token, &req)
                .await
                .map_err(|e| anyhow::anyhow!("llm_chat_stream: {}", e))?;
            let outcome = parse_sse_stream(resp).await?;

            if !outcome.first_chunk_seen {
                emit_status(tx, "llm_no_chunks", "SSE 已结束，但未收到任何数据分片。准备自动重试。", Some("warn")).await;
                continue;
            }
            if outcome.content.trim().is_empty() {
                emit_status(tx, "empty_assistant_reply", "模型回复为空，准备自动重试。", Some("warn")).await;
                continue;
            }

            match parse_action(&outcome.content) {
                Ok(action) => {
                    return Ok(action);
                }
                Err(err) => {
                    emit_status(
                        tx,
                        "invalid_action",
                        format!("模型未返回合法结构化动作：{}。准备自动重试。", err),
                        Some("warn"),
                    )
                    .await;
                }
            }
        }

        Err(anyhow::anyhow!("模型连续多次未返回可执行动作"))
    }

    async fn execute_tool_with_retry(
        &self,
        tool_name: &str,
        arguments: &Value,
        access_token: &str,
        tx: &mpsc::Sender<StreamEvent>,
        run: &mut AgentRunState,
        step_no: u32,
    ) -> SkillResult {
        let skill_id = match resolve_tool_name_to_skill_id(tool_name, self.context.skill_registry.as_ref()) {
            Some(skill_id) => skill_id,
            None => {
                return SkillResult::err(format!("unknown tool {}", tool_name), None);
            }
        };

        let max_retries = 2_u32;
        let _ = tx
            .send(StreamEvent::ToolUse {
                name: tool_name.to_string(),
            })
            .await;

        for attempt in 0..=max_retries {
            if attempt > 0 {
                run.total_retries += 1;
                if let Some(step) = run.steps.last_mut() {
                    step.status = AgentStepStatus::Retrying;
                    step.retry_count = step.retry_count.max(attempt);
                    step.error = Some(format!("retrying tool `{}`", tool_name));
                }
                run.status = AgentRunStatus::Retrying;
                self.publish_run_state(
                    tx,
                    run,
                    Some("tool_retry"),
                    Some(json!({"step": step_no, "attempt": attempt, "tool_name": tool_name})),
                )
                .await;
                run.status = AgentRunStatus::Running;
            }

            let call = SkillCall {
                skill_id: skill_id.clone(),
                params: arguments.clone(),
                account_id: arguments.get("account_id").and_then(|v| v.as_i64()),
                dry_run: false,
            };

            let result = self
                .skill_executor
                .execute(call, Some(access_token))
                .await
                .unwrap_or_else(|e| SkillResult::err(e.to_string(), None));

            if result.success || attempt == max_retries {
                return result;
            }

            emit_status(
                tx,
                "tool_retry",
                format!(
                    "步骤 {} 的工具 `{}` 执行失败，准备第 {} 次重试。",
                    step_no,
                    tool_name,
                    attempt + 1
                ),
                Some("warn"),
            )
            .await;
        }

        SkillResult::err(format!("tool `{}` exhausted retries", tool_name), None)
    }
}

fn mark_step_failed(run: &mut AgentRunState, step_no: u32, error: String) {
    if let Some(step) = run.steps.iter_mut().find(|s| s.step == step_no) {
        step.status = AgentStepStatus::Failed;
        step.error = Some(error);
    }
}

fn mark_step_terminal(run: &mut AgentRunState, step_no: u32, action_type: &str, message: Option<String>) {
    if let Some(step) = run.steps.iter_mut().find(|s| s.step == step_no) {
        step.status = AgentStepStatus::Succeeded;
        step.action_type = Some(action_type.to_string());
        step.message = message;
    }
}

fn parse_action(raw: &str) -> anyhow::Result<AgentAction> {
    let trimmed = raw.trim();
    let unwrapped = trimmed
        .strip_prefix("```json")
        .and_then(|s| s.strip_suffix("```"))
        .or_else(|| trimmed.strip_prefix("```").and_then(|s| s.strip_suffix("```")))
        .map(|s| s.trim())
        .unwrap_or(trimmed);
    serde_json::from_str(unwrapped).map_err(|e| anyhow::anyhow!("parse action json: {}", e))
}

fn canonical_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
}

async fn parse_sse_stream(resp: reqwest::Response) -> Result<SseOutcome, anyhow::Error> {
    let mut content = String::new();
    let mut buffer = String::new();
    let mut first_chunk_seen = false;

    let mut stream = resp.bytes_stream();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| anyhow::anyhow!("stream read: {}", e))?;
        if !first_chunk_seen {
            first_chunk_seen = true;
        }
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        process_sse_buffer(&mut buffer, &mut content);
    }

    if !buffer.trim().is_empty() {
        buffer.push('\n');
        process_sse_buffer(&mut buffer, &mut content);
    }

    Ok(SseOutcome {
        content,
        first_chunk_seen,
    })
}

fn process_sse_buffer(buffer: &mut String, content: &mut String) {
    while let Some(pos) = buffer.find('\n') {
        let line = buffer[..pos].to_string();
        *buffer = buffer[pos + 1..].to_string();

        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(':') || trimmed == "data: [DONE]" {
            continue;
        }
        let Some(data) = trimmed.strip_prefix("data: ") else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<Value>(data) else {
            continue;
        };
        if let Some(delta) = json.pointer("/choices/0/delta/content").and_then(|v| v.as_str()) {
            content.push_str(delta);
        }
    }
}

fn resolve_tool_name_to_skill_id(name: &str, registry: &crate::skill::registry::SkillRegistry) -> Option<String> {
    if name == "bili_account_list" || name == "bili_account_list_local" {
        if registry.get("bili.account.list").is_some() {
            return Some("bili.account.list".to_string());
        }
    }
    for def in registry.list() {
        if let Some(ref t) = def.llm_tool {
            if t.name == name {
                return Some(def.id.clone());
            }
        }
    }
    let normalized = name.replace('_', ".");
    if registry.get(&normalized).is_some() {
        return Some(normalized);
    }
    None
}

fn infer_monthly_report_plan(user_message: &str) -> Option<MonthlyReportPlan> {
    let asks_daily = ["按日", "每天", "逐日", "按天"].iter().any(|kw| user_message.contains(kw));
    if !asks_daily {
        return None;
    }

    let chars = user_message.chars().collect::<Vec<_>>();
    for window in chars.windows(7) {
        let candidate = window.iter().collect::<String>();
        if candidate.chars().nth(4) == Some('-') {
            let year = candidate[0..4].parse::<i32>().ok()?;
            let month = candidate[5..7].parse::<u32>().ok()?;
            let first = NaiveDate::from_ymd_opt(year, month, 1)?;
            let mut dates = Vec::new();
            let mut current = first;
            while current.month() == month {
                dates.push(current.format("%Y-%m-%d").to_string());
                current = current.succ_opt()?;
            }
            return Some(MonthlyReportPlan {
                expected_dates: dates,
                completed_dates: BTreeSet::new(),
            });
        }
    }
    None
}

fn maybe_mark_report_date_completed(plan: &mut MonthlyReportPlan, tool_name: &str, arguments: &Value) {
    if tool_name != "bili_report_query" {
        return;
    }
    let start = arguments.get("start_time").and_then(|v| v.as_str());
    let end = arguments.get("end_time").and_then(|v| v.as_str());
    if start.is_some() && start == end {
        if let Some(date) = start {
            plan.completed_dates.insert(date.to_string());
        }
    }
}

fn persist_run_state(run: &AgentRunState) {
    let db_path = db::models::db_path();
    if let Ok(conn) = db::models::open_rw(&db_path) {
        let _ = db::models::upsert_agent_run(
            &conn,
            &db::models::AgentRunRow {
                id: run.run_id.clone(),
                task_id: run.task_id.clone(),
                session_id: run.session_id.clone(),
                status: serde_json::to_string(&run.status).unwrap_or_else(|_| "\"running\"".to_string()).trim_matches('"').to_string(),
                current_step: run.current_step,
                total_retries: run.total_retries,
                max_steps: run.max_steps,
                final_message: run.final_message.clone(),
                waiting_for_user: run.waiting_for_user.clone(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            },
        );
        let rows = run
            .steps
            .iter()
            .map(|step| db::models::AgentStepRow {
                id: format!("{}:{}", run.run_id, step.step),
                run_id: run.run_id.clone(),
                step: step.step,
                title: step.title.clone(),
                status: serde_json::to_string(&step.status).unwrap_or_else(|_| "\"running\"".to_string()).trim_matches('"').to_string(),
                action_type: step.action_type.clone(),
                tool_name: step.tool_name.clone(),
                tool_arguments: step.tool_arguments.as_ref().and_then(|v| serde_json::to_string(v).ok()),
                tool_success: step.tool_success,
                tool_result: step.tool_result.as_ref().and_then(|v| serde_json::to_string(v).ok()),
                message: step.message.clone(),
                retry_count: step.retry_count,
                error: step.error.clone(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
            .collect::<Vec<_>>();
        let _ = db::models::replace_agent_steps(&conn, &run.run_id, &rows);
    }
}

fn persist_run_event(run: &AgentRunState, event_type: &str, payload: Option<Value>) {
    let db_path = db::models::db_path();
    if let Ok(conn) = db::models::open_rw(&db_path) {
        let _ = db::models::insert_agent_event(
            &conn,
            &db::models::AgentEventRow {
                id: Uuid::new_v4().to_string(),
                run_id: run.run_id.clone(),
                task_id: run.task_id.clone(),
                step: if run.current_step == 0 { None } else { Some(run.current_step) },
                event_type: event_type.to_string(),
                payload_json: payload.and_then(|v| serde_json::to_string(&v).ok()),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
        );
    }
}
