//! Read-only Hermes model metadata fill suggestions.

use serde_json::{Map, Value};
use std::collections::BTreeMap;
use vw_api_types::model_catalog::{ModelMetadataDto, ModelMetadataField, ModelMetadataProvenance};

/// Hermes model keys to merge into the draft, with sources for the changed fields.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct HermesModelFill {
    pub changes: Map<String, Value>,
    pub provenance: BTreeMap<ModelMetadataField, ModelMetadataProvenance>,
}

/// Compare a native `HermesProfile::declaration.models` row with metadata resolved for
/// this Hermes deployment and model.
pub fn fill_hermes_model(
    model: &Map<String, Value>,
    resolved: &ModelMetadataDto,
) -> HermesModelFill {
    use ModelMetadataField::{ContextWindow, InputModalities};

    let mut fill = HermesModelFill::default();
    let mut add = |key: &str, field, value| {
        fill.changes.insert(key.into(), value);
        if let Some(source) = resolved.provenance.get(&field) {
            fill.provenance.insert(field, source.clone());
        }
    };

    if model.get("context_length").is_none_or(|value| {
        value.is_null() || value.as_str().is_some_and(|text| text.trim().is_empty())
    }) && let Some(window) = resolved.context_window.filter(|window| *window > 0)
    {
        add("context_length", ContextWindow, window.into());
    }

    // Preserve explicit settings. Hermes can infer support when the key is absent.
    if !model.contains_key("supports_vision")
        && resolved
            .input_modalities
            .as_ref()
            .is_some_and(|modalities| modalities.iter().any(|mode| mode == "image"))
    {
        add("supports_vision", InputModalities, true.into());
    }

    fill
}
