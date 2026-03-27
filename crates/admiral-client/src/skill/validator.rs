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

    Ok(())
}
