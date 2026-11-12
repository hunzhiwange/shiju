//! Model-specific thinking controls: never equate numeric budgets with effort.
use crate::provider::provider;
use axum::http::StatusCode;
use serde_json::{Value, json};
pub(crate) fn supports(api: &crate::provider::provider::ApiInfo, provider: &str) -> bool {
    use vw_shared::provider::reasoning::{ReasoningFormat, reasoning_profile};
    let profile = reasoning_profile(&api.id, &api.adapter, provider, &api.url);
    matches!(api.adapter.as_str(), "openai" | "openai-compatible")
        && (!profile.levels.is_empty() || matches!(profile.format, ReasoningFormat::DashScope))
}

pub(super) fn configure(
    body: &mut Value,
    input: &Value,
    model: &crate::provider::provider::Model,
) -> Result<(), StatusCode> {
    use vw_shared::provider::reasoning::{ReasoningFormat, reasoning_profile};
    let api = &model.api;
    let profile = reasoning_profile(&api.id, &api.adapter, &model.provider_id, &api.url);
    let requested = input["thinking"]["type"].as_str();
    let effort = input["output_config"].get("effort");
    if requested.is_none() || requested == Some("disabled") {
        if effort.is_some() {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        if requested == Some("disabled")
            && !profile.levels.is_empty()
            && !profile.levels.contains(&"none")
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        configure_disabled(body, model);
        return Ok(());
    }
    if !model.capabilities.reasoning || !supports(api, &model.provider_id) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let budget = input["thinking"].get("budget_tokens");
    if let Some(effort) = effort {
        let effort = effort.as_str().ok_or(StatusCode::BAD_REQUEST)?;
        if budget.is_some()
            || effort == "none"
            || !profile.levels.contains(&effort)
            || !matches!(profile.format, ReasoningFormat::OpenAi | ReasoningFormat::DeepSeek)
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        body["reasoning_effort"] = effort.into();
    }
    match profile.format {
        ReasoningFormat::DashScope => {
            body["enable_thinking"] = true.into();
            if let Some(budget) = budget {
                body["thinking_budget"] = budget.clone();
            }
        }
        ReasoningFormat::DeepSeek if budget.is_none() => {
            body["thinking"] = json!({"type":"enabled"});
        }
        ReasoningFormat::OpenAi if budget.is_none() => {
            let effort = input["output_config"]["effort"]
                .as_str()
                .or_else(|| {
                    profile
                        .levels
                        .iter()
                        .find(|level| **level == "high")
                        .or_else(|| profile.levels.iter().find(|level| **level != "none"))
                        .copied()
                })
                .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            body["reasoning_effort"] = effort.into();
        }
        // A numeric thinking budget is not equivalent to an effort setting.
        _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
    }
    Ok(())
}

pub(super) fn configure_disabled(body: &mut serde_json::Value, model: &provider::Model) {
    use vw_shared::provider::reasoning::{ReasoningFormat, reasoning_levels, reasoning_profile};

    // Apply an explicitly disabled mode only where the deployment can express it.
    if super::transport::official_deepseek_messages_url(&model.api).is_some() {
        body["thinking"] = serde_json::json!({"type": "disabled"});
        return;
    }
    let api = &model.api;
    let profile = reasoning_profile(&api.id, &api.adapter, &model.provider_id, &api.url);
    let levels = reasoning_levels(
        &api.id,
        &api.adapter,
        &model.provider_id,
        &api.url,
        model.capabilities.reasoning_options.as_deref(),
    );
    let supports_disabled = match &model.capabilities.reasoning_options {
        Some(options) => options.iter().any(|option| match option {
            vw_shared::provider::reasoning::ReasoningOption::Toggle => true,
            vw_shared::provider::reasoning::ReasoningOption::Effort { values } => {
                values.iter().any(|value| value == "none")
            }
            _ => false,
        }),
        None => profile.levels.contains(&"none"),
    };
    if supports_disabled {
        match profile.format {
            ReasoningFormat::DashScope => body["enable_thinking"] = false.into(),
            ReasoningFormat::DeepSeek => {
                body["thinking"] = serde_json::json!({"type": "disabled"});
            }
            ReasoningFormat::OpenAi
                if profile.levels.contains(&"none")
                    || levels.iter().any(|level| level == "none") =>
            {
                body["reasoning_effort"] = "none".into();
            }
            _ => {}
        }
    }
}
