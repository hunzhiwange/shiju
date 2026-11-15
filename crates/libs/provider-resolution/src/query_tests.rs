use super::*;
use crate::*;
use crate::test_support::*;
use serde_json::{Value, json};

#[test]
fn suggest_returns_best_three_matches_above_threshold() {
    let suggestions = suggest(
        "test-providr",
        vec!["other", "test-provider", "test-provider-alt", "test-proxy", "unrelated"],
    );

    assert_eq!(suggestions.len(), 3);
    assert_eq!(suggestions[0], "test-provider");
    assert!(suggestions.contains(&"test-provider-alt".to_string()));
}

#[tokio::test]
async fn get_small_model_prioritizes_github_copilot_models() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "github-copilot-test": {
                "models": {
                    "claude-haiku-4.5": {"status": "active"},
                    "gpt-5-mini": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    let model = get_small_model("github-copilot-test").await.expect("small model should resolve");

    assert_eq!(model.id, "gpt-5-mini");
}

#[tokio::test]
async fn get_model_resolves_unique_api_alias() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    let model =
        get_model("zz-test-provider", "alpha-api").await.expect("unique api alias should resolve");

    assert_eq!(model.id, "alpha");
}

#[tokio::test]
async fn get_model_reports_provider_suggestions() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    let error =
        get_model("zz-test-providr", "alpha").await.expect_err("unknown provider should error");

    assert_eq!(error.provider_id, "zz-test-providr");
    assert_eq!(error.model_id, "alpha");
    assert_eq!(error.suggestions, vec!["zz-test-provider"]);
}

#[tokio::test]
async fn get_model_reports_model_suggestions_for_missing_model() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    let error =
        get_model("zz-test-provider", "alpah").await.expect_err("unknown model should error");

    assert_eq!(error.suggestions.first().map(String::as_str), Some("alpha"));
}

#[tokio::test]
async fn get_model_reports_ambiguous_api_aliases_as_suggestions() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    let error = get_model("zz-test-provider", "shared-api")
        .await
        .expect_err("ambiguous api alias should error");

    assert_eq!(error.suggestions, vec!["alias-a", "alias-b"]);
}

#[tokio::test]
async fn default_model_uses_configured_model_without_loading_state() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(json!("manual-provider/manual-model"), json!({}))).await;

    let parsed = default_model().await.expect("configured default should parse");

    assert_eq!(parsed.provider_id, "manual-provider");
    assert_eq!(parsed.model_id, "manual-model");
}

#[tokio::test]
async fn default_model_falls_back_to_sorted_active_provider_model() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "zz-test-provider": {
                "models": {
                    "alpha": {"status": "active"},
                    "gpt-5-nano": {
                        "status": "active",
                        "limit": {"context": 1024, "output": 256}
                    }
                }
            }
        }),
    ))
    .await;

    let parsed = default_model().await.expect("fallback default should resolve");

    assert_eq!(parsed.provider_id, "zz-test-provider");
    assert_eq!(parsed.model_id, "gpt-5-nano");
}

#[tokio::test]
async fn default_model_errors_when_filter_removes_all_models() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, json!({}))).await;

    let error = default_model().await.expect_err("empty filtered state should error");

    assert_eq!(error, "没有可用的 provider");
}

#[tokio::test]
async fn get_small_model_uses_vibewindow_priority() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "vibewindow-local": {
                "models": {
                    "other-small": {"status": "active"},
                    "gpt-5-nano": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    let model = get_small_model("vibewindow-local").await.expect("small model should resolve");

    assert_eq!(model.id, "gpt-5-nano");
}

#[tokio::test]
async fn get_small_model_returns_none_without_priority_match() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(
        Value::Null,
        json!({
            "zz-test-provider": {
                "models": {
                    "alpha": {"status": "active"}
                }
            }
        }),
    ))
    .await;

    assert!(get_small_model("zz-test-provider").await.is_none());
}

#[tokio::test]
async fn get_small_model_returns_none_for_missing_provider() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    assert!(get_small_model("missing-provider").await.is_none());
}

#[tokio::test]
async fn get_provider_returns_none_for_unknown_provider() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    assert!(get_provider("unknown-provider").await.is_none());
}

#[tokio::test]
async fn init_preloads_model_cache_without_changing_query_results() {
    let env = ProviderTestEnv::new().await;
    env.write_config(config_with_default(Value::Null, active_test_provider_config())).await;

    init();

    assert!(get_provider("zz-test-provider").await.is_some());
}
