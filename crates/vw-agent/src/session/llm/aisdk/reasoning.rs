//! Validate selected effort and construct protocol-specific thinking switches.
use aisdk::core::language_model::ReasoningEffort;
use serde_json::{Map, Value, json};
use vw_shared::provider::reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};
use vw_shared::provider::types::Model;

fn parse_effort(options: &Map<String, Value>) -> aisdk::Result<Option<ReasoningEffort>> {
    options
        .get("reasoning_effort")
        .map(|value| {
            serde_json::from_value(value.clone()).map_err(|_| {
                aisdk::Error::InvalidInput(
                    "reasoning_effort must be one of none/minimal/low/medium/high/xhigh/max".into(),
                )
            })
        })
        .transpose()
}

pub(super) fn validate_reasoning(model: &Model, options: &Value) -> aisdk::Result<()> {
    let Some(options) = options.as_object() else {
        return Ok(());
    };
    let Some(effort) = parse_effort(options)? else {
        return Ok(());
    };
    let levels = reasoning_levels(
        &model.api.id,
        &model.api.adapter,
        &model.provider_id,
        &model.api.url,
        model.capabilities.reasoning_options.as_deref(),
    );
    if !model.capabilities.reasoning || !levels.iter().any(|level| level == effort.as_str()) {
        return Err(aisdk::Error::InvalidInput(format!(
            "model {}/{} does not support reasoning_effort={}; supported levels: {}",
            model.provider_id,
            model.api.id,
            effort.as_str(),
            levels.join(", ")
        )));
    }
    if options.contains_key("thinking_budget") {
        return Err(aisdk::Error::InvalidInput(
            "reasoning_effort cannot be combined with thinking_budget".into(),
        ));
    }
    Ok(())
}

pub(super) fn reasoning_request_config(
    provider: &str,
    url: &str,
    model: &str,
    options: &Map<String, Value>,
) -> aisdk::Result<(Option<ReasoningEffort>, Map<String, Value>)> {
    let Some(effort) = parse_effort(options)? else {
        return Ok((None, Map::new()));
    };
    let adapter = if url.trim_end_matches('/').ends_with("/messages") {
        "anthropic"
    } else {
        "openai-compatible"
    };
    let profile = reasoning_profile(model, adapter, provider, url);
    let mut body = Map::new();
    match profile.format {
        ReasoningFormat::DashScope => {
            body.insert("enable_thinking".into(), json!(effort != ReasoningEffort::None));
            if effort == ReasoningEffort::None {
                return Ok((None, body));
            }
        }
        ReasoningFormat::DeepSeek if !url.trim_end_matches('/').ends_with("/responses") => {
            body.insert(
                "thinking".into(),
                json!({"type": if effort == ReasoningEffort::None {"disabled"} else {"enabled"}}),
            );
            if effort == ReasoningEffort::None {
                return Ok((None, body));
            }
        }
        ReasoningFormat::Anthropic if !model.starts_with("claude-opus-4-5") => {
            body.insert(
                "thinking".into(),
                json!({"type": if effort == ReasoningEffort::None {"disabled"} else {"adaptive"}}),
            );
        }
        _ => {}
    }
    Ok((Some(effort), body))
}

#[cfg(test)]
#[path = "reasoning_tests.rs"]
mod tests;
