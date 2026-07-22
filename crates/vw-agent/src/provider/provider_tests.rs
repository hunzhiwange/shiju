use super::*;

#[test]
fn config_fingerprint_tracks_provider_snapshot() {
    let base = config::Config::default();
    let mut changed = base.clone();
    changed.providers.insert("custom".to_string(), serde_json::json!({"models": {}}));

    assert_eq!(config_fingerprint(&base), config_fingerprint(&base));
    assert_ne!(config_fingerprint(&base), config_fingerprint(&changed));
}

#[test]
fn suggestions_prefer_close_candidates() {
    let suggestions = suggest("opnai", vec!["openai", "anthropic"]);
    assert!(suggestions.contains(&"openai".to_string()));
}

#[test]
fn config_fingerprint_ignores_unrelated_configuration() {
    let base = config::Config::default();
    let mut changed = base.clone();
    changed.default_model = Some("openai/gpt-test".to_string());

    assert_eq!(config_fingerprint(&base), config_fingerprint(&changed));
}

#[test]
fn suggestions_drop_distant_candidates() {
    let suggestions = suggest("openai", vec!["anthropic", "gemini", "local"]);

    assert!(suggestions.is_empty());
}

#[test]
fn suggestions_rank_exact_match_first_and_limit_results() {
    let suggestions =
        suggest("openai", vec!["openai-compatible", "openair", "openapi", "openai", "opena"]);

    assert_eq!(suggestions.len(), 3);
    assert_eq!(suggestions.first().map(String::as_str), Some("openai"));
}

#[test]
fn suggestions_handle_empty_inputs() {
    assert!(suggest("", vec![]).is_empty());
    assert!(suggest("provider", vec![]).is_empty());
}
