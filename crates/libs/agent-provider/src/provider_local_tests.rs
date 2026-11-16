//! 供应商局部配置指纹与候选建议，不安装宿主服务。

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
// 指纹只跟踪供应商快照，避免无关配置变化触发实例重建。
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

#[test]
fn shiju_model_credential_changes_invalidate_provider_fingerprint() {
    let config = config::Config::default();
    let first = auth::Info::Api(auth::ApiInfo { key: "first-model-key".to_string() });
    let next = auth::Info::Api(auth::ApiInfo { key: "next-model-key".to_string() });
    let fingerprint = |info| config_fingerprint_with_shiju_auth(&config, info);
    assert_ne!(fingerprint(None), fingerprint(Some(&first)));
    assert_ne!(fingerprint(Some(&first)), fingerprint(Some(&next)));
    assert_eq!(fingerprint(Some(&first)), fingerprint(Some(&first)));
}
