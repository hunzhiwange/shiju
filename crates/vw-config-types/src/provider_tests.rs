#[test]
fn provider_api_mode_serializes_as_kebab_case() {
    let serialized = serde_json::to_string(&super::ProviderApiMode::OpenAiResponses).unwrap();
    assert_eq!(serialized, "\"open-ai-responses\"");
}

#[test]
fn provider_api_mode_deserializes_from_kebab_case() {
    let parsed: super::ProviderApiMode =
        serde_json::from_str("\"open-ai-chat-completions\"").unwrap();

    assert_eq!(parsed, super::ProviderApiMode::OpenAiChatCompletions);
}

#[test]
fn provider_api_mode_maps_to_compatible_mode() {
    assert_eq!(
        super::ProviderApiMode::OpenAiChatCompletions.as_compatible_mode(),
        super::CompatibleApiMode::OpenAiChatCompletions
    );
    assert_eq!(
        super::ProviderApiMode::OpenAiResponses.as_compatible_mode(),
        super::CompatibleApiMode::OpenAiResponses
    );
}

#[test]
fn compatible_api_mode_round_trips_with_internal_variant_names() {
    let serialized = serde_json::to_string(&super::CompatibleApiMode::OpenAiResponses).unwrap();
    let parsed: super::CompatibleApiMode = serde_json::from_str(&serialized).unwrap();

    assert_eq!(serialized, "\"OpenAiResponses\"");
    assert_eq!(parsed, super::CompatibleApiMode::OpenAiResponses);
}

#[test]
fn model_provider_config_defaults_missing_fields() {
    let config: super::ModelProviderConfig = serde_json::from_value(serde_json::json!({})).unwrap();

    assert!(config.name.is_none());
    assert!(config.base_url.is_none());
    assert!(config.wire_api.is_none());
    assert!(!config.requires_openai_auth);
}

#[test]
fn model_provider_config_deserializes_explicit_fields() {
    let config: super::ModelProviderConfig = serde_json::from_value(serde_json::json!({
        "name": "openai",
        "base_url": "https://api.example.invalid/v1",
        "wire_api": "responses",
        "requires_openai_auth": true
    }))
    .unwrap();

    assert_eq!(config.name.as_deref(), Some("openai"));
    assert_eq!(config.base_url.as_deref(), Some("https://api.example.invalid/v1"));
    assert_eq!(config.wire_api.as_deref(), Some("responses"));
    assert!(config.requires_openai_auth);
}

#[test]
fn provider_config_defaults_and_deserializes_reasoning_level() {
    let default_config: super::ProviderConfig =
        serde_json::from_value(serde_json::json!({})).unwrap();
    let explicit_config: super::ProviderConfig = serde_json::from_value(serde_json::json!({
        "reasoning_level": "high"
    }))
    .unwrap();

    assert!(default_config.reasoning_level.is_none());
    assert_eq!(explicit_config.reasoning_level.as_deref(), Some("high"));
}
