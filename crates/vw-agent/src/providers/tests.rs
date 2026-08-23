//! 验证供应商工厂、名称校验、别名与错误信息脱敏。

use super::*;

#[test]
// 错误文本进入用户界面前必须同时清理常见令牌头和键值凭据。
fn secret_scrubber_redacts_bearer_and_api_key_values() {
    let input = "Authorization: Bearer sk-secret-value api_key=abcd1234";
    let scrubbed = scrub_secret_patterns(input);
    assert!(!scrubbed.contains("sk-secret-value"));
    assert!(!scrubbed.contains("abcd1234"));
}

#[test]
fn moonshot_aliases_are_case_insensitive() {
    assert!(is_moonshot_alias("KIMI"));
    assert!(is_moonshot_alias("moonshot"));
    assert!(!is_moonshot_alias("openai"));
}

#[test]
fn runtime_options_default_is_secure_and_unset() {
    let options = ProviderRuntimeOptions::default();
    assert!(options.secrets_encrypt);
    assert!(options.auth_profile_override.is_none());
    assert!(options.provider_api_url.is_none());
    assert!(options.vibewindow_dir.is_none());
    assert!(options.reasoning_enabled.is_none());
    assert!(options.reasoning_level.is_none());
    assert!(options.custom_provider_api_mode.is_none());
    assert!(options.max_tokens_override.is_none());
    assert!(options.model_support_vision.is_none());
}

#[test]
// 自定义供应商仅接受合法 URL，别名则走受控的内置名称集合。
fn provider_name_validation_accepts_supported_aliases_and_custom_urls() {
    assert!(create_provider(" openai ", None).is_ok());
    assert!(create_provider("KIMI-K2", None).is_ok());
    assert!(create_provider("alibaba-cn", None).is_ok());
    assert!(create_provider("custom:https://llm.example.test/v1", None).is_ok());
    assert!(create_provider("acme-custom:http://localhost:11434/v1", None).is_ok());
}

#[test]
fn provider_factory_accepts_shiju() {
    assert!(create_provider("shiju", None).is_ok());
}

#[test]
fn provider_name_validation_rejects_unknown_or_bad_custom_urls() {
    assert!(
        create_provider("", None)
            .err()
            .expect("empty provider should error")
            .to_string()
            .contains("must not be empty")
    );
    assert!(
        create_provider("nope", None)
            .err()
            .expect("unknown provider should error")
            .to_string()
            .contains("Unknown provider")
    );
    assert!(
        create_provider("custom:", None)
            .err()
            .expect("missing custom URL should error")
            .to_string()
            .contains("requires a URL")
    );
    assert!(
        create_provider("-custom:https://example.test", None)
            .err()
            .expect("empty custom prefix should error")
            .to_string()
            .contains("prefix must not be empty")
    );
    assert!(
        create_provider("custom:ftp://example.test", None)
            .err()
            .expect("unsupported custom URL scheme should error")
            .to_string()
            .contains("http/https")
    );
    assert!(
        create_provider("custom:http://[invalid", None)
            .err()
            .expect("malformed custom URL should error")
            .to_string()
            .contains("invalid custom provider URL")
    );
}

#[tokio::test]
// 保留 HTTP 状态便于诊断，但响应正文中的秘密不能泄露。
async fn api_error_includes_status_and_redacts_response_body_secrets() {
    let response = axum::http::Response::builder()
        .status(axum::http::StatusCode::UNAUTHORIZED)
        .body(reqwest::Body::from("upstream rejected sk-secret-value"))
        .unwrap()
        .into();

    let error = api_error("Example", response).await.to_string();

    assert!(error.contains("Example API error (401 Unauthorized)"));
    assert!(error.contains("[REDACTED]"));
    assert!(!error.contains("sk-secret-value"));
}

#[test]
fn sanitized_api_errors_redact_multiple_secret_shapes_and_truncate_utf8_safely() {
    let input = r#"sk-abc xoxb-bot ghp_token AIzaKEY AKIAKEY {"access_token":"12345678TOKEN"} token=12345678TOKEN Bearer 1234567890123456TOKEN"#;
    let scrubbed = scrub_secret_patterns(input);
    assert!(!scrubbed.contains("sk-abc"));
    assert!(!scrubbed.contains("xoxb-bot"));
    assert!(!scrubbed.contains("ghp_token"));
    assert!(!scrubbed.contains("12345678TOKEN"));
    assert!(scrubbed.contains("[REDACTED]"));

    let long = "你".repeat(MAX_API_ERROR_CHARS + 5);
    let sanitized = sanitize_api_error(&long);
    assert!(sanitized.ends_with("..."));
    assert!(sanitized.is_char_boundary(sanitized.len()));
}

#[test]
fn url_and_routed_factories_delegate_to_provider_creation() {
    let reliability = crate::app::agent::config::ReliabilityConfig::default();
    let options = ProviderRuntimeOptions { secrets_encrypt: false, ..Default::default() };

    assert!(create_provider_with_url("openai", Some("key"), Some("https://example.test")).is_ok());
    assert!(
        create_provider_with_url_and_options(
            "openai",
            None,
            Some("https://example.test"),
            &options
        )
        .is_ok()
    );
    assert!(create_resilient_provider("openai", None, None, &reliability).is_ok());
    assert!(
        create_resilient_provider_with_options("openai", None, None, &reliability, &options)
            .is_ok()
    );
    assert!(create_routed_provider("openai", None, None, &reliability, &[], "gpt").is_ok());
    assert!(
        create_routed_provider_with_options(
            "openai",
            None,
            None,
            &reliability,
            &[],
            "gpt",
            &options
        )
        .is_ok()
    );
}
