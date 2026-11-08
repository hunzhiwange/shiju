use super::codex_template::codex_custom_template;
use crate::providers::codex_catalog::form;
use std::collections::BTreeMap;
use toml_edit::{DocumentMut, Item, value};
use vw_api_types::app_providers::{
    basic::ProviderBasic, codex_catalog::CodexFormWrite, create::ProviderSecretWrite,
};

#[test]
fn catalog_codex_template_returns_independent_empty_auth() {
    let mut edited = codex_custom_template();
    edited.auth.insert("OPENAI_API_KEY".into(), "draft-key".into());
    edited.config.clear();

    let fresh = codex_custom_template();
    assert_eq!(fresh.auth, BTreeMap::from([("OPENAI_API_KEY".into(), String::new())]));
    assert!(!fresh.config.is_empty());
}

#[test]
fn catalog_codex_template_uses_responses_defaults_without_forcing_goal_mode() {
    let draft: DocumentMut = codex_custom_template().config.parse().unwrap();
    assert_eq!(draft.get("model").and_then(Item::as_str), Some("gpt-5.6-sol"));
    assert_eq!(draft["model_reasoning_effort"].as_str(), Some("high"));
    assert_eq!(draft["disable_response_storage"].as_bool(), Some(true));
    assert_eq!(draft["model_provider"].as_str(), Some("custom"));

    let provider = &draft["model_providers"]["custom"];
    assert_eq!(provider["name"].as_str(), Some("custom"));
    assert_eq!(provider["wire_api"].as_str(), Some("responses"));
    assert_eq!(provider["requires_openai_auth"].as_bool(), Some(true));
    assert!(provider.get("base_url").is_none());
    assert!(draft.get("features").and_then(|features| features.get("goals")).is_none());
}

#[test]
fn catalog_codex_template_requires_endpoint_and_key_to_prepare_provider() {
    let mut draft: DocumentMut = codex_custom_template().config.parse().unwrap();
    let mut request = CodexFormWrite {
        target_host_id: "local".into(),
        fields: ProviderBasic {
            name: "Example".into(),
            category: None,
            notes: None,
            website_url: None,
        },
        toml: draft.to_string(),
        api_key: ProviderSecretWrite::Replace { value: "fixture-key".into() },
        models: Some(Vec::new()),
        apply: false,
    };
    assert!(form::prepare("example", &request, None).is_err());

    let provider = draft
        .get_mut("model_providers")
        .and_then(|providers| providers.get_mut("custom"))
        .expect("template must contain the custom provider");
    provider["base_url"] = value("https://example.com/v1");
    request.toml = draft.to_string();
    request.api_key = ProviderSecretWrite::Keep;
    assert!(form::prepare("example", &request, None).is_err());

    request.api_key = ProviderSecretWrite::Replace { value: "fixture-key".into() };
    let profile = form::prepare("example", &request, None).unwrap();
    assert_eq!(profile.api_key.as_deref(), Some("fixture-key"));
    let config: DocumentMut = profile.config.parse().unwrap();
    assert_eq!(config["model"].as_str(), Some("gpt-5.6-sol"));
    assert_eq!(config["model_providers"]["custom"]["wire_api"].as_str(), Some("responses"));
}
