use super::fill_codex::{CodexModelFill, fill_codex_catalog_model};
use vw_api_types::{
    common::TimestampMs,
    model_catalog::{
        CodexNativeModel, ModelCapabilityState::Supported, ModelMetadataDto, ModelMetadataField::*,
        ModelMetadataProvenance, ModelMetadataSource::*, ModelTokenCost,
    },
};

fn native_row(context_window: Option<u64>) -> CodexNativeModel {
    CodexNativeModel {
        slug: "vendor/model".into(),
        display_name: "Custom name".into(),
        description: "Custom description".into(),
        base_instructions: "Custom instructions".into(),
        supports_reasoning_summaries: false,
        supports_parallel_tool_calls: true,
        context_window,
        supported_reasoning_levels: vec![],
    }
}

fn resolved() -> ModelMetadataDto {
    let snapshot =
        ModelMetadataProvenance { source: ModelsDev, fetched_at: Some(TimestampMs(1234)) };
    ModelMetadataDto {
        context_window: Some(262_144),
        reasoning_efforts: Some(
            ["ultra", "max", "low", "turbo", "high", "high", "minimal"].map(String::from).into(),
        ),
        input_modalities: Some(["text", "image", "video"].map(String::from).into()),
        max_output_tokens: Some(32_768),
        reasoning: Supported,
        tool_calls: Supported,
        output_modalities: Some(vec!["audio".into()]),
        cost: Some(ModelTokenCost { input: 1.0, output: 2.0, cache_read: None, cache_write: None }),
        provenance: [
            (ContextWindow, ModelMetadataProvenance { source: Preset, fetched_at: None }),
            (ReasoningEfforts, snapshot.clone()),
            (InputModalities, snapshot.clone()),
            (MaxOutputTokens, snapshot.clone()),
            (Reasoning, snapshot.clone()),
            (ToolCalls, snapshot.clone()),
            (OutputModalities, snapshot.clone()),
            (Cost, snapshot),
        ]
        .into(),
    }
}

#[test]
fn catalog_fill_codex_preserves_user_context() {
    let row = native_row(Some(128_000));
    let before = row.clone();
    let current = ModelMetadataDto {
        reasoning_efforts: Some(vec![]),
        input_modalities: Some(vec!["text".into()]),
        ..Default::default()
    };
    let metadata = resolved();
    let fill = fill_codex_catalog_model(&row, &current, &metadata);
    assert_eq!(
        fill,
        CodexModelFill {
            reasoning_levels: Some(
                ["minimal", "low", "high", "max", "ultra"].map(String::from).into()
            ),
            provenance: [(ReasoningEfforts, metadata.provenance[&ReasoningEfforts].clone())].into(),
            ..Default::default()
        }
    );
    assert_eq!(row, before);
}

#[test]
fn catalog_fill_codex_fills_blank_fields_with_contributing_sources() {
    let metadata = resolved();
    let fill = fill_codex_catalog_model(&native_row(None), &ModelMetadataDto::default(), &metadata);
    assert_eq!(
        fill,
        CodexModelFill {
            context_window: Some("262144".into()),
            reasoning_levels: Some(
                ["minimal", "low", "high", "max", "ultra"].map(String::from).into()
            ),
            input_modalities: Some(["text", "image"].map(String::from).into()),
            provenance: [ContextWindow, ReasoningEfforts, InputModalities]
                .map(|field| (field, metadata.provenance[&field].clone()))
                .into(),
        }
    );
}

#[test]
fn catalog_fill_codex_keeps_explicit_values_even_with_image_metadata() {
    let metadata = resolved();
    for input in [vec!["text".into()], vec![]] {
        let current = ModelMetadataDto {
            context_window: Some(128_000),
            reasoning_efforts: Some(vec!["custom".into()]),
            input_modalities: Some(input),
            ..Default::default()
        };
        assert_eq!(
            fill_codex_catalog_model(&native_row(None), &current, &metadata),
            CodexModelFill::default()
        );
    }
}

#[test]
fn catalog_fill_codex_keeps_image_entry_for_text_only_metadata() {
    let mut metadata = resolved();
    metadata.reasoning_efforts = Some(vec!["turbo".into()]);
    metadata.input_modalities = Some(["text", "video"].map(String::from).into());
    assert_eq!(
        fill_codex_catalog_model(&native_row(None), &ModelMetadataDto::default(), &metadata),
        CodexModelFill {
            context_window: Some("262144".into()),
            provenance: [(ContextWindow, metadata.provenance[&ContextWindow].clone())].into(),
            ..Default::default()
        }
    );
}

#[test]
fn catalog_fill_codex_skips_zero_context_and_absent_metadata() {
    let row = native_row(None);
    let current = ModelMetadataDto::default();
    let mut metadata = resolved();
    metadata.context_window = Some(0);
    metadata.reasoning_efforts = None;
    metadata.input_modalities = None;
    assert_eq!(fill_codex_catalog_model(&row, &current, &metadata), CodexModelFill::default());
    assert_eq!(fill_codex_catalog_model(&row, &current, &current), CodexModelFill::default());
}

#[test]
fn catalog_fill_codex_keeps_unknown_provenance_absent() {
    let metadata =
        ModelMetadataDto { input_modalities: Some(vec!["image".into()]), ..Default::default() };
    assert_eq!(
        fill_codex_catalog_model(&native_row(None), &ModelMetadataDto::default(), &metadata),
        CodexModelFill {
            input_modalities: Some(["text", "image"].map(String::from).into()),
            ..Default::default()
        }
    );
}
