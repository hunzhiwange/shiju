//! Read-only Codex model metadata fill suggestions.

use std::collections::BTreeMap;
use vw_api_types::model_catalog::{
    CodexNativeModel, ModelMetadataDto, ModelMetadataField, ModelMetadataProvenance,
};

const CODEX_REASONING_LEVELS: [&str; 8] =
    ["none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra"];

/// Only the fields that can be added to the draft.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CodexModelFill {
    pub context_window: Option<String>,
    pub reasoning_levels: Option<Vec<String>>,
    pub input_modalities: Option<Vec<String>>,
    pub provenance: BTreeMap<ModelMetadataField, ModelMetadataProvenance>,
}

/// Compute a draft patch from metadata already resolved for this Codex deployment and model.
/// `current` holds the row's existing catalog metadata alongside its saved native fields.
pub fn fill_codex_catalog_model(
    row: &CodexNativeModel,
    current: &ModelMetadataDto,
    resolved: &ModelMetadataDto,
) -> CodexModelFill {
    use ModelMetadataField::{ContextWindow, InputModalities, ReasoningEfforts};

    let mut fill = CodexModelFill::default();
    let mut record = |field| {
        if let Some(source) = resolved.provenance.get(&field) {
            fill.provenance.insert(field, source.clone());
        }
    };

    if row.context_window.is_none()
        && current.context_window.is_none()
        && let Some(window) = resolved.context_window.filter(|window| *window > 0)
    {
        fill.context_window = Some(window.to_string());
        record(ContextWindow);
    }

    if current.reasoning_efforts.as_ref().is_none_or(Vec::is_empty)
        && let Some(efforts) = &resolved.reasoning_efforts
    {
        let levels: Vec<_> = CODEX_REASONING_LEVELS
            .into_iter()
            .filter(|level| efforts.iter().any(|effort| effort == level))
            .map(String::from)
            .collect();
        if !levels.is_empty() {
            fill.reasoning_levels = Some(levels);
            record(ReasoningEfforts);
        }
    }

    // A text-only suggestion would disable Codex's image input.
    if current.input_modalities.is_none()
        && resolved
            .input_modalities
            .as_ref()
            .is_some_and(|modalities| modalities.iter().any(|mode| mode == "image"))
    {
        fill.input_modalities = Some(vec!["text".into(), "image".into()]);
        record(InputModalities);
    }

    fill
}
