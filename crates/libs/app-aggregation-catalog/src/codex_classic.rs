//! Classic subagent tool settings and model row projection for Codex aggregation.

use serde_json::{Map, Value};
use toml_edit::{DocumentMut, Item};

#[derive(Debug, PartialEq, Eq)]
pub struct CodexClassicTools {
    pub requested: bool,
    pub effective: bool,
    /// The client feature flag overrides a request for classic tools.
    pub multi_agent_v2_forced: bool,
    /// The subagent provider may be unable to decrypt the task from the main provider.
    pub encrypted_task_cross_provider: bool,
}

/// Project the classic tool preference onto fresh aggregate catalog rows.
/// Use the final client config and compare the providers of the effective models.
pub fn project_classic_tools(
    requested: Option<bool>,
    config: &DocumentMut,
    cross_provider_subagent: bool,
    rows: &mut [Map<String, Value>],
) -> CodexClassicTools {
    let requested = requested.unwrap_or(true);
    let multi_agent_v2_forced = requested
        && config
            .get("features")
            .and_then(|features| features.get("multi_agent_v2"))
            .and_then(|feature| {
                feature.as_bool().or_else(|| feature.get("enabled").and_then(Item::as_bool))
            })
            .unwrap_or(false);
    let effective = requested && !multi_agent_v2_forced;

    // Codex gives the user feature flag precedence over the row. Retain the
    // requested catalog setting so it takes effect when the override is removed.
    if requested {
        for row in rows {
            row.insert("multi_agent_version".into(), Value::String("v1".into()));
        }
    }

    CodexClassicTools {
        requested,
        effective,
        multi_agent_v2_forced,
        encrypted_task_cross_provider: cross_provider_subagent && !effective,
    }
}
