use super::fill_opencode::{OpenCodeModelFill, fill_opencode_model};
use crate::providers::{dsh, mcode, opencode};
use serde_json::{Value, json};
use vw_api_types::{
    agent_apps::AppId::{self, Dsh, Mcode, OpenCode},
    app_providers::{
        dsh::{DshActivation, DshProfile},
        mcode::McodeProfile,
        opencode::{OpenCodeFormat, OpenCodeProfile},
    },
    common::TimestampMs,
    model_catalog::{
        ModelCapabilityState::{Supported, Unsupported},
        ModelMetadataDto,
        ModelMetadataField::{
            ContextWindow, InputModalities, MaxOutputTokens, OutputModalities, Reasoning, ToolCalls,
        },
        ModelMetadataProvenance,
        ModelMetadataSource::{ModelsDev, Preset},
    },
};

fn draft_fill(app: AppId, row: &Value, resolved: &ModelMetadataDto) -> OpenCodeModelFill {
    fill_opencode_model(app, row.as_object().unwrap(), resolved).unwrap()
}

fn full_metadata() -> ModelMetadataDto {
    ModelMetadataDto {
        context_window: Some(262_144),
        max_output_tokens: Some(32_768),
        input_modalities: Some(vec!["text".into(), "image".into(), "video".into()]),
        output_modalities: Some(vec!["text".into(), "audio".into()]),
        reasoning: Supported,
        ..Default::default()
    }
}

fn merge(row: &Value, fill: OpenCodeModelFill) -> Value {
    let mut draft = row.clone();
    draft.as_object_mut().unwrap().extend(fill.changes);
    draft
}

#[test]
fn catalog_fill_opencode_preserves_extensions() {
    let mut profile = OpenCodeProfile {
        name: "Fixture".into(),
        format: OpenCodeFormat::V1,
        declaration: json!({"models": {"vendor/model": {
            "name": "Custom",
            "limit": {"context": 1000, "input": 500, "extension": {"custom": 7}},
            "modalities": {"input": ["text"], "extension": true},
            "reasoning": false,
            "variants": {"manual": {"options": {"effort": "low"}}},
            "extension": {"key": "custom value"}
        }}}),
    };
    let row = &profile.declaration["models"]["vendor/model"];
    let fill = draft_fill(OpenCode, row, &full_metadata());
    let mut expected = row.clone();
    expected["limit"]["output"] = json!(32_768);
    let filled = merge(row, fill);
    assert_eq!(filled, expected);
    assert!(row["limit"].get("output").is_none());
    assert_eq!(draft_fill(OpenCode, &filled, &full_metadata()), OpenCodeModelFill::default());
    profile.declaration["models"]["vendor/model"] = filled;
    opencode::validate(&profile).unwrap();
}

#[test]
fn catalog_fill_opencode_adds_only_supported_native_fields() {
    let mut metadata = full_metadata();
    metadata.reasoning_efforts = Some(vec!["high".into()]);
    metadata.tool_calls = Supported;
    let row = json!({"name": "Custom"});
    let expected = json!({
        "limit": {"context": 262_144, "output": 32_768},
        "modalities": {"input": ["text", "image", "video"], "output": ["text", "audio"]},
        "reasoning": true
    });
    for app in [OpenCode, Mcode, Dsh] {
        assert_eq!(Value::Object(draft_fill(app, &row, &metadata).changes), expected);
    }
    let mcode = McodeProfile {
        name: "Fixture".into(),
        declaration: json!({
            "options": {"baseURL": "https://example.com/v1", "apiKey": "fixture-key"},
            "models": {"vendor/model": merge(&row, draft_fill(Mcode, &row, &metadata))}
        }),
    };
    mcode::validate(&mcode).unwrap();
    assert!(fill_opencode_model(AppId::Hermes, row.as_object().unwrap(), &metadata).is_err());
}

#[test]
fn catalog_fill_opencode_fills_blank_limits_independently() {
    let metadata = full_metadata();
    for blank in [Value::Null, json!(" \t")] {
        let row = json!({"limit": {"context": blank, "output": 50}});
        let fill = draft_fill(OpenCode, &row, &metadata);
        assert_eq!(fill.changes["limit"], json!({"context": 262_144, "output": 50}));
    }
    let row = json!({"limit": {"context": 0, "output": 50}, "modalities": {}, "reasoning": true});
    assert_eq!(draft_fill(OpenCode, &row, &metadata), OpenCodeModelFill::default());
    for limit in [Value::Null, json!({})] {
        let row = json!({"limit": limit});
        assert_eq!(
            draft_fill(OpenCode, &row, &metadata).changes["limit"],
            json!({"context": 262_144, "output": 32_768})
        );
    }
    let row = json!({"limit": "opaque"});
    assert!(!draft_fill(OpenCode, &row, &metadata).changes.contains_key("limit"));
}

#[test]
fn catalog_fill_opencode_defaults_output_only_when_adding_modalities() {
    let metadata = ModelMetadataDto {
        input_modalities: Some(vec!["image".into()]),
        reasoning: Unsupported,
        ..Default::default()
    };
    let row = json!({});
    assert_eq!(
        Value::Object(draft_fill(OpenCode, &row, &metadata).changes),
        json!({"modalities": {"input": ["image"], "output": ["text"]}})
    );
    for explicit in [Value::Null, json!({}), json!({"input": []})] {
        let row = json!({"modalities": explicit});
        assert_eq!(draft_fill(OpenCode, &row, &metadata), OpenCodeModelFill::default());
    }
    let row = json!({"capabilities": {"tools": false, "input": ["text"], "output": ["text"]}});
    assert_eq!(draft_fill(OpenCode, &row, &metadata), OpenCodeModelFill::default());
}

#[test]
fn catalog_fill_opencode_ignores_unavailable_or_invalid_metadata() {
    let row = json!({});
    let metadata = ModelMetadataDto {
        context_window: Some(0),
        max_output_tokens: Some(u64::MAX),
        output_modalities: Some(vec!["image".into()]),
        reasoning: Unsupported,
        ..Default::default()
    };
    assert_eq!(draft_fill(OpenCode, &row, &metadata), OpenCodeModelFill::default());
    assert_eq!(
        draft_fill(OpenCode, &row, &ModelMetadataDto::default()),
        OpenCodeModelFill::default()
    );
}

#[test]
fn catalog_fill_opencode_reports_only_actual_contributors() {
    let preset = ModelMetadataProvenance { source: Preset, fetched_at: None };
    let snapshot =
        ModelMetadataProvenance { source: ModelsDev, fetched_at: Some(TimestampMs(1234)) };
    let mut metadata = full_metadata();
    metadata.provenance = [
        (ContextWindow, snapshot.clone()),
        (MaxOutputTokens, preset.clone()),
        (InputModalities, snapshot.clone()),
        (OutputModalities, preset.clone()),
        (Reasoning, snapshot.clone()),
        (ToolCalls, preset.clone()),
    ]
    .into();
    let row = json!({"limit": {"context": 1000}, "reasoning": false});
    assert_eq!(
        draft_fill(OpenCode, &row, &metadata).provenance,
        [
            (MaxOutputTokens, preset.clone()),
            (InputModalities, snapshot.clone()),
            (OutputModalities, preset.clone()),
        ]
        .into()
    );
    metadata.output_modalities = None;
    assert_eq!(
        draft_fill(OpenCode, &row, &metadata).provenance,
        [(MaxOutputTokens, preset), (InputModalities, snapshot)].into()
    );
}

#[test]
fn catalog_fill_opencode_preserves_dsh_native_aliases_at_publication() {
    let row = json!({
        "contextWindow": 1000,
        "input": ["text"],
        "reasoningEfforts": false,
        "extension": "kept"
    });
    let fill = draft_fill(Dsh, &row, &full_metadata());
    assert_eq!(Value::Object(fill.changes.clone()), json!({"limit": {"output": 32_768}}));
    let profile = DshProfile {
        name: "Fixture".into(),
        declaration: json!({
            "api": "openai-completions",
            "options": {"baseURL": "https://example.com/v1", "apiKey": "fixture-key"},
            "models": {"vendor/model": merge(&row, fill)}
        }),
    };
    let pre = [("web".into(), None)].into();
    let planned = dsh::plan(&pre, "fixture", &profile, DshActivation::Add).unwrap();
    let web: Value = serde_yaml::from_slice(planned["web"].as_deref().unwrap()).unwrap();
    let models = &web[0]["config"]["providers"]["fixture"]["models"];
    assert_eq!(
        models[0],
        json!({
            "id": "vendor/model",
            "contextWindow": 1000,
            "maxTokens": 32_768,
            "input": ["text"],
            "reasoningEfforts": false,
            "extension": "kept"
        })
    );
}
