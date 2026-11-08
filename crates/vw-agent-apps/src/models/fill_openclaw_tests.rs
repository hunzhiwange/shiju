use super::fill_openclaw::{OpenClawModelFill, fill_openclaw_model};
use serde_json::{Value, json};
use vw_api_types::{
    common::TimestampMs,
    model_catalog::{
        ModelCapabilityState::{Supported, Unsupported},
        ModelMetadataDto,
        ModelMetadataField::*,
        ModelMetadataProvenance,
        ModelMetadataSource::{ModelsDev, Preset},
        ModelTokenCost,
    },
};

fn resolved() -> ModelMetadataDto {
    let preset = ModelMetadataProvenance { source: Preset, fetched_at: None };
    let snapshot =
        ModelMetadataProvenance { source: ModelsDev, fetched_at: Some(TimestampMs(1_234)) };
    ModelMetadataDto {
        context_window: Some(262_144),
        max_output_tokens: Some(32_768),
        reasoning: Supported,
        input_modalities: Some(vec!["text".into(), "image".into(), "video".into()]),
        output_modalities: Some(vec!["text".into()]),
        reasoning_efforts: Some(vec!["high".into()]),
        tool_calls: Supported,
        cost: Some(ModelTokenCost {
            input: 1.0,
            output: 4.0,
            cache_read: Some(0.0),
            cache_write: Some(2.0),
        }),
        provenance: [
            (ContextWindow, preset.clone()),
            (MaxOutputTokens, snapshot.clone()),
            (Reasoning, preset.clone()),
            (InputModalities, preset),
            (OutputModalities, snapshot.clone()),
            (ReasoningEfforts, snapshot.clone()),
            (ToolCalls, snapshot.clone()),
            (Cost, snapshot),
        ]
        .into(),
    }
}

#[test]
fn catalog_fill_openclaw_preserves_user_cost() {
    let model = json!({
        "id": "custom/model",
        "name": "Custom",
        "contextWindow": 1000,
        "maxTokens": " ",
        "cost": {"input": 0, "output": 9, "custom": 1},
        "compat": {"supportsTools": false}
    });
    let metadata = resolved();
    let fill = fill_openclaw_model(model.as_object().unwrap(), &metadata);
    assert_eq!(model["cost"], json!({"input": 0, "output": 9, "custom": 1}));
    assert_eq!(model["contextWindow"], 1000);
    assert_eq!(
        Value::Object(fill.changes),
        json!({"maxTokens": 32768, "reasoning": true, "input": ["text", "image"]})
    );
    assert_eq!(
        fill.provenance,
        [MaxOutputTokens, Reasoning, InputModalities]
            .map(|field| (field, metadata.provenance[&field].clone()))
            .into()
    );
}

#[test]
fn catalog_fill_openclaw_fills_native_keys_and_reports_only_contributing_sources() {
    let model = json!({"id": "m", "contextWindow": null, "cost": null, "input": ["text"]});
    let metadata = resolved();
    let fill = fill_openclaw_model(model.as_object().unwrap(), &metadata);
    assert_eq!(
        Value::Object(fill.changes),
        json!({
            "contextWindow": 262144,
            "maxTokens": 32768,
            "reasoning": true,
            "input": ["text", "image"],
            "cost": {"input": 1.0, "output": 4.0, "cacheRead": 0.0, "cacheWrite": 2.0}
        })
    );
    assert_eq!(
        fill.provenance,
        [ContextWindow, MaxOutputTokens, Reasoning, InputModalities, Cost]
            .map(|field| (field, metadata.provenance[&field].clone()))
            .into()
    );
}

#[test]
fn catalog_fill_openclaw_preserves_capabilities_and_nonblank_values() {
    let model = json!({
        "id": "m",
        "contextWindow": "128000",
        "maxTokens": 0,
        "cost": {},
        "reasoning": true,
        "input": ["image", "text", "audio"]
    });
    let mut metadata = resolved();
    metadata.reasoning = Unsupported;
    metadata.input_modalities = Some(vec!["text".into()]);
    assert_eq!(
        fill_openclaw_model(model.as_object().unwrap(), &metadata),
        OpenClawModelFill::default()
    );
}

#[test]
fn catalog_fill_openclaw_upgrades_false_and_appends_image_without_reordering() {
    let model = json!({"id": "m", "reasoning": false, "input": ["audio", "text"]});
    let metadata = ModelMetadataDto {
        reasoning: Supported,
        input_modalities: Some(vec!["image".into()]),
        ..Default::default()
    };
    let fill = fill_openclaw_model(model.as_object().unwrap(), &metadata);
    assert_eq!(
        Value::Object(fill.changes),
        json!({"reasoning": true, "input": ["audio", "text", "image"]})
    );
    assert!(fill.provenance.is_empty());
}

#[test]
fn catalog_fill_openclaw_fills_unsupported_reasoning_only_when_absent() {
    let metadata = ModelMetadataDto {
        reasoning: Unsupported,
        input_modalities: Some(vec!["video".into()]),
        ..Default::default()
    };
    for (model, expected) in [
        (json!({"id": "m", "input": []}), json!({"reasoning": false, "input": ["text"]})),
        (json!({"id": "m", "reasoning": null, "input": null}), json!({"input": ["text"]})),
    ] {
        let fill = fill_openclaw_model(model.as_object().unwrap(), &metadata);
        assert_eq!(Value::Object(fill.changes), expected);
    }
}

#[test]
fn catalog_fill_openclaw_skips_unknown_metadata_and_zero_limits() {
    let model = json!({"id": "m"});
    let mut metadata = ModelMetadataDto {
        context_window: Some(0),
        max_output_tokens: Some(0),
        ..Default::default()
    };
    metadata
        .provenance
        .insert(ContextWindow, ModelMetadataProvenance { source: Preset, fetched_at: None });
    assert_eq!(
        fill_openclaw_model(model.as_object().unwrap(), &metadata),
        OpenClawModelFill::default()
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn catalog_fill_openclaw_uses_resolved_price_only_for_the_matching_endpoint() {
    use super::metadata::resolve_model_metadata;
    use vw_api_types::agent_apps::AppId;
    use vw_provider_resolver::models::ModelsSnapshot;

    let snapshot = ModelsSnapshot {
        providers: serde_json::from_value(json!({
            "deepseek": {
                "api": "https://vendor.example.test",
                "models": {"deepseek-chat": {
                    "canonical_model_id": "deepseek/deepseek-chat",
                    "limit": {"context": 65536, "output": 8192},
                    "cost": {"input": 0, "output": 2}
                }}
            }
        }))
        .unwrap(),
        fetched_at: Some(std::time::UNIX_EPOCH),
    };
    let model = json!({"id": "deepseek-chat"});
    for endpoint in ["https://vendor.example.test", "https://relay.example.test"] {
        let metadata =
            resolve_model_metadata(AppId::OpenClaw, endpoint, "deepseek-chat", Some(&snapshot))
                .unwrap()
                .metadata;
        let fill = fill_openclaw_model(model.as_object().unwrap(), &metadata);
        assert_eq!(fill.changes.get("contextWindow"), Some(&json!(65536)));
        if endpoint == "https://vendor.example.test" {
            assert_eq!(fill.changes["cost"], json!({"input": 0.0, "output": 2.0}));
            assert_eq!(fill.provenance[&Cost].source, ModelsDev);
        } else {
            assert!(!fill.changes.contains_key("cost"));
            assert!(!fill.provenance.contains_key(&Cost));
        }
    }
}
