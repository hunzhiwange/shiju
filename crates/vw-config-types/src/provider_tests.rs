//! 验证模型提供方 API 模式、兼容映射、默认字段与推理级别配置。
//! 外部 serde 名称与内部兼容枚举分开测试，避免两层表示被混用。

// 提供方 API 模式使用短横线外部名称，与 Rust 变体命名保持解耦。
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

// 兼容模式映射只表达协议能力，不改变原始提供方模式的序列化值。
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

// 缺省字段必须产生可比较的稳定配置，显式字段则需完整往返。
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

// 推理级别属于提供方覆盖项，省略与显式设置需保持不同语义。
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
