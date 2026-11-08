//! Read-only metadata fill for Kimi model drafts.

use serde_json::{Map, Value};
use std::collections::BTreeMap;
use vw_api_types::model_catalog::{
    ModelCapabilityState, ModelMetadataDto, ModelMetadataField, ModelMetadataProvenance,
};

/// Kimi model keys to merge into the draft, with sources for the changed fields.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct KimiModelFill {
    pub changes: Map<String, Value>,
    pub provenance: BTreeMap<ModelMetadataField, ModelMetadataProvenance>,
}

/// Compare a `KimiProfile::declaration.models` draft row with resolved metadata.
/// The caller marks a placeholder only for an untouched context in a new draft.
pub fn fill_kimi_model(
    model: &Map<String, Value>,
    resolved: &ModelMetadataDto,
    context_is_placeholder: bool,
) -> KimiModelFill {
    use ModelMetadataField::{ContextWindow, InputModalities, Reasoning};

    let mut fill = KimiModelFill::default();
    let mut record_source = |field| {
        if let Some(source) = resolved.provenance.get(&field) {
            fill.provenance.insert(field, source.clone());
        }
    };

    let context = model.get("max_context_size");
    let context_is_blank = context.is_none_or(|value| {
        value.is_null() || value.as_str().is_some_and(|text| text.trim().is_empty())
    });
    if (context_is_blank || context_is_placeholder)
        && let Some(window) =
            resolved.context_window.filter(|window| (1..=i64::MAX as u64).contains(window))
        && context.and_then(Value::as_u64) != Some(window)
    {
        fill.changes.insert("max_context_size".into(), window.into());
        record_source(ContextWindow);
    }

    let existing = match model.get("capabilities") {
        None => &[][..],
        Some(Value::Array(values)) if values.iter().all(Value::is_string) => values.as_slice(),
        Some(_) => return fill,
    };
    let supports_image = resolved
        .input_modalities
        .as_ref()
        .is_some_and(|modalities| modalities.iter().any(|mode| mode == "image"));
    let mut additions = Vec::new();
    for (capability, supported, field) in [
        ("image_in", supports_image, InputModalities),
        ("thinking", resolved.reasoning == ModelCapabilityState::Supported, Reasoning),
    ] {
        if supported && !existing.iter().any(|value| value.as_str() == Some(capability)) {
            additions.push(Value::from(capability));
            record_source(field);
        }
    }
    if !additions.is_empty() {
        let capabilities = existing.iter().cloned().chain(additions).collect();
        fill.changes.insert("capabilities".into(), Value::Array(capabilities));
    }

    fill
}
