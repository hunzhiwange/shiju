//! Codex request controls that are specific to a Chat upstream.
use super::{UNSUPPORTED, object_keys};
use axum::http::StatusCode;
use serde_json::{Value, json};

/// Codex sends these response-only hints even with a third-party provider.
pub(in crate::model_relay) fn codex_chat_effort(
    body: &mut Value,
) -> Result<Option<String>, StatusCode> {
    let object = body.as_object_mut().ok_or(StatusCode::BAD_REQUEST)?;
    // Cache and client identifiers are optional hints. Do not forward them to Chat.
    object.remove("client_metadata");
    object.remove("prompt_cache_key");
    if let Some(include) = object.remove("include")
        && !include
            .as_array()
            .is_some_and(|parts| parts.iter().all(|part| part == "reasoning.encrypted_content"))
    {
        return Err(UNSUPPORTED);
    }
    let Some(reasoning) = object.remove("reasoning") else {
        return Ok(None);
    };
    object_keys(&reasoning, &["effort", "summary"])?;
    let effort = reasoning.get("effort").filter(|effort| !effort.is_null());
    match effort {
        None => Ok(None),
        Some(value)
            if matches!(
                value.as_str(),
                Some("none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max")
            ) =>
        {
            Ok(value.as_str().map(str::to_owned))
        }
        _ => Err(UNSUPPORTED),
    }
}

pub(in crate::model_relay) fn configure_codex_chat(
    body: &mut Value,
    model: &crate::provider::provider::Model,
    effort: Option<&str>,
) {
    use vw_shared::provider::reasoning::{ReasoningFormat, reasoning_profile};
    let api = &model.api;
    let profile = reasoning_profile(&api.id, &api.adapter, &model.provider_id, &api.url);
    if crate::model_relay::official_deepseek_messages_url(api).is_some() {
        // DeepSeek Chat enables thinking by default. Opt out for non-reasoning models.
        let enabled = model.capabilities.reasoning && effort != Some("none");
        body["thinking"] = json!({"type":if enabled {"enabled"} else {"disabled"}});
        if enabled && let Some(effort) = effort {
            body["reasoning_effort"] = match effort {
                "minimal" | "low" => "low",
                "max" => "max",
                _ => "high",
            }
            .into();
        }
        return;
    }
    if let Some(effort) =
        effort.filter(|effort| model.capabilities.reasoning && profile.levels.contains(effort))
    {
        match profile.format {
            ReasoningFormat::DeepSeek => {
                body["thinking"] =
                    json!({"type":if effort == "none" {"disabled"} else {"enabled"}});
            }
            ReasoningFormat::DashScope => body["enable_thinking"] = json!(effort != "none"),
            _ => {}
        }
        if effort != "none" || profile.format == ReasoningFormat::OpenAi {
            body["reasoning_effort"] = effort.into();
        }
    }
}
