use super::capabilities::resolve_model_capabilities;
use serde_json::json;
use vw_api_types::model_catalog::{
    ModelCapabilityState::{Supported, Unknown, Unsupported},
    ModelMetadataDto,
};

#[test]
fn catalog_capabilities_explicit_support_overrides_registry() {
    let settings = json!({"models": {"qwen3-coder-plus": {"supportsImage": true}}});
    let metadata = ModelMetadataDto::default();
    let result = resolve_model_capabilities("qwen3-coder-plus", Some(&settings), &metadata, true);
    assert_eq!(result.image_input, Supported);
    assert!(!result.text_only);
    assert_eq!(
        resolve_model_capabilities("qwen3-coder-plus-vision", Some(&settings), &metadata, true)
            .image_input,
        Unknown
    );
}

#[test]
fn catalog_capabilities_reads_nested_and_top_level_settings_shapes() {
    let metadata = ModelMetadataDto::default();
    for settings in [
        json!({"modelCatalog": {"models": [{"model": "visual", "modalities": {"input": ["text", " IMAGE "]}}]}}),
        json!({"modelCatalog": {"visual": {"supports_image": true}}}),
        json!({"modelCatalog": [{"id": "visual", "input_modalities": ["image"]}]}),
        json!({"models": [{"name": "visual", "vision": true}]}),
        json!({"models": {"visual": {"inputModalities": ["image"]}}}),
        json!({"models": {"visual": {"input": ["image"]}}}),
    ] {
        let result = resolve_model_capabilities("visual", Some(&settings), &metadata, false);
        assert_eq!(result.image_input, Supported, "{settings}");
        assert!(!result.text_only, "{settings}");
    }
}

#[test]
fn catalog_capabilities_negative_image_flag_does_not_imply_text_only() {
    let settings = json!({"models": {
        "deepseek-chat": {"vision": false},
        "audio": {"supportsImage": false, "input": ["text", "audio"]},
        "text": {"inputModalities": [" TEXT "]}
    }});
    let metadata = ModelMetadataDto::default();
    for (model, text_only) in [("deepseek-chat", false), ("audio", false), ("text", true)] {
        let result = resolve_model_capabilities(model, Some(&settings), &metadata, true);
        assert_eq!(result.image_input, Unsupported, "{model}");
        assert_eq!(result.text_only, text_only, "{model}");
    }
}

#[test]
fn catalog_capabilities_handles_conflicts_in_one_declaration() {
    let settings = json!({"models": {"model": {"supportsImage": true, "input": ["text"]}}});
    let metadata = ModelMetadataDto::default();
    let result = resolve_model_capabilities("model", Some(&settings), &metadata, false);
    assert_eq!(result.image_input, Supported);
    assert!(!result.text_only);
}

#[test]
fn catalog_capabilities_reads_catalog_modalities_without_guessing() {
    for (modalities, state, text_only) in [
        (None, Unknown, false),
        (Some(vec![]), Unsupported, false),
        (Some(vec!["text".into()]), Unsupported, true),
        (Some(vec!["text".into(), "file".into()]), Unsupported, false),
        (Some(vec![" Image ".into()]), Supported, false),
    ] {
        let metadata = ModelMetadataDto { input_modalities: modalities, ..Default::default() };
        let result = resolve_model_capabilities("custom", None, &metadata, true);
        assert_eq!(result.image_input, state, "{metadata:?}");
        assert_eq!(result.text_only, text_only, "{metadata:?}");
    }
}

#[test]
fn catalog_capabilities_prefers_settings_and_keeps_exact_model_identity() {
    let settings = json!({
        "modelCatalog": {"models": [
            {"id": "other/model", "name": "vendor/model", "supportsImage": true},
            {"id": "vendor/model", "supportsImage": false}
        ]},
        "models": {"vendor/model": {"supportsImage": true}}
    });
    let metadata =
        ModelMetadataDto { input_modalities: Some(vec!["image".into()]), ..Default::default() };
    assert_eq!(
        resolve_model_capabilities("vendor/model", Some(&settings), &metadata, true).image_input,
        Unsupported
    );
    assert_eq!(
        resolve_model_capabilities(
            "third/model",
            Some(&settings),
            &ModelMetadataDto::default(),
            true
        )
        .image_input,
        Unknown
    );
}

#[test]
fn catalog_capabilities_uses_registry_only_for_confirmed_names() {
    let metadata = ModelMetadataDto::default();
    for model in [
        " models/GLM-5.2[1M] ",
        "deepseek/deepseek-v4-flash-0731",
        "Qwen/Qwen3-Coder-480B-A35B-Instruct",
    ] {
        let result = resolve_model_capabilities(model, None, &metadata, true);
        assert_eq!(result.image_input, Unsupported, "{model}");
        assert!(result.text_only, "{model}");
        assert_eq!(resolve_model_capabilities(model, None, &metadata, false).image_input, Unknown);
    }
    for model in [
        "deepseek/deepseek-v4-flash",
        "deepseek/deepseek-v4-pro",
        "glm-5.2v",
        "qwen3-coder-plus-vl",
        "deepseek-chat:custom",
        "gpt-5.4",
        "",
    ] {
        let result = resolve_model_capabilities(model, None, &metadata, true);
        assert_eq!(result.image_input, Unknown, "{model}");
        assert!(!result.text_only, "{model}");
    }
}

#[test]
fn catalog_capabilities_skips_malformed_settings_declarations() {
    let metadata = ModelMetadataDto::default();
    let settings = json!({"modelCatalog": [{"model": "custom", "input": ["text", 1]}]});
    let result = resolve_model_capabilities("custom", Some(&settings), &metadata, true);
    assert_eq!(result.image_input, Unknown);
    assert!(!result.text_only);
}

#[test]
fn catalog_capabilities_keeps_declared_tools_and_reasoning_efforts() {
    let metadata = ModelMetadataDto {
        tool_calls: Unsupported,
        reasoning: Supported,
        reasoning_efforts: Some(vec!["low".into(), "xhigh".into()]),
        ..Default::default()
    };
    let result = resolve_model_capabilities("custom", None, &metadata, true);
    assert_eq!(result.image_input, Unknown);
    assert_eq!(result.tool_calls, Unsupported);
    assert_eq!(result.reasoning, Supported);
    assert_eq!(result.reasoning_efforts, metadata.reasoning_efforts.as_deref());

    let metadata = ModelMetadataDto::default();
    let result = resolve_model_capabilities("custom", None, &metadata, true);
    assert_eq!(result.tool_calls, Unknown);
    assert_eq!(result.reasoning, Unknown);
    assert_eq!(result.reasoning_efforts, None);
}
