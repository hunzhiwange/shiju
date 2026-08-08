use super::*;
use serde_json::json;
use std::collections::HashMap;

fn raw_provider() -> RawProvider {
    RawProvider {
        api: Some("https://api.example.test/v1".to_string()),
        name: "Example".to_string(),
        env: vec!["EXAMPLE_API_KEY".to_string()],
        id: "example".to_string(),
        adapter: Some("provider-adapter".to_string()),
        models: HashMap::new(),
    }
}

fn raw_model(id: &str) -> RawModel {
    RawModel {
        id: id.to_string(),
        name: "Example Model".to_string(),
        family: Some("example-family".to_string()),
        release_date: "2026-01-02".to_string(),
        attachment: true,
        reasoning: true,
        temperature: true,
        tool_call: true,
        interleaved: None,
        cost: None,
        limit: models::ModelLimit { context: 128_000, input: Some(64_000), output: 8_192 },
        modalities: None,
        experimental: None,
        status: None,
        options: HashMap::new(),
        headers: None,
        provider: None,
        variants: None,
    }
}

#[test]
fn state_default_has_no_providers() {
    let state = State::default();

    assert!(state.providers.is_empty());
}

#[test]
fn normalize_adapter_uses_default_for_blank_input() {
    assert_eq!(normalize_adapter(""), default_adapter());
    assert_eq!(normalize_adapter("   "), default_adapter());
}

#[test]
fn normalize_adapter_maps_historical_aliases() {
    assert_eq!(normalize_adapter("acp"), default_adapter());
    assert_eq!(normalize_adapter("Agent-Client-Protocol"), default_adapter());
    assert_eq!(normalize_adapter("agent_client_protocol"), default_adapter());
}

#[test]
fn normalize_adapter_preserves_trimmed_custom_adapter() {
    assert_eq!(normalize_adapter("  CustomAdapter  "), "CustomAdapter");
}

#[test]
fn model_limit_resolution_uses_defaults_only_for_missing_or_zero_values() {
    let fallback = resolve_model_limit(None, None, None);

    assert_eq!(fallback.context, DEFAULT_MODEL_CONTEXT_LIMIT);
    assert_eq!(fallback.input, Some(DEFAULT_MODEL_INPUT_LIMIT));
    assert_eq!(fallback.output, DEFAULT_MODEL_OUTPUT_LIMIT);

    let explicit = resolve_model_limit(Some(12_345), Some(10_000), Some(2_048));
    assert_eq!(explicit.context, 12_345);
    assert_eq!(explicit.input, Some(10_000));
    assert_eq!(explicit.output, 2_048);

    let output_not_applicable = resolve_model_limit(Some(0), Some(0), Some(0));
    assert_eq!(output_not_applicable.context, DEFAULT_MODEL_CONTEXT_LIMIT);
    assert_eq!(output_not_applicable.input, Some(DEFAULT_MODEL_INPUT_LIMIT));
    assert_eq!(output_not_applicable.output, 0);
}

#[test]
fn effective_input_window_matches_runtime_default_limit() {
    let limit = resolve_model_limit(None, None, None);

    assert_eq!(super::effective_input_window(&limit), DEFAULT_MODEL_INPUT_LIMIT);
}

#[test]
fn from_models_dev_model_uses_model_adapter_before_provider_adapter() {
    let provider = raw_provider();
    let mut model = raw_model("model-a");
    model.provider = Some(models::ModelProviderInfo { adapter: "model-adapter".to_string() });

    let converted = from_models_dev_model(&provider, &model);

    assert_eq!(converted.api.adapter, "model-adapter");
}

#[test]
fn from_models_dev_model_uses_provider_adapter_when_model_adapter_missing() {
    let provider = raw_provider();
    let model = raw_model("model-a");

    let converted = from_models_dev_model(&provider, &model);

    assert_eq!(converted.api.adapter, "provider-adapter");
}

#[test]
fn from_models_dev_model_uses_default_adapter_when_no_adapter_configured() {
    let mut provider = raw_provider();
    provider.adapter = None;
    let model = raw_model("model-a");

    let converted = from_models_dev_model(&provider, &model);

    assert_eq!(converted.api.adapter, default_adapter());
}

#[test]
fn from_models_dev_model_converts_full_model_metadata() {
    let provider = raw_provider();
    let mut model = raw_model("model-full");
    model.interleaved = Some(models::ModelInterleaved::Field { field: "messages".to_string() });
    model.cost = Some(models::ModelCost {
        input: 1.25,
        output: 2.5,
        cache_read: Some(0.25),
        cache_write: Some(0.5),
        context_over_200k: Some(models::ModelCostOver200k {
            input: 3.0,
            output: 4.0,
            cache_read: Some(0.75),
            cache_write: Some(1.0),
        }),
    });
    model.modalities = Some(models::ModelModalities {
        input: vec!["text".to_string(), "audio".to_string(), "image".to_string()],
        output: vec!["text".to_string(), "video".to_string(), "pdf".to_string()],
    });
    model.status = Some("deprecated".to_string());
    model.headers = Some(HashMap::from([("x-test".to_string(), "yes".to_string())]));
    model.options = HashMap::from([("temperature".to_string(), json!(0.2))]);
    model.variants = Some(HashMap::from([(
        "fast".to_string(),
        HashMap::from([("model".to_string(), json!("model-fast"))]),
    )]));

    let converted = from_models_dev_model(&provider, &model);

    assert_eq!(converted.id, "model-full");
    assert_eq!(converted.provider_id, "example");
    assert_eq!(converted.api.id, "model-full");
    assert_eq!(converted.api.url, "https://api.example.test/v1");
    assert_eq!(converted.name, "Example Model");
    assert_eq!(converted.family.as_deref(), Some("example-family"));
    assert!(converted.capabilities.temperature);
    assert!(converted.capabilities.reasoning);
    assert!(converted.capabilities.attachment);
    assert!(converted.capabilities.toolcall);
    assert!(converted.capabilities.input.text);
    assert!(converted.capabilities.input.audio);
    assert!(converted.capabilities.input.image);
    assert!(!converted.capabilities.input.video);
    assert!(!converted.capabilities.input.pdf);
    assert!(converted.capabilities.output.text);
    assert!(!converted.capabilities.output.audio);
    assert!(!converted.capabilities.output.image);
    assert!(converted.capabilities.output.video);
    assert!(converted.capabilities.output.pdf);
    match converted.capabilities.interleaved {
        InterleavedCapability::Field { field } => assert_eq!(field, "messages"),
        InterleavedCapability::Bool(_) => panic!("expected field interleaved capability"),
    }
    assert_eq!(converted.cost.input, 1.25);
    assert_eq!(converted.cost.output, 2.5);
    assert_eq!(converted.cost.cache.read, 0.25);
    assert_eq!(converted.cost.cache.write, 0.5);
    let over_200k = converted.cost.experimental_over_200k.expect("over 200k cost");
    assert_eq!(over_200k.input, 3.0);
    assert_eq!(over_200k.output, 4.0);
    assert_eq!(over_200k.cache.read, 0.75);
    assert_eq!(over_200k.cache.write, 1.0);
    assert_eq!(converted.limit.context, 128_000);
    assert_eq!(converted.limit.input, Some(64_000));
    assert_eq!(converted.limit.output, 8_192);
    assert_eq!(converted.status, "deprecated");
    assert_eq!(converted.headers.get("x-test").map(String::as_str), Some("yes"));
    assert_eq!(converted.options.get("temperature"), Some(&json!(0.2)));
    assert_eq!(
        converted.variants.get("fast").and_then(|v| v.get("model")),
        Some(&json!("model-fast"))
    );
    assert_eq!(converted.release_date, "2026-01-02");
}

#[test]
fn from_models_dev_model_defaults_missing_optional_metadata() {
    let mut provider = raw_provider();
    provider.api = None;
    let model = raw_model("model-defaults");

    let converted = from_models_dev_model(&provider, &model);

    assert_eq!(converted.api.url, "");
    assert_eq!(converted.cost.input, 0.0);
    assert_eq!(converted.cost.output, 0.0);
    assert_eq!(converted.cost.cache.read, 0.0);
    assert_eq!(converted.cost.cache.write, 0.0);
    assert!(converted.cost.experimental_over_200k.is_none());
    assert!(!converted.capabilities.input.text);
    assert!(!converted.capabilities.input.audio);
    assert!(!converted.capabilities.input.image);
    assert!(!converted.capabilities.input.video);
    assert!(!converted.capabilities.input.pdf);
    assert!(!converted.capabilities.output.text);
    assert!(!converted.capabilities.output.audio);
    assert!(!converted.capabilities.output.image);
    assert!(!converted.capabilities.output.video);
    assert!(!converted.capabilities.output.pdf);
    match converted.capabilities.interleaved {
        InterleavedCapability::Bool(value) => assert!(!value),
        InterleavedCapability::Field { .. } => panic!("expected bool interleaved capability"),
    }
    assert_eq!(converted.status, "active");
    assert!(converted.headers.is_empty());
    assert!(converted.variants.is_empty());
}

#[test]
fn from_models_dev_model_preserves_bool_interleaved_value() {
    let provider = raw_provider();
    let mut model = raw_model("model-interleaved");
    model.interleaved = Some(models::ModelInterleaved::Bool(true));

    let converted = from_models_dev_model(&provider, &model);

    match converted.capabilities.interleaved {
        InterleavedCapability::Bool(value) => assert!(value),
        InterleavedCapability::Field { .. } => panic!("expected bool interleaved capability"),
    }
}

#[test]
fn from_models_dev_model_defaults_missing_cache_costs() {
    let provider = raw_provider();
    let mut model = raw_model("model-cost-defaults");
    model.cost = Some(models::ModelCost {
        input: 1.0,
        output: 2.0,
        cache_read: None,
        cache_write: None,
        context_over_200k: Some(models::ModelCostOver200k {
            input: 3.0,
            output: 4.0,
            cache_read: None,
            cache_write: None,
        }),
    });

    let converted = from_models_dev_model(&provider, &model);

    assert_eq!(converted.cost.cache.read, 0.0);
    assert_eq!(converted.cost.cache.write, 0.0);
    let over_200k = converted.cost.experimental_over_200k.expect("over 200k cost");
    assert_eq!(over_200k.cache.read, 0.0);
    assert_eq!(over_200k.cache.write, 0.0);
}

#[test]
fn from_models_dev_provider_converts_provider_and_models() {
    let mut provider = raw_provider();
    provider.models = HashMap::from([
        ("first".to_string(), raw_model("model-one")),
        ("second".to_string(), raw_model("model-two")),
    ]);

    let converted = from_models_dev_provider(provider);

    assert_eq!(converted.id, "example");
    assert_eq!(converted.name, "Example");
    assert!(matches!(converted.source, ProviderSource::Custom));
    assert_eq!(converted.env, vec!["EXAMPLE_API_KEY"]);
    assert!(converted.key.is_none());
    assert!(converted.options.is_empty());
    assert_eq!(converted.models.len(), 2);
    assert!(converted.models.contains_key("model-one"));
    assert!(converted.models.contains_key("model-two"));
}

#[test]
fn as_string_map_extracts_only_string_values_from_object() {
    let value = json!({
        "text": "kept",
        "number": 1,
        "bool": true,
        "null": null
    });

    let mapped = as_string_map(&value);

    assert_eq!(mapped, HashMap::from([("text".to_string(), "kept".to_string())]));
}

#[test]
fn as_string_map_returns_empty_map_for_non_object() {
    assert!(as_string_map(&json!(["not", "an", "object"])).is_empty());
}

#[test]
fn builtin_model_limit_returns_none_for_any_input() {
    assert_eq!(builtin_model_limit("provider", "model"), None);
}
