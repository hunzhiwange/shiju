// 验证集成配置的别名匹配、修订计算、凭据更新与面板载荷生成。

// 凭据只用于判断配置状态，生成面板载荷时不得回传原始密钥。
use super::*;
use crate::app::agent::integrations::IntegrationStatus;
use std::collections::BTreeMap;

// 查找 helper 让载荷断言聚焦字段契约，缺项会立即使测试失败。
fn field<'a>(entry: &'a IntegrationSettingsEntry, key: &str) -> &'a IntegrationCredentialsField {
    entry.fields.iter().find(|field| field.key == key).unwrap()
}

fn entry<'a>(payload: &'a IntegrationSettingsPayload, id: &str) -> &'a IntegrationSettingsEntry {
    payload.integrations.iter().find(|entry| entry.id == id).unwrap()
}

#[test]
fn provider_alias_matches_is_case_insensitive() {
    let spec = DashboardAiIntegrationSpec {
        id: "openai",
        integration_name: "OpenAI",
        provider_id: "openai",
        requires_api_key: true,
        supports_api_url: false,
        model_options: &[],
    };

    assert!(provider_alias_matches(&spec, "OpenAI"));
    assert!(!provider_alias_matches(&spec, "anthropic"));
}

#[test]
fn provider_alias_matches_supports_dashboard_aliases() {
    let google = find_dashboard_spec("google").unwrap();
    assert!(provider_alias_matches(google, " google-gemini "));
    assert!(provider_alias_matches(google, "Gemini"));
    assert!(!provider_alias_matches(google, "openai"));

    let xai = find_dashboard_spec("xai").unwrap();
    assert!(provider_alias_matches(xai, "GROK"));
    assert!(!provider_alias_matches(xai, "x-ai"));

    let vercel = find_dashboard_spec("vercel").unwrap();
    assert!(provider_alias_matches(vercel, "vercel-ai"));

    let cloudflare = find_dashboard_spec("cloudflare").unwrap();
    assert!(provider_alias_matches(cloudflare, "cloudflare-ai"));
}

#[test]
fn find_dashboard_spec_matches_id_case_insensitively() {
    let spec = find_dashboard_spec("OpenAI").unwrap();

    assert_eq!(spec.id, "openai");
    assert_eq!(spec.integration_name, "OpenAI");
    assert!(find_dashboard_spec("missing").is_none());
}

#[test]
fn has_non_empty_rejects_blank_values() {
    assert!(!has_non_empty(None));
    assert!(!has_non_empty(Some("  ")));
    assert!(has_non_empty(Some("token")));
}

#[test]
fn config_revision_changes_when_config_changes() {
    let mut config = Config::default();
    let original = config_revision(&config);

    config.default_provider = Some("openai".to_string());
    let changed = config_revision(&config);

    assert_eq!(original.len(), 64);
    assert_eq!(changed.len(), 64);
    assert_ne!(original, changed);
}

#[test]
// 面板载荷只报告是否已配置，不得把实际凭据复制到响应字段。
fn build_payload_marks_active_keyed_provider_without_exposing_secret() {
    let mut config = Config::default();
    config.default_provider = Some("openai".to_string());
    config.default_model = Some("gpt-5.2".to_string());
    config.api_key = Some("sk-test".to_string());
    config.api_url = Some("https://stale.example".to_string());

    let payload = build_integration_settings_payload(&config);
    let openai = entry(&payload, "openai");
    let anthropic = entry(&payload, "anthropic");

    assert_eq!(payload.active_default_provider_integration_id.as_deref(), Some("openai"));
    assert_eq!(payload.integrations.len(), DASHBOARD_AI_INTEGRATION_SPECS.len());
    assert_eq!(openai.status, IntegrationStatus::Active);
    assert!(openai.configured);
    assert!(openai.activates_default_provider);

    let api_key = field(openai, "api_key");
    assert!(api_key.required);
    assert!(api_key.has_value);
    assert_eq!(api_key.input_type, "secret");
    assert_eq!(api_key.current_value, None);
    assert_eq!(api_key.masked_value.as_deref(), Some("••••••••"));

    let model = field(openai, "default_model");
    assert_eq!(model.input_type, "select");
    assert!(model.has_value);
    assert_eq!(model.current_value.as_deref(), Some("gpt-5.2"));
    assert!(model.options.iter().any(|option| option == "gpt-5.2"));

    assert!(!anthropic.configured);
    assert!(!field(anthropic, "default_model").has_value);
    assert_eq!(field(anthropic, "default_model").current_value, None);
}

#[test]
fn build_payload_marks_unkeyed_provider_as_unconfigured() {
    let mut config = Config::default();
    config.default_provider = Some("openrouter".to_string());
    config.default_model = Some("openai/gpt-5.2".to_string());
    config.api_key = None;

    let payload = build_integration_settings_payload(&config);
    let openrouter = entry(&payload, "openrouter");

    assert_eq!(payload.active_default_provider_integration_id.as_deref(), Some("openrouter"));
    assert_eq!(openrouter.status, IntegrationStatus::Available);
    assert!(!openrouter.configured);
    assert!(!field(openrouter, "api_key").has_value);
    assert!(field(openrouter, "default_model").has_value);
}

#[test]
fn build_payload_uses_provider_aliases_for_active_id() {
    let mut config = Config::default();
    config.default_provider = Some(" gemini ".to_string());
    config.default_model = Some("google/gemini-3.1-pro".to_string());
    config.api_key = Some("key".to_string());

    let payload = build_integration_settings_payload(&config);
    let google = entry(&payload, "google");

    assert_eq!(payload.active_default_provider_integration_id.as_deref(), Some("google"));
    assert_eq!(google.status, IntegrationStatus::Active);
    assert!(google.configured);
    assert_eq!(
        field(google, "default_model").current_value.as_deref(),
        Some("google/gemini-3.1-pro")
    );
}

#[test]
fn build_payload_includes_api_url_field_only_for_supported_provider() {
    let mut config = Config::default();
    config.default_provider = Some("ollama".to_string());
    config.default_model = Some("llama3.2".to_string());
    config.api_url = Some("http://localhost:11434".to_string());

    let payload = build_integration_settings_payload(&config);
    let ollama = entry(&payload, "ollama");
    let openai = entry(&payload, "openai");

    assert!(ollama.configured);
    assert!(!field(ollama, "api_key").required);

    let api_url = field(ollama, "api_url");
    assert_eq!(api_url.label, "Base URL");
    assert_eq!(api_url.input_type, "text");
    assert!(api_url.has_value);
    assert_eq!(api_url.current_value.as_deref(), Some("http://localhost:11434"));
    assert!(openai.fields.iter().all(|field| field.key != "api_url"));
}

#[test]
fn build_payload_returns_no_active_id_without_default_provider() {
    let mut config = Config::default();
    config.default_provider = None;

    let payload = build_integration_settings_payload(&config);

    assert_eq!(payload.active_default_provider_integration_id, None);
    assert!(payload.integrations.iter().all(|entry| !entry.configured));
}

#[test]
fn apply_update_sets_provider_and_default_model_on_first_activation() {
    let mut config = Config::default();
    config.default_provider = Some("anthropic".to_string());
    config.default_model = Some("claude-sonnet-4-6".to_string());
    config.api_url = Some("https://stale.example".to_string());

    let mut fields = BTreeMap::new();
    fields.insert("api_key".to_string(), "  sk-openai  ".to_string());

    let updated = apply_integration_credentials_update(&config, "openai", &fields).unwrap();

    assert_eq!(updated.default_provider.as_deref(), Some("openai"));
    assert_eq!(updated.default_model.as_deref(), Some("gpt-5.2"));
    assert_eq!(updated.api_key.as_deref(), Some("sk-openai"));
    assert_eq!(updated.api_url, None);
}

#[test]
fn apply_update_preserves_active_model_when_model_is_omitted() {
    let mut config = Config::default();
    config.default_provider = Some("openai".to_string());
    config.default_model = Some("gpt-4o".to_string());

    let fields = BTreeMap::new();
    let updated = apply_integration_credentials_update(&config, "openai", &fields).unwrap();

    assert_eq!(updated.default_provider.as_deref(), Some("openai"));
    assert_eq!(updated.default_model.as_deref(), Some("gpt-4o"));
}

#[test]
fn apply_update_clears_blank_fields() {
    let mut config = Config::default();
    config.default_provider = Some("ollama".to_string());
    config.default_model = Some("llama3.2".to_string());
    config.api_key = Some("key".to_string());
    config.api_url = Some("http://localhost:11434".to_string());

    let mut fields = BTreeMap::new();
    fields.insert("api_key".to_string(), " ".to_string());
    fields.insert("default_model".to_string(), " ".to_string());
    fields.insert("api_url".to_string(), " ".to_string());

    let updated = apply_integration_credentials_update(&config, "ollama", &fields).unwrap();

    assert_eq!(updated.default_provider.as_deref(), Some("ollama"));
    assert_eq!(updated.api_key, None);
    assert_eq!(updated.default_model, None);
    assert_eq!(updated.api_url, None);
}

#[test]
fn apply_update_accepts_supported_api_url() {
    let config = Config::default();
    let mut fields = BTreeMap::new();
    fields.insert("default_model".to_string(), "qwen2.5-coder:7b".to_string());
    fields.insert("api_url".to_string(), " http://127.0.0.1:11434 ".to_string());

    let updated = apply_integration_credentials_update(&config, "ollama", &fields).unwrap();

    assert_eq!(updated.default_provider.as_deref(), Some("ollama"));
    assert_eq!(updated.default_model.as_deref(), Some("qwen2.5-coder:7b"));
    assert_eq!(updated.api_url.as_deref(), Some("http://127.0.0.1:11434"));
}

#[test]
fn apply_update_clears_stale_api_url_when_supported_provider_first_activates_without_url() {
    let mut config = Config::default();
    config.default_provider = Some("openai".to_string());
    config.api_url = Some("https://stale.example".to_string());

    let updated =
        apply_integration_credentials_update(&config, "ollama", &BTreeMap::new()).unwrap();

    assert_eq!(updated.default_provider.as_deref(), Some("ollama"));
    assert_eq!(updated.default_model.as_deref(), Some("llama3.2"));
    assert_eq!(updated.api_url, None);
}

#[test]
fn apply_update_rejects_unknown_integration_id() {
    let err = apply_integration_credentials_update(&Config::default(), "missing", &BTreeMap::new())
        .unwrap_err();

    assert_eq!(err, "Unknown integration id: missing");
}

#[test]
fn apply_update_rejects_api_url_for_unsupported_integration() {
    let mut fields = BTreeMap::new();
    fields.insert("api_url".to_string(), "http://localhost:11434".to_string());

    let err =
        apply_integration_credentials_update(&Config::default(), "openai", &fields).unwrap_err();

    assert_eq!(err, "Integration 'OpenAI' does not support api_url");
}

#[test]
fn apply_update_rejects_unknown_field() {
    let mut fields = BTreeMap::new();
    fields.insert("token".to_string(), "secret".to_string());

    let err =
        apply_integration_credentials_update(&Config::default(), "openai", &fields).unwrap_err();

    assert_eq!(err, "Unsupported field 'token' for integration 'openai'");
}

#[test]
fn apply_update_returns_validation_error_for_invalid_config() {
    let mut config = Config::default();
    config.gateway.host.clear();

    let err =
        apply_integration_credentials_update(&config, "openai", &BTreeMap::new()).unwrap_err();

    assert!(err.starts_with("Invalid integration config update: gateway.host must not be empty"));
}
