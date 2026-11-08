//! Read-only metadata fill for OpenCode-shaped model drafts.

use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use vw_api_types::{
    agent_apps::AppId,
    model_catalog::{ModelMetadataDto, ModelMetadataField, ModelMetadataProvenance},
};

// OpenCode's native integer schema requires exact JavaScript integers.
const MAX_NATIVE_LIMIT: u64 = 9_007_199_254_740_991;

/// Model keys to merge into the same draft, with sources for the changed fields.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct OpenCodeModelFill {
    pub changes: Map<String, Value>,
    pub provenance: BTreeMap<ModelMetadataField, ModelMetadataProvenance>,
}

/// Compare a native provider's `declaration.models` row with metadata already
/// resolved for that deployment and model. A changed `limit` includes its existing keys.
pub fn fill_opencode_model(
    app: AppId,
    model: &Map<String, Value>,
    resolved: &ModelMetadataDto,
) -> Result<OpenCodeModelFill, &'static str> {
    use ModelMetadataField::{
        ContextWindow, InputModalities, MaxOutputTokens, OutputModalities, Reasoning,
    };
    use vw_api_types::model_catalog::ModelCapabilityState::Supported;

    if !matches!(app, AppId::OpenCode | AppId::Mcode | AppId::Dsh) {
        return Err("unsupported OpenCode-shaped model");
    }
    let dsh = app == AppId::Dsh;
    let mut fill = OpenCodeModelFill::default();
    let mut record_source = |field| {
        if let Some(source) = resolved.provenance.get(&field) {
            fill.provenance.insert(field, source.clone());
        }
    };

    let current_limit = model.get("limit");
    if current_limit.is_none_or(|limit| limit.is_null() || limit.is_object()) {
        let existing = current_limit.and_then(Value::as_object);
        let mut updated: Option<Map<String, Value>> = None;
        for (key, dsh_alias, field, tokens) in [
            ("context", "contextWindow", ContextWindow, resolved.context_window),
            ("output", "maxTokens", MaxOutputTokens, resolved.max_output_tokens),
        ] {
            if blank(existing.and_then(|limit| limit.get(key)))
                && (!dsh || blank(model.get(dsh_alias)))
                && let Some(tokens) =
                    tokens.filter(|tokens| (1..=MAX_NATIVE_LIMIT).contains(tokens))
            {
                let limit = updated.get_or_insert_with(|| existing.cloned().unwrap_or_default());
                limit.insert(key.into(), tokens.into());
                record_source(field);
            }
        }
        if let Some(limit) = updated {
            fill.changes.insert("limit".into(), Value::Object(limit));
        }
    }

    let explicit_modalities = model.contains_key("modalities")
        || (app == AppId::OpenCode && model.contains_key("capabilities"))
        || (dsh && model.contains_key("input"));
    if !explicit_modalities && let Some(input) = &resolved.input_modalities {
        let output = resolved.output_modalities.as_ref().map_or_else(
            || json!(["text"]),
            |output| {
                record_source(OutputModalities);
                json!(output)
            },
        );
        fill.changes.insert("modalities".into(), json!({ "input": input, "output": output }));
        record_source(InputModalities);
    }

    // An absent flag leaves runtime inference available. DSH also accepts explicit efforts.
    if resolved.reasoning == Supported
        && !model.contains_key("reasoning")
        && !(dsh && model.contains_key("reasoningEfforts"))
    {
        fill.changes.insert("reasoning".into(), true.into());
        record_source(Reasoning);
    }

    Ok(fill)
}

fn blank(value: Option<&Value>) -> bool {
    value.is_none_or(|value| {
        value.is_null() || value.as_str().is_some_and(|text| text.trim().is_empty())
    })
}
