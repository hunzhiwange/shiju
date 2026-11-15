use super::codex_template::codex_custom_template;
use std::collections::BTreeMap;
use toml_edit::{DocumentMut, Item};

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
