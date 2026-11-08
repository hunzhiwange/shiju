use super::fill_kimi::{KimiModelFill, fill_kimi_model};
use crate::providers::kimi;
use serde_json::{Value, json};
use vw_api_types::{
    app_providers::kimi::{KimiActivation, KimiProfile},
    common::TimestampMs,
    model_catalog::{
        ModelCapabilityState::{Supported, Unsupported},
        ModelMetadataDto,
        ModelMetadataField::{ContextWindow, InputModalities, MaxOutputTokens, Reasoning},
        ModelMetadataProvenance,
        ModelMetadataSource::{ModelsDev, Preset},
    },
};

fn metadata() -> ModelMetadataDto {
    ModelMetadataDto {
        context_window: Some(1_048_576),
        max_output_tokens: Some(32_768),
        input_modalities: Some(vec!["text".into(), "image".into(), "video".into()]),
        output_modalities: Some(vec!["text".into()]),
        reasoning: Supported,
        tool_calls: Supported,
        ..Default::default()
    }
}

fn fill(row: &Value, resolved: &ModelMetadataDto, placeholder: bool) -> KimiModelFill {
    fill_kimi_model(row.as_object().unwrap(), resolved, placeholder)
}

#[test]
fn catalog_fill_kimi_merges_capabilities_without_loss() {
    let mut profile = KimiProfile {
        name: "Fixture".into(),
        declaration: json!({
            "provider": {"type": "openai", "api_key_env": "FIXTURE_KEY"},
            "models": {"alias": {
                "model": "vendor/model",
                "display_name": "Custom name",
                "max_context_size": 1_000,
                "capabilities": ["tool_use", "future", "tool_use"],
                "overrides": {"capabilities": ["video_in"], "temperature": 0.4},
                "extension": {"custom": 7}
            }}
        }),
    };
    let original = profile.declaration["models"]["alias"].clone();
    let mut expected = original.clone();
    expected["capabilities"] = json!(["tool_use", "future", "tool_use", "image_in", "thinking"]);
    let mut merged = original.clone();
    merged.as_object_mut().unwrap().extend(fill(&original, &metadata(), false).changes);
    assert_eq!(merged, expected);
    assert_eq!(profile.declaration["models"]["alias"], original);
    assert_eq!(fill(&merged, &metadata(), false), KimiModelFill::default());

    profile.declaration["models"]["alias"] = merged;
    let bytes =
        kimi::patch(None, "fixture", Some(&profile.declaration), KimiActivation::Add).unwrap();
    let declarations = kimi::declarations(Some(&bytes)).unwrap();
    expected["provider"] = json!("fixture");
    assert_eq!(declarations["fixture"].declaration["models"]["alias"], expected);
}

#[test]
fn catalog_fill_kimi_fills_only_blank_or_marked_context() {
    let resolved = ModelMetadataDto { context_window: Some(1_048_576), ..Default::default() };
    let expected = json!({"max_context_size": 1_048_576});
    for row in [
        json!({"model": "m"}),
        json!({"max_context_size": null}),
        json!({"max_context_size": " \t"}),
    ] {
        assert_eq!(Value::Object(fill(&row, &resolved, false).changes), expected);
    }
    let row = json!({"max_context_size": 262_144});
    assert_eq!(Value::Object(fill(&row, &resolved, true).changes), expected);
    assert_eq!(fill(&row, &resolved, false), KimiModelFill::default());
    assert_eq!(fill(&json!({"max_context_size": 1}), &resolved, false), KimiModelFill::default());
}

#[test]
fn catalog_fill_kimi_skips_unavailable_unrepresentable_and_unchanged_windows() {
    let blank = json!({"model": "m"});
    for context_window in [None, Some(0), Some(i64::MAX as u64 + 1)] {
        let resolved = ModelMetadataDto { context_window, ..Default::default() };
        assert_eq!(fill(&blank, &resolved, false), KimiModelFill::default());
    }
    let resolved = ModelMetadataDto { context_window: Some(262_144), ..Default::default() };
    assert_eq!(
        fill(&json!({"max_context_size": 262_144}), &resolved, true),
        KimiModelFill::default()
    );
}

#[test]
fn catalog_fill_kimi_adds_only_confirmed_native_capabilities() {
    let mut resolved = metadata();
    let row = json!({"max_context_size": 1_000});
    assert_eq!(
        Value::Object(fill(&row, &resolved, false).changes),
        json!({"capabilities": ["image_in", "thinking"]})
    );
    resolved.reasoning = Unsupported;
    assert_eq!(
        Value::Object(fill(&row, &resolved, false).changes),
        json!({"capabilities": ["image_in"]})
    );
    resolved.input_modalities = Some(vec!["text".into(), "video".into()]);
    let configured = json!({"max_context_size": 1_000, "capabilities": ["thinking", "video_in"]});
    assert_eq!(fill(&configured, &resolved, false), KimiModelFill::default());
    resolved.reasoning = Default::default();
    resolved.input_modalities = None;
    assert_eq!(fill(&row, &resolved, false), KimiModelFill::default());
}

#[test]
fn catalog_fill_kimi_keeps_malformed_capability_values() {
    for capabilities in [json!("future"), json!(["future", 42])] {
        let row = json!({"max_context_size": 1_000, "capabilities": capabilities});
        assert_eq!(fill(&row, &metadata(), false), KimiModelFill::default());
    }
}

#[test]
fn catalog_fill_kimi_reports_only_contributing_metadata() {
    let mut resolved = metadata();
    let fetched = ModelMetadataProvenance {
        source: ModelsDev,
        fetched_at: Some(TimestampMs(1_760_000_000_000)),
    };
    let preset = ModelMetadataProvenance { source: Preset, fetched_at: None };
    resolved.provenance = [
        (ContextWindow, fetched.clone()),
        (InputModalities, preset.clone()),
        (Reasoning, fetched.clone()),
        (MaxOutputTokens, preset),
    ]
    .into();
    let row = json!({"capabilities": ["image_in"]});
    let result = fill(&row, &resolved, false);
    assert_eq!(
        Value::Object(result.changes),
        json!({"max_context_size": 1_048_576, "capabilities": ["image_in", "thinking"]})
    );
    assert_eq!(result.provenance, [(ContextWindow, fetched.clone()), (Reasoning, fetched)].into());
}
