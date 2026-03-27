//! Agent core interaction loop (ReAct): context assembly, LLM call, tool execution, result feedback.

use crate::agent::context::{ContextAssembler, Message, Session};
use crate::agent::memory::MemoryStore;
use crate::agent::profile;
use crate::network::api_client::ApiClient;
use admiral_types::api::LlmChatRequest;
use admiral_types::skill::{SkillCall, SkillResult};
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Events emitted during streaming LLM interaction.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "delta")]
    Delta { content: String },
    #[serde(rename = "tool_use")]
    ToolUse { name: String },
    #[serde(rename = "tool_result")]
    ToolResult { name: String, success: bool },
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "error")]
    Error { message: String },
}

/// Response from one agent turn (final text and optional tool calls made).
#[derive(Clone, Debug, serde::Serialize)]
pub struct AgentResponse {
    pub content: String,
    pub tool_calls_used: Vec<(String, SkillResult)>,
    pub session: Session,
}

/// Agent runtime: owns context assembler, and uses external API client + skill executor.
pub struct AgentRuntime {
    context: ContextAssembler,
    api_client: Arc<ApiClient>,
    skill_executor: Arc<crate::skill::executor::SkillExecutor>,
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
        })
    }

    /// Non-streaming process (kept for backward compatibility, e.g. memory_updater).
    pub async fn process_message(
        &self,
        session: Session,
        user_message: &str,
        access_token: Option<&str>,
    ) -> Result<AgentResponse, anyhow::Error> {
        let (tx, _rx) = mpsc::channel(1);
        self.process_message_stream(session, user_message, access_token, tx).await
    }

    /// Streaming process: emits StreamEvent tokens via `tx` while performing the ReAct loop.
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

        let knowledge_snippets: Vec<String> = if let Some(token) = access_token {
            self.api_client
                .knowledge_query(token, user_message, 5)
                .await
                .map(|s| s.into_iter().map(|k| k.content).collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let mut current_session = session;
        let mut tool_calls_used: Vec<(String, SkillResult)> = Vec::new();
        let max_turns = 10;
        let mut turn = 0;

        loop {
            turn += 1;
            if turn > max_turns {
                let _ = tx.send(StreamEvent::Done).await;
                break Ok(AgentResponse {
                    content: "达到最大轮次，请简化请求。".to_string(),
                    tool_calls_used,
                    session: current_session,
                });
            }

            let messages = self.context.build(
                &current_session,
                &knowledge_snippets,
            );

            let tools_list = self.context.skill_registry.build_llm_tools();
            let tools = if tools_list.is_empty() {
                None
            } else {
                Some(serde_json::to_value(&tools_list).unwrap_or_else(|_| json!([])))
            };
            let tool_choice = tools.as_ref().map(|_| json!("auto"));

            let req = LlmChatRequest {
                messages,
                stream: true,
                model: self.context.profile.default_model.clone(),
                max_tokens: Some(4096),
                tools,
                tool_choice,
            };

            let token = access_token.ok_or_else(|| anyhow::anyhow!("access_token required for LLM"))?;

            let resp = self
                .api_client
                .llm_chat_stream(token, &req)
                .await
                .map_err(|e| anyhow::anyhow!("llm_chat_stream: {}", e))?;

            let (content, tool_calls_value) = parse_sse_stream(resp, &tx).await?;

            current_session.messages.push(Message {
                role: "assistant".to_string(),
                content: if content.is_empty() { None } else { Some(content.clone()) },
                tool_calls: tool_calls_value.clone(),
                name: None,
                tool_call_id: None,
            });

            let tool_calls_arr = tool_calls_value
                .as_ref()
                .and_then(|v| v.as_array())
                .cloned();

            if tool_calls_arr.is_none() || tool_calls_arr.as_ref().map(|a| a.is_empty()).unwrap_or(true) {
                let _ = tx.send(StreamEvent::Done).await;
                break Ok(AgentResponse {
                    content,
                    tool_calls_used,
                    session: current_session,
                });
            }

            let tool_calls = tool_calls_arr.unwrap();
            for tc in &tool_calls {
                let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let func = tc.get("function").ok_or_else(|| anyhow::anyhow!("tool_call missing function"))?;
                let name = func.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args_str = func.get("arguments").and_then(|v| v.as_str()).unwrap_or("{}");
                let params: Value = serde_json::from_str(args_str).unwrap_or(json!({}));

                let _ = tx.send(StreamEvent::ToolUse { name: name.to_string() }).await;

                let skill_id = resolve_tool_name_to_skill_id(name, self.context.skill_registry.as_ref());
                let skill_id = match skill_id {
                    Some(s) => s,
                    None => {
                        let _ = tx.send(StreamEvent::ToolResult { name: name.to_string(), success: false }).await;
                        current_session.messages.push(Message {
                            role: "tool".to_string(),
                            content: Some(format!("error: unknown tool {}", name)),
                            tool_calls: None,
                            name: Some(name.to_string()),
                            tool_call_id: Some(id.to_string()),
                        });
                        continue;
                    }
                };

                let account_id = params.get("account_id").and_then(|v| v.as_i64());
                let call = SkillCall {
                    skill_id: skill_id.clone(),
                    params,
                    account_id,
                    dry_run: false,
                };

                let result = self
                    .skill_executor
                    .execute(call, access_token)
                    .await
                    .unwrap_or_else(|e| SkillResult::err(e.to_string(), None));

                let _ = tx.send(StreamEvent::ToolResult { name: name.to_string(), success: result.success }).await;

                tool_calls_used.push((skill_id, result.clone()));

                let result_content = if result.success {
                    serde_json::to_string(&result.data).unwrap_or_else(|_| "{}".to_string())
                } else {
                    result.error.unwrap_or_else(|| "unknown error".to_string())
                };

                current_session.messages.push(Message {
                    role: "tool".to_string(),
                    content: Some(result_content),
                    tool_calls: None,
                    name: Some(name.to_string()),
                    tool_call_id: Some(id.to_string()),
                });
            }
        }
    }
}

/// Process complete lines from the SSE buffer, emitting deltas and accumulating tool_calls.
async fn process_sse_buffer(
    buffer: &mut String,
    content: &mut String,
    tool_calls_map: &mut HashMap<u64, (String, String, String)>,
    tx: &mpsc::Sender<StreamEvent>,
) {
    while let Some(pos) = buffer.find('\n') {
        let line = buffer[..pos].to_string();
        *buffer = buffer[pos + 1..].to_string();

        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(':') {
            continue;
        }
        if trimmed == "data: [DONE]" {
            continue;
        }

        let data = match trimmed.strip_prefix("data: ") {
            Some(d) => d,
            None => continue,
        };

        let json: Value = match serde_json::from_str(data) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let delta = match json.pointer("/choices/0/delta") {
            Some(d) => d,
            None => continue,
        };

        if let Some(c) = delta.get("content").and_then(|v| v.as_str()) {
            if !c.is_empty() {
                content.push_str(c);
                let _ = tx.send(StreamEvent::Delta { content: c.to_string() }).await;
            }
        }

        if let Some(tcs) = delta.get("tool_calls").and_then(|v| v.as_array()) {
            for tc in tcs {
                let index = tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                let entry = tool_calls_map.entry(index).or_insert_with(|| (String::new(), String::new(), String::new()));
                if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                    entry.0 = id.to_string();
                }
                if let Some(func) = tc.get("function") {
                    if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                        entry.1 = name.to_string();
                    }
                    if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                        entry.2.push_str(args);
                    }
                }
            }
        }
    }
}

/// Parse an SSE stream from the LLM, emitting content deltas and accumulating tool_calls.
/// Returns (full_content, tool_calls_json_value).
async fn parse_sse_stream(
    resp: reqwest::Response,
    tx: &mpsc::Sender<StreamEvent>,
) -> Result<(String, Option<Value>), anyhow::Error> {
    let mut content = String::new();
    let mut tool_calls_map: HashMap<u64, (String, String, String)> = HashMap::new();
    let mut buffer = String::new();

    let mut stream = resp.bytes_stream();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| anyhow::anyhow!("stream read: {}", e))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        process_sse_buffer(&mut buffer, &mut content, &mut tool_calls_map, tx).await;
    }

    if !buffer.trim().is_empty() {
        buffer.push('\n');
        process_sse_buffer(&mut buffer, &mut content, &mut tool_calls_map, tx).await;
    }

    let tool_calls_value = if tool_calls_map.is_empty() {
        None
    } else {
        let mut sorted: Vec<_> = tool_calls_map.into_iter().collect();
        sorted.sort_by_key(|(k, _)| *k);
        let arr: Vec<Value> = sorted
            .into_iter()
            .map(|(_, (id, name, args))| {
                json!({
                    "id": id,
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": args,
                    }
                })
            })
            .collect();
        Some(Value::Array(arr))
    };

    Ok((content, tool_calls_value))
}

/// Resolve LLM tool name to skill_id.
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
