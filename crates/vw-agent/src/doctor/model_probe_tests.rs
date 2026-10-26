// 覆盖模型探测目标解析及错误类别的诊断映射。

use super::model_probe::{
    ModelProbeOutcome, classify_model_probe_error, doctor_model_targets,
    report_model_probe_targets, resolve_model_probe_targets, run_models,
};
use crate::app::agent::config::Config;

// 诊断断言同时检查严重级别与可操作提示，避免只有返回值正确而用户指引失真。
#[test]
fn classify_model_probe_error_marks_unsupported_as_skipped() {
    let outcome = classify_model_probe_error("Provider does not support live model discovery yet");

    assert_eq!(outcome, ModelProbeOutcome::Skipped);
}

#[test]
fn classify_model_probe_error_marks_auth_access_quota_and_rate_limit_hints() {
    for message in [
        "OpenAI API error (401): unauthorized",
        "API key missing",
        "Forbidden token scope",
        "insufficient balance",
        "insufficient quota",
        "plan does not include requested model",
        "rate limit exceeded",
    ] {
        assert_eq!(
            classify_model_probe_error(message),
            ModelProbeOutcome::AuthOrAccess,
            "{message}"
        );
    }
}

#[test]
fn classify_model_probe_error_falls_back_to_error_for_unknown_text() {
    assert_eq!(classify_model_probe_error("transport closed"), ModelProbeOutcome::Error);
}

#[test]
fn classify_model_probe_error_is_case_insensitive_and_prefers_skip_reason() {
    let outcome =
        classify_model_probe_error("PROVIDER DOES NOT SUPPORT LIVE MODEL DISCOVERY (HTTP 401)");

    assert_eq!(outcome, ModelProbeOutcome::Skipped);
}

#[test]
fn classify_model_probe_error_recognizes_numeric_access_statuses() {
    for message in ["HTTP 403 response", "HTTP 429 response"] {
        assert_eq!(
            classify_model_probe_error(message),
            ModelProbeOutcome::AuthOrAccess,
            "{message}"
        );
    }
}

#[tokio::test]
async fn run_models_accepts_trimmed_provider_override() {
    let config = Config::default();

    run_models(&config, Some(" openrouter "), false).await.expect("provider override should run");
}

#[test]
fn resolve_model_probe_targets_uses_trimmed_provider_override() {
    let targets = resolve_model_probe_targets(
        Some(" openrouter "),
        ["zai".to_string(), "openai".to_string()],
    );

    assert_eq!(targets, vec!["openrouter"]);
}

#[test]
fn resolve_model_probe_targets_sorts_provider_keys() {
    let targets = resolve_model_probe_targets(
        None,
        ["zai".to_string(), "openai".to_string(), "anthropic".to_string()],
    );

    assert_eq!(targets, vec!["anthropic", "openai", "zai"]);
}

#[test]
fn resolve_model_probe_targets_treats_blank_override_as_missing() {
    let targets =
        resolve_model_probe_targets(Some(" \t\n "), ["zai".to_string(), "openai".to_string()]);

    assert_eq!(targets, vec!["openai", "zai"]);
}

#[test]
fn resolve_model_probe_targets_returns_empty_without_override_or_provider_keys() {
    let targets = resolve_model_probe_targets(None, Vec::<String>::new());

    assert!(targets.is_empty());
}

#[test]
fn report_model_probe_targets_errors_without_targets() {
    let error =
        report_model_probe_targets(Vec::new()).expect_err("empty provider list should fail");

    assert_eq!(error.to_string(), "No providers available for model probing");
}

#[test]
fn report_model_probe_targets_accepts_non_empty_targets() {
    report_model_probe_targets(vec!["anthropic".to_string(), "openai".to_string()])
        .expect("non-empty provider list should be reportable");
}

#[tokio::test]
async fn doctor_model_targets_with_override_returns_only_trimmed_provider() {
    let targets = doctor_model_targets(Some(" openrouter ")).await.unwrap();

    assert_eq!(targets, vec!["openrouter"]);
}

#[tokio::test]
async fn doctor_model_targets_without_override_returns_sorted_provider_keys() {
    let targets = doctor_model_targets(None).await.unwrap();
    let mut sorted = targets.clone();
    sorted.sort();

    assert_eq!(targets, sorted);
}
