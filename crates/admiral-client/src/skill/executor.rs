//! Skill executor: dispatches to local or remote (MAPI via server) execution.

use admiral_types::api::MapiProxyRequest;
use admiral_types::skill::{SkillCall, SkillDefinition, SkillResult};
use crate::network::api_client::ApiClient;
use crate::skill::validator;
use std::sync::Arc;
use std::time::Instant;

pub struct SkillExecutor {
    registry: Arc<crate::skill::registry::SkillRegistry>,
    api_client: Arc<ApiClient>,
}

impl SkillExecutor {
    pub fn new(
        registry: Arc<crate::skill::registry::SkillRegistry>,
        api_client: Arc<ApiClient>,
    ) -> Self {
        Self {
            registry,
            api_client,
        }
    }

    /// Execute a skill call. For MAPI skills, access_token must be Some.
    pub async fn execute(
        &self,
        call: SkillCall,
        access_token: Option<&str>,
    ) -> Result<SkillResult, anyhow::Error> {
        let skill = self
            .registry
            .get(&call.skill_id)
            .ok_or_else(|| anyhow::anyhow!("skill not found: {}", call.skill_id))?;

        validator::validate_call(&skill, &call).map_err(|s| anyhow::anyhow!("{}", s))?;

        let start = Instant::now();

        let result = match skill.category {
            admiral_types::skill::SkillCategory::Mapi => {
                let token = access_token
                    .ok_or_else(|| anyhow::anyhow!("MAPI skill requires access_token"))?;
                self.execute_mapi(&skill, &call, token).await
            }
            admiral_types::skill::SkillCategory::Tools => self.execute_tools(&skill, &call).await,
            admiral_types::skill::SkillCategory::Semantic => {
                return Err(anyhow::anyhow!("semantic skills are executed by LLM, not by executor"));
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;
        Ok(match result {
            Ok(data) => SkillResult::ok(data, duration_ms),
            Err(e) => SkillResult::err(e.to_string(), Some(duration_ms)),
        })
    }

    async fn execute_mapi(
        &self,
        skill: &SkillDefinition,
        call: &SkillCall,
        access_token: &str,
    ) -> Result<serde_json::Value, anyhow::Error> {
        let api = skill
            .api
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("skill has no api definition"))?;

        let account_id = call
            .account_id
            .or_else(|| call.params.get("account_id").and_then(|v| v.as_i64()))
            .ok_or_else(|| anyhow::anyhow!("account_id required for MAPI"))?;

        let (query, body) = if api.method.eq_ignore_ascii_case("GET") {
            (Some(call.params.clone()), None)
        } else {
            (None, Some(call.params.clone()))
        };

        let req = MapiProxyRequest {
            account_id,
            method: api.method.clone(),
            path: api.path.clone(),
            query,
            body,
        };

        let (status, bytes) = self
            .api_client
            .mapi_proxy(access_token, &req)
            .await
            .map_err(|e| anyhow::anyhow!("mapi_proxy: {}", e))?;

        if !(200..300).contains(&status) {
            let msg = String::from_utf8_lossy(&bytes);
            return Err(anyhow::anyhow!("MAPI error {}: {}", status, msg));
        }

        let data: serde_json::Value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| serde_json::json!({ "raw": String::from_utf8_lossy(&bytes) }));
        Ok(data)
    }

    async fn execute_tools(
        &self,
        _skill: &SkillDefinition,
        _call: &SkillCall,
    ) -> Result<serde_json::Value, anyhow::Error> {
        Err(anyhow::anyhow!(
            "tools skills not implemented yet (excel_export, email_sender)"
        ))
    }
}
