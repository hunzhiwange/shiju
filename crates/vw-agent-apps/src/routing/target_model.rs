//! Model policy for an already authorized application/provider snapshot.
use serde_json::Value;
use vw_api_types::agent_apps::AppId;

/// Resolve only within the supplied provider. Authorization and target selection
/// remain with the executor; this policy never searches providers or tenants.
pub fn resolve(app: AppId, settings: &Value, model: &str) -> String {
    if app != AppId::Claude {
        // In particular, Grok Build names are opaque, even when they resemble
        // Claude/Codex aggregation identifiers or local context markers.
        return model.to_owned();
    }
    let configured = |key: &str| {
        settings
            .get("env")
            .and_then(|env| env.get(key))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
    };
    let lower = model.to_lowercase();
    let defaults = crate::providers::desktop_mapping::defaults();
    for role in ["fable", "haiku", "opus", "sonnet"] {
        if lower.contains(role) {
            let route = defaults.iter().find(|route| route.route_id.contains(role));
            if let Some(mapped) = route.and_then(|route| configured(&route.env_key)) {
                return strip_context(mapped).to_owned();
            }
            if role == "fable"
                && let Some(mapped) = configured("ANTHROPIC_DEFAULT_OPUS_MODEL")
            {
                return strip_context(mapped).to_owned();
            }
        }
    }
    if let Some(subagent) = configured("CLAUDE_CODE_SUBAGENT_MODEL")
        && (strip_context(model) == strip_context(subagent)
            || subagent_client_id(subagent).is_some_and(|id| strip_context(model) == id))
    {
        return strip_context(subagent).to_owned();
    }
    strip_context(configured("ANTHROPIC_MODEL").unwrap_or(model)).to_owned()
}

fn strip_context(model: &str) -> &str {
    let trimmed = model.trim_end();
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 4 && bytes[bytes.len() - 4..].eq_ignore_ascii_case(b"[1m]") {
        trimmed[..trimmed.len() - 4].trim_end()
    } else {
        model
    }
}

// Mirror the native Claude client catalog's two point-release aliases. These
// are reverse aliases only; no provider or application is selected here.
fn subagent_client_id(model: &str) -> Option<&'static str> {
    let model = strip_context(model).trim().to_ascii_lowercase();
    for (stem, client) in [("opus-5", "claude-opus-5-5"), ("sonnet-5", "claude-sonnet-5-5")] {
        for minor in [".5", "-5"] {
            if let Some(at) = model.find(&format!("{stem}{minor}")) {
                let rest = &model[at + stem.len() + minor.len()..];
                if !rest.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
                    return Some(client);
                }
            }
        }
    }
    None
}
