use super::*;
use crate::test_support::test_model;
use serde_json::json;

#[test]
fn live_shiju_metadata_overrides_the_snapshot_field_by_field() {
    let entry = serde_json::from_value(json!({
        "id": "fixture",
        "supported_endpoint_types": ["openai-video"],
        "capabilities": {"supports_image": false, "supports_tool_calling": false},
        "limit": {"context": 64_000, "input": 48_000},
        "max_output_tokens": 16_000,
    }))
    .unwrap();
    let mut model = test_model("fixture", "disabled");
    let config = json!({"status": "disabled", "limit": {"input": 24_000}});

    assert!(apply(&mut model, &entry, Some(&config)));
    assert_eq!(model.status, "disabled");
    assert!(!model.capabilities.input.image);
    assert!(!model.capabilities.attachment);
    assert!(model.capabilities.output.video);
    assert!(!model.capabilities.output.text);
    assert!(!model.capabilities.toolcall);
    assert_eq!(model.limit.context, 64_000);
    assert_eq!(model.limit.input, Some(24_000));
    assert_eq!(model.limit.output, 16_000);
    assert!(!apply(&mut model, &entry, Some(&config)));
}

#[test]
fn explicit_modalities_take_precedence_over_endpoint_hints() {
    let entry = serde_json::from_value(json!({
        "id": "fixture",
        "supported_endpoint_types": ["image-generation", "openai"],
        "modalities": {"input": ["image"], "output": []},
        "context_window": "invalid",
    }))
    .unwrap();
    let mut model = test_model("fixture", "active");
    let config = json!({"modalities": {"input": ["text", "image"]}});

    assert!(apply(&mut model, &entry, Some(&config)));
    assert!(model.capabilities.input.text);
    assert!(model.capabilities.input.image);
    assert!(!model.capabilities.output.text);
    assert!(!model.capabilities.output.image);
    assert_eq!(model.limit.context, 32_000);
}
