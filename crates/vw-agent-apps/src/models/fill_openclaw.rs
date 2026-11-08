//! Read-only OpenClaw model metadata fill suggestions.

use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use vw_api_types::model_catalog::{ModelMetadataDto, ModelMetadataField, ModelMetadataProvenance};

/// OpenClaw model keys to merge into the draft, with sources for the changed fields.
#[derive(Debug, Default, PartialEq)]
pub struct OpenClawModelFill {
    pub changes: Map<String, Value>,
    pub provenance: BTreeMap<ModelMetadataField, ModelMetadataProvenance>,
}

/// Compare a native `OpenClawProfile::declaration.models` row with metadata resolved for
/// this OpenClaw deployment and model.
pub fn fill_openclaw_model(
    model: &Map<String, Value>,
    resolved: &ModelMetadataDto,
) -> OpenClawModelFill {
    use vw_api_types::model_catalog::{
        ModelCapabilityState::{Supported, Unsupported},
        ModelMetadataField::{ContextWindow, Cost, InputModalities, MaxOutputTokens, Reasoning},
    };

    let mut fill = OpenClawModelFill::default();
    let mut add = |key: &str, field, value| {
        fill.changes.insert(key.into(), value);
        if let Some(source) = resolved.provenance.get(&field) {
            fill.provenance.insert(field, source.clone());
        }
    };

    for (key, field, limit) in [
        ("contextWindow", ContextWindow, resolved.context_window),
        ("maxTokens", MaxOutputTokens, resolved.max_output_tokens),
    ] {
        if model.get(key).is_none_or(|value| {
            value.is_null() || value.as_str().is_some_and(|text| text.trim().is_empty())
        }) && let Some(limit) = limit.filter(|limit| *limit > 0)
        {
            add(key, field, limit.into());
        }
    }

    match (resolved.reasoning, model.get("reasoning")) {
        (Supported, current) if current != Some(&Value::Bool(true)) => {
            add("reasoning", Reasoning, true.into());
        }
        (Unsupported, None) => add("reasoning", Reasoning, false.into()),
        _ => {}
    }

    if let Some(modalities) = &resolved.input_modalities {
        let image = modalities.iter().any(|mode| mode == "image");
        let current = model.get("input");
        if current
            .is_none_or(|value| value.is_null() || value.as_array().is_some_and(Vec::is_empty))
        {
            let input = if image { json!(["text", "image"]) } else { json!(["text"]) };
            add("input", InputModalities, input);
        } else if image
            && let Some(input) = current.and_then(Value::as_array)
            && !input.iter().any(|mode| mode == "image")
        {
            let mut input = input.clone();
            input.push("image".into());
            add("input", InputModalities, Value::Array(input));
        }
    }

    if model.get("cost").is_none_or(Value::is_null)
        && let Some(cost) = &resolved.cost
    {
        add("cost", Cost, json!(cost));
    }

    fill
}
