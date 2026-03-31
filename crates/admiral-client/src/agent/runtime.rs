//! Agent core interaction loop with explicit observe -> decide -> act state.

use crate::agent::context::{ContextAssembler, Message, Session};
use crate::agent::memory::MemoryStore;
use crate::agent::profile;
use crate::db;
use crate::network::api_client::extract_text_from_llm_response;
use crate::network::api_client::ApiClient;
use admiral_types::api::LlmResponsesRequest;
use admiral_types::skill::{SkillCall, SkillResult};
use chrono::{Datelike, NaiveDate};
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
    Finish {
        message: String,
    },
}

#[derive(Clone, Debug)]
enum ActionParseErrorKind {
    EmptyResponse,
    EmptyObject,
    JsonParseFailed,
    MissingFields,
    InvalidAction,
    InvalidEnvelope,
}

impl ActionParseErrorKind {
    fn stage(&self) -> &'static str {
        match self {
            Self::EmptyResponse => "empty_assistant_reply",
            Self::EmptyObject => "empty_action_object",
            Self::JsonParseFailed => "action_json_parse_failed",
            Self::MissingFields => "action_missing_fields",
            Self::InvalidAction => "invalid_action",
            Self::InvalidEnvelope => "llm_response_invalid",
        }
    }

    fn level(&self) -> &'static str {
        match self {
            Self::InvalidEnvelope => "error",
            _ => "warn",
        }
    }
}

#[derive(Clone, Debug)]
struct ActionParseError {
    kind: ActionParseErrorKind,
    detail: String,
    raw_output: Option<String>,
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
                AgentAction::Finish { message } => {
                    current_session.messages.push(Message {
                        role: "assistant".to_string(),
                        content: Some(message.clone()),
                        tool_calls: None,
                        name: None,
                        tool_call_id: None,
                    });
                    mark_step_terminal(&mut run, step_no, "finish", Some(message.clone()));
                    run.status = AgentRunStatus::Succeeded;
                    run.final_message = Some(message.clone());
                    self.publish_run_state(
                        &tx,
                        &run,
                        Some("run_finished"),
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
            let llm_selection = crate::config::ClientConfig::from_env().llm_selection();
            let req = LlmResponsesRequest {
                messages,
                stream: false,
                provider: Some(llm_selection.provider),
                model: self.context.profile.default_model.clone(),
                max_tokens: Some(2048),
                tools: None,
                tool_choice: None,
                response_format: Some(agent_action_response_format()),
            };

            let raw_response = self
                .api_client
                .llm_responses(access_token, &req)
                .await
                .map_err(|e| anyhow::anyhow!("llm_responses: {}", e))?;
            let content = match extract_assistant_content(&raw_response) {
                Ok(content) => content,
                Err(err) => {
                    record_decision_parse_failure(tx, run, attempt, err).await;
                    continue;
                }
            };

            match parse_action(&content) {
                Ok(action) => {
                    return Ok(action);
                }
                Err(err) => {
                    record_decision_parse_failure(tx, run, attempt, err).await;
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

fn parse_action(raw: &str) -> Result<AgentAction, ActionParseError> {
    let trimmed = raw.trim();
    let unwrapped = trimmed
        .strip_prefix("```json")
        .and_then(|s| s.strip_suffix("```"))
        .or_else(|| trimmed.strip_prefix("```").and_then(|s| s.strip_suffix("```")))
        .map(|s| s.trim())
        .unwrap_or(trimmed);
    if unwrapped.is_empty() {
        return Err(ActionParseError {
            kind: ActionParseErrorKind::EmptyResponse,
            detail: "assistant content is empty".to_string(),
            raw_output: Some(raw.to_string()),
        });
    }

    let value = parse_action_json_value(unwrapped)?;
    let Some(obj) = value.as_object() else {
        return Err(ActionParseError {
            kind: ActionParseErrorKind::InvalidAction,
            detail: "action payload must be a JSON object".to_string(),
            raw_output: Some(unwrapped.to_string()),
        });
    };
    if obj.is_empty() {
        return Err(ActionParseError {
            kind: ActionParseErrorKind::EmptyObject,
            detail: "action object is empty".to_string(),
            raw_output: Some(unwrapped.to_string()),
        });
    }

    let action_type = required_string(obj, "type", unwrapped)?;
    match action_type {
        "call_tool" => {
            let tool_name = required_string(obj, "tool_name", unwrapped)?.to_string();
            let arguments = obj
                .get("arguments")
                .ok_or_else(|| missing_field("arguments", unwrapped))?;
            let arguments_value = match arguments {
                Value::Object(map) => Value::Object(map.clone()),
                Value::String(raw_json) => serde_json::from_str::<Value>(raw_json).map_err(|e| ActionParseError {
                    kind: ActionParseErrorKind::JsonParseFailed,
                    detail: format!("parse `arguments` json string: {}", e),
                    raw_output: Some(unwrapped.to_string()),
                })?,
                _ => {
                    return Err(ActionParseError {
                        kind: ActionParseErrorKind::InvalidAction,
                        detail: "`arguments` must be a JSON object or a JSON string".to_string(),
                        raw_output: Some(unwrapped.to_string()),
                    })
                }
            };
            let Some(arguments_obj) = arguments_value.as_object() else {
                return Err(ActionParseError {
                    kind: ActionParseErrorKind::InvalidAction,
                    detail: "`arguments` must decode to a JSON object".to_string(),
                    raw_output: Some(unwrapped.to_string()),
                });
            };
            let reason = optional_string(obj, "reason", unwrapped)?;
            Ok(AgentAction::CallTool {
                tool_name,
                arguments: Value::Object(arguments_obj.clone()),
                reason,
            })
        }
        "respond" => Ok(AgentAction::Respond {
            message: required_string(obj, "message", unwrapped)?.to_string(),
        }),
        "ask_user" => Ok(AgentAction::AskUser {
            message: required_string(obj, "message", unwrapped)?.to_string(),
        }),
        "finish" => Ok(AgentAction::Finish {
            message: required_string(obj, "message", unwrapped)?.to_string(),
        }),
        other => Err(ActionParseError {
            kind: ActionParseErrorKind::InvalidAction,
            detail: format!("unsupported action type `{}`", other),
            raw_output: Some(unwrapped.to_string()),
        }),
    }
}

fn parse_action_json_value(raw: &str) -> Result<Value, ActionParseError> {
    let mut stream = serde_json::Deserializer::from_str(raw).into_iter::<Value>();
    let first = match stream.next() {
        Some(Ok(value)) => value,
        Some(Err(e)) => {
            return Err(ActionParseError {
                kind: ActionParseErrorKind::JsonParseFailed,
                detail: format!("parse action json: {}", e),
                raw_output: Some(raw.to_string()),
            });
        }
        None => {
            return Err(ActionParseError {
                kind: ActionParseErrorKind::EmptyResponse,
                detail: "assistant content is empty".to_string(),
                raw_output: Some(raw.to_string()),
            });
        }
    };

    let mut duplicate_count = 0usize;
    for item in stream {
        let value = item.map_err(|e| ActionParseError {
            kind: ActionParseErrorKind::JsonParseFailed,
            detail: format!("parse action json: {}", e),
            raw_output: Some(raw.to_string()),
        })?;
        if value != first {
            return Err(ActionParseError {
                kind: ActionParseErrorKind::InvalidAction,
                detail: "multiple JSON objects found in action output".to_string(),
                raw_output: Some(raw.to_string()),
            });
        }
        duplicate_count += 1;
    }

    if duplicate_count > 0 {
        tracing::warn!(duplicate_count, "deduplicated repeated action JSON objects from model output");
    }

    Ok(first)
}

fn canonical_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
}

fn extract_assistant_content(raw_response: &[u8]) -> Result<String, ActionParseError> {
    let raw_text = String::from_utf8_lossy(raw_response).to_string();
    let text = extract_text_from_llm_response(raw_response).map_err(|detail| ActionParseError {
        kind: ActionParseErrorKind::InvalidEnvelope,
        detail,
        raw_output: Some(raw_text.clone()),
    })?;

    if text.trim().is_empty() {
        return Err(ActionParseError {
            kind: ActionParseErrorKind::EmptyResponse,
            detail: "assistant content is empty".to_string(),
            raw_output: Some(raw_text),
        });
    }

    Ok(text)
}

fn required_string<'a>(
    obj: &'a serde_json::Map<String, Value>,
    field: &str,
    raw_output: &str,
) -> Result<&'a str, ActionParseError> {
    match obj.get(field).and_then(|v| v.as_str()) {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(missing_field(field, raw_output)),
    }
}

fn optional_string(
    obj: &serde_json::Map<String, Value>,
    field: &str,
    raw_output: &str,
) -> Result<Option<String>, ActionParseError> {
    match obj.get(field) {
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(Value::Null) | None => Ok(None),
        Some(_) => Err(ActionParseError {
            kind: ActionParseErrorKind::InvalidAction,
            detail: format!("`{}` must be a string when present", field),
            raw_output: Some(raw_output.to_string()),
        }),
    }
}

fn missing_field(field: &str, raw_output: &str) -> ActionParseError {
    ActionParseError {
        kind: ActionParseErrorKind::MissingFields,
        detail: format!("missing or empty required field `{}`", field),
        raw_output: Some(raw_output.to_string()),
    }
}

fn agent_action_response_format() -> Value {
    json!({
        "type": "json_schema",
        "json_schema": {
            "name": "agent_action",
            "strict": true,
            "schema": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "type": {
                        "type": "string",
                        "enum": ["call_tool", "respond", "ask_user", "finish"]
                    },
                    "tool_name": {
                        "type": ["string", "null"]
                    },
                    "arguments": {
                        "type": ["object", "null"]
                    },
                    "reason": {
                        "type": ["string", "null"]
                    },
                    "message": {
                        "type": ["string", "null"]
                    }
                },
                "required": ["type"]
            }
        }
    })
}

async fn record_decision_parse_failure(
    tx: &mpsc::Sender<StreamEvent>,
    run: &mut AgentRunState,
    attempt: u32,
    err: ActionParseError,
) {
    if let Some(step) = run.steps.iter_mut().find(|s| s.step == run.current_step) {
        step.error = Some(format!("{}: {}", err.kind.stage(), err.detail));
    }
    emit_status(
        tx,
        err.kind.stage(),
        format!("模型动作输出无效：{}。准备自动重试。", err.detail),
        Some(err.kind.level()),
    )
    .await;
    persist_run_event(
        run,
        err.kind.stage(),
        Some(json!({
            "step": run.current_step,
            "attempt": attempt,
            "detail": err.detail,
            "raw_output": err.raw_output,
        })),
    );
    persist_run_state(run);
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

#[cfg(test)]
mod tests {
    use super::{agent_action_response_format, parse_action, ActionParseErrorKind};

    #[test]
    fn agent_action_schema_matches_openai_structured_output_constraints() {
        let schema = agent_action_response_format();
        let root = &schema["json_schema"]["schema"];

        assert_eq!(
            root["type"],
            serde_json::json!("object")
        );
        assert_eq!(
            root["required"],
            serde_json::json!(["type"])
        );
        assert_eq!(
            root["properties"]["reason"]["type"],
            serde_json::json!(["string", "null"])
        );
        assert_eq!(
            root["properties"]["arguments"]["type"],
            serde_json::json!(["object", "null"])
        );
        assert!(root.get("anyOf").is_none());
        assert!(root.get("oneOf").is_none());
        assert!(root.get("allOf").is_none());
    }

    #[test]
    fn parse_action_accepts_duplicate_identical_json_objects() {
        let raw = concat!(
            "{\"type\":\"ask_user\",\"message\":\"pick account\"}\n",
            "{\"type\":\"ask_user\",\"message\":\"pick account\"}"
        );

        let action = parse_action(raw).expect("duplicate identical action should be accepted");

        assert!(matches!(action, super::AgentAction::AskUser { message } if message == "pick account"));
    }

    #[test]
    fn parse_action_rejects_multiple_different_json_objects() {
        let raw = concat!(
            "{\"type\":\"ask_user\",\"message\":\"pick account\"}\n",
            "{\"type\":\"finish\",\"message\":\"done\"}"
        );

        let err = parse_action(raw).expect_err("different concatenated objects must be rejected");

        assert!(matches!(err.kind, ActionParseErrorKind::InvalidAction));
        assert!(err.detail.contains("multiple JSON objects"));
    }
}
