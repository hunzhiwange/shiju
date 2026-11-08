use super::fill_hermes::{HermesModelFill, fill_hermes_model};
use crate::providers::hermes::normalize_declaration;
use serde_json::{Value, json};
use vw_api_types::{
    app_providers::hermes::HermesProfile,
    common::TimestampMs,
    model_catalog::{
        ModelCapabilityState::Supported,
        ModelMetadataDto,
        ModelMetadataField::{ContextWindow, InputModalities, MaxOutputTokens},
        ModelMetadataProvenance,
        ModelMetadataSource::{ModelsDev, Preset},
        ModelTokenCost,
    },
};

#[test]
fn catalog_fill_hermes_fills_blank_context_length() {
    let resolved = ModelMetadataDto { context_window: Some(262_144), ..Default::default() };
    for model in [json!({}), json!({"context_length": null}), json!({"context_length": " \t"})] {
        let fill = fill_hermes_model(model.as_object().unwrap(), &resolved);
        assert_eq!(Value::Object(fill.changes), json!({"context_length": 262144}));
    }

    for value in [json!(128_000), json!("128000"), json!(0)] {
        let model = json!({"context_length": value});
        assert_eq!(
            fill_hermes_model(model.as_object().unwrap(), &resolved),
            HermesModelFill::default()
        );
    }
}

#[test]
fn catalog_fill_hermes_only_suggests_vision_for_image_input() {
    let model = json!({});
    for modalities in [None, Some(vec![]), Some(vec!["text".into(), "video".into()])] {
        let resolved = ModelMetadataDto {
            input_modalities: modalities,
            output_modalities: Some(vec!["image".into()]),
            ..Default::default()
        };
        assert_eq!(
            fill_hermes_model(model.as_object().unwrap(), &resolved),
            HermesModelFill::default()
        );
    }

    let resolved =
        ModelMetadataDto { input_modalities: Some(vec!["image".into()]), ..Default::default() };
    let fill = fill_hermes_model(model.as_object().unwrap(), &resolved);
    assert_eq!(Value::Object(fill.changes), json!({"supports_vision": true}));
}

#[test]
fn catalog_fill_hermes_preserves_explicit_vision_settings() {
    let resolved = ModelMetadataDto {
        context_window: Some(262_144),
        input_modalities: Some(vec!["image".into()]),
        ..Default::default()
    };
    for enabled in [false, true] {
        let model = json!({"supports_vision": enabled});
        let fill = fill_hermes_model(model.as_object().unwrap(), &resolved);
        assert_eq!(Value::Object(fill.changes), json!({"context_length": 262144}));
    }
}

#[test]
fn catalog_fill_hermes_ignores_unavailable_windows_and_their_sources() {
    let model = json!({});
    for context_window in [None, Some(0)] {
        let resolved = ModelMetadataDto {
            context_window,
            provenance: [(
                ContextWindow,
                ModelMetadataProvenance { source: Preset, fetched_at: None },
            )]
            .into(),
            ..Default::default()
        };
        assert_eq!(
            fill_hermes_model(model.as_object().unwrap(), &resolved),
            HermesModelFill::default()
        );
    }
}

#[test]
fn catalog_fill_hermes_reports_sources_only_for_native_changes() {
    let profile = HermesProfile {
        name: "Example".into(),
        declaration: json!({"models": [{"id": "group/model", "name": "Custom", "custom": 7}]}),
    };
    let native = normalize_declaration(&profile.declaration).unwrap();
    let model = native["models"]["group/model"].as_object().unwrap();
    let preset = ModelMetadataProvenance { source: Preset, fetched_at: None };
    let snapshot =
        ModelMetadataProvenance { source: ModelsDev, fetched_at: Some(TimestampMs(1234)) };
    let resolved = ModelMetadataDto {
        context_window: Some(262_144),
        max_output_tokens: Some(32_768),
        input_modalities: Some(vec!["text".into(), "image".into()]),
        output_modalities: Some(vec!["text".into()]),
        reasoning: Supported,
        cost: Some(ModelTokenCost { input: 1.0, output: 4.0, cache_read: None, cache_write: None }),
        provenance: [
            (ContextWindow, preset.clone()),
            (InputModalities, snapshot.clone()),
            (MaxOutputTokens, snapshot.clone()),
        ]
        .into(),
        ..Default::default()
    };

    let fill = fill_hermes_model(model, &resolved);
    assert_eq!(
        Value::Object(fill.changes.clone()),
        json!({"context_length": 262144, "supports_vision": true})
    );
    assert_eq!(fill.provenance, [(ContextWindow, preset), (InputModalities, snapshot)].into());

    let mut updated = model.clone();
    updated.extend(fill.changes);
    assert_eq!(updated["name"], "Custom");
    assert_eq!(updated["custom"], 7);
    assert!(!model.contains_key("context_length"));
    assert!(!model.contains_key("supports_vision"));
    assert_eq!(fill_hermes_model(&updated, &resolved), HermesModelFill::default());
}
