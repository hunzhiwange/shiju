//! Final wire normalization for Messages thinking options. No optimizer or
//! provider configuration is inferred from a display name.
use serde_json::{Value, json};

/// Called after authorized model mapping and request overrides. Only the
/// official DeepSeek endpoint and Claude model families have rules here.
pub fn normalize_messages(body: &mut Value, host: &str, path: &str) {
    if !path.trim_end_matches('/').ends_with("/messages") {
        return;
    }
    let deepseek = host.eq_ignore_ascii_case("api.deepseek.com")
        && path.trim_end_matches('/') == "/anthropic/v1/messages";
    let model = body
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .replace(['.', '_'], "-");
    let claude = model.contains("claude-");
    if !deepseek && !claude {
        return;
    }
    let adaptive = [
        "fable-5",
        "mythos-5",
        "mythos-preview",
        "sonnet-5",
        "opus-5",
        "opus-4-8",
        "opus-4-7",
        "opus-4-6",
        "sonnet-4-6",
    ]
    .iter()
    .any(|name| model.contains(name));
    let default_adaptive = ["fable-5", "mythos-5", "mythos-preview", "sonnet-5", "opus-5"]
        .iter()
        .any(|name| model.contains(name));
    let cannot_disable = ["fable-5", "mythos-5"].iter().any(|name| model.contains(name));
    let kind = body.pointer("/thinking/type").and_then(Value::as_str).map(str::to_owned);
    if !deepseek
        && claude
        && ((kind.is_none() && default_adaptive)
            || (kind.as_deref() == Some("enabled") && adaptive)
            || (kind.as_deref() == Some("disabled") && cannot_disable))
    {
        body["thinking"] = json!({"type": "adaptive"});
        if kind.as_deref() == Some("disabled") {
            if !body.get("output_config").is_some_and(Value::is_object) {
                body["output_config"] = json!({});
            }
            body["output_config"]["effort"] = json!("low");
        }
    }
    if !deepseek && body.pointer("/thinking/type").and_then(Value::as_str) == Some("enabled") {
        // Keep the caller's output ceiling; never emit a budget below the
        // Anthropic floor or one which consumes the whole output allowance.
        let ceiling = body.get("max_tokens").and_then(Value::as_u64).unwrap_or(0) / 2;
        let budget = body
            .pointer("/thinking/budget_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(ceiling);
        if budget < 1024 {
            body["thinking"] = json!({"type": "disabled"});
        } else {
            body["thinking"]["budget_tokens"] = json!(budget);
        }
    }
    match body.pointer("/thinking/type").and_then(Value::as_str) {
        Some("disabled") => {
            if let Some(thinking) = body["thinking"].as_object_mut() {
                thinking.remove("budget_tokens");
            }
            if let Some(obj) = body.as_object_mut() {
                obj.remove("reasoning_effort");
            }
            let empty = body.get_mut("output_config").and_then(Value::as_object_mut).is_some_and(
                |config| {
                    config.remove("effort");
                    config.is_empty()
                },
            );
            if empty {
                body.as_object_mut().unwrap().remove("output_config");
            }
        }
        Some("adaptive") => {
            if let Some(thinking) = body["thinking"].as_object_mut() {
                thinking.remove("budget_tokens");
            }
        }
        _ => {}
    }
}
