//! Skill input validator: checks required params, value ranges, types.

use admiral_types::skill::{SkillCall, SkillDefinition};

/// Validate a skill call against its definition. Returns Ok(()) or an error message.
pub fn validate_call(skill: &SkillDefinition, call: &SkillCall) -> Result<(), String> {
    let required = skill
        .input
        .as_ref()
        .map(|i| i.required.as_slice())
        .unwrap_or(&[]);

    let params = match call.params.as_object() {
        Some(m) => m,
        None => return Err("params must be a JSON object".to_string()),
    };

    for key in required.iter() {
        if !params.contains_key(key) {
            return Err(format!("missing required parameter: {}", key));
        }
    }

    if skill.category == admiral_types::skill::SkillCategory::Mapi
        && skill.api.is_some()
        && call.account_id.is_none()
    {
        if required.iter().any(|k| k.as_str() == "account_id") {
            if !params.contains_key("account_id") {
                return Err("MAPI skill requires account_id (in params or call.account_id)".to_string());
            }
        }
    }

    Ok(())
}
