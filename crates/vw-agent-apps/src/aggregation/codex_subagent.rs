//! Codex aggregation subagent choices and managed config projection.

use std::collections::HashSet;

use serde_json::Value;
use toml_edit::Value as TomlValue;
use vw_api_types::agent_apps::AppId;

use super::codex_catalog::CodexCatalogError;
use super::model_ids::{self, Decoded};

const DEFAULT_MODEL: &str = "default_subagent_model";
const DEFAULT_EFFORT: &str = "default_subagent_reasoning_effort";

#[derive(Debug, PartialEq, Eq)]
pub struct CodexSubagentChoice<'a> {
    pub model_id: &'a str,
    pub label: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CodexSubagentPlan<'a> {
    pub choices: Vec<CodexSubagentChoice<'a>>,
    pub effective_model_id: Option<&'a str>,
}

/// Use the managed catalog selected for this publication. Pass None when the
/// client uses its own catalog or aggregation has no published catalog.
/// The caller retains the requested ID even if it is currently ineffective.
pub fn plan_codex_subagent<'a>(
    published_catalog: Option<&'a Value>,
    requested_model_id: Option<&str>,
) -> Result<CodexSubagentPlan<'a>, CodexCatalogError> {
    let mut plan = CodexSubagentPlan { choices: Vec::new(), effective_model_id: None };
    let Some(catalog) = published_catalog else {
        return Ok(plan);
    };
    let rows =
        catalog.get("models").and_then(Value::as_array).ok_or(CodexCatalogError::InvalidCatalog)?;
    let mut slugs = HashSet::with_capacity(rows.len());
    for row in rows {
        let model_id = row
            .get("slug")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty() && !id.chars().any(char::is_control))
            .ok_or(CodexCatalogError::InvalidCatalog)?;
        if !slugs.insert(model_id) {
            return Err(CodexCatalogError::InvalidCatalog);
        }
        // Only non-default members have encoded IDs in the managed catalog.
        let decoded = model_ids::decode(AppId::Codex, model_id)
            .map_err(|_| CodexCatalogError::InvalidCatalog)?;
        if !matches!(decoded, Decoded::Stack { .. })
            || row.get("multi_agent_version").and_then(Value::as_str) == Some("disabled")
        {
            continue;
        }

        let label = row.get("display_name").and_then(Value::as_str).unwrap_or(model_id);
        plan.choices.push(CodexSubagentChoice { model_id, label });
        if requested_model_id == Some(model_id) {
            plan.effective_model_id = Some(model_id);
        }
    }
    Ok(plan)
}

impl CodexSubagentPlan<'_> {
    /// Replace the provider's subagent defaults in its nested projection.
    /// The existing config writer clears the managed keys before applying it.
    pub fn apply_to_nested(&self, nested: &mut Vec<(Vec<String>, TomlValue)>) {
        nested.retain(|(path, _)| {
            !matches!(path.as_slice(), [table, key] if table == "agents"
                && matches!(key.as_str(), DEFAULT_MODEL | DEFAULT_EFFORT))
        });
        if let Some(model_id) = self.effective_model_id {
            nested.push((vec!["agents".into(), DEFAULT_MODEL.into()], TomlValue::from(model_id)));
        }
    }
}
