// 覆盖配置诊断对提供商、渠道及语义错误的报告。

use super::Severity;
use super::config_checks::{
    check_config_semantics, embedding_provider_validation_error, provider_validation_error,
};
use crate::app::agent::config::{
    Config, DelegateAgentConfig, EmbeddingRouteConfig, ModelRouteConfig, TelegramConfig,
};
use std::path::PathBuf;

// 诊断断言同时检查严重级别与可操作提示，避免只有返回值正确而用户指引失真。
// 损坏或缺失的外部状态应转换成诊断项，而不是中断后续检查。
#[test]
fn provider_validation_errors_include_provider_name() {
    let err = provider_validation_error("vibewindow-provider-that-should-not-exist")
        .expect("invalid provider should produce an error");
    let embedding_err = embedding_provider_validation_error("voyage")
        .expect("unsupported embedding provider should produce an error");

    assert!(!err.is_empty());
    assert!(embedding_err.contains("supported values"));
}

#[test]
fn alibaba_embedding_providers_are_valid() {
    assert!(embedding_provider_validation_error("alibaba").is_none());
    assert!(embedding_provider_validation_error("alibaba-cn").is_none());
}

#[test]
fn custom_embedding_provider_validation_checks_url_shape() {
    assert!(embedding_provider_validation_error("custom:https://embeddings.example.com").is_none());
    assert!(embedding_provider_validation_error("custom:http://localhost:8080").is_none());
    assert!(embedding_provider_validation_error(" NONE ").is_none());
    assert!(embedding_provider_validation_error("OPENAI").is_none());

    let empty = embedding_provider_validation_error("custom:").unwrap();
    assert!(empty.contains("requires a non-empty URL"));

    let scheme = embedding_provider_validation_error("custom:file:///tmp/embed").unwrap();
    assert!(scheme.contains("must use http/https"));

    let invalid = embedding_provider_validation_error("custom:not a url").unwrap();
    assert!(invalid.contains("invalid custom provider URL"));
}

#[test]
fn config_semantics_reports_valid_core_settings() {
    let mut config = Config::default();
    config.config_path = PathBuf::from("Cargo.toml");
    config.default_provider = Some("openai".into());
    config.api_key = Some("configured".into());
    config.default_model = Some("gpt-4.1".into());
    config.default_temperature = 2.0;
    config.gateway.port = 42617;
    config.channels_config.telegram = Some(TelegramConfig {
        bot_token: "token".into(),
        allowed_users: vec!["user".into()],
        stream_mode: Default::default(),
        draft_update_interval_ms: 1000,
        interrupt_on_new_message: false,
        mention_only: false,
        group_reply: None,
        base_url: None,
    });
    let mut items = Vec::new();

    check_config_semantics(&config, &mut items);

    assert!(items.iter().any(|item| {
        item.severity == Severity::Ok && item.message.starts_with("config file: ")
    }));
    assert!(items.iter().any(|item| item.message == "provider \"openai\" is valid"));
    assert!(items.iter().any(|item| item.message == "API key configured"));
    assert!(items.iter().any(|item| item.message == "default model: gpt-4.1"));
    assert!(items.iter().any(|item| item.message == "temperature 2.0 (valid range 0.0–2.0)"));
    assert!(items.iter().any(|item| item.message == "gateway port: 42617"));
    assert!(items.iter().any(|item| item.message == "at least one channel configured"));
}

#[test]
fn config_semantics_reports_missing_core_settings() {
    let mut config = Config::default();
    config.config_path = PathBuf::from("target/vibewindow-doctor-missing-config.json");
    config.default_provider = None;
    config.api_key = None;
    config.default_model = None;
    config.default_temperature = -0.1;
    config.gateway.port = 0;
    let mut items = Vec::new();

    check_config_semantics(&config, &mut items);

    assert!(items.iter().any(|item| {
        item.severity == Severity::Error && item.message.starts_with("config file not found: ")
    }));
    assert!(items.iter().any(|item| item.message == "no default_provider configured"));
    assert!(items.iter().any(|item| item.message.contains("no api_key set")));
    assert!(items.iter().any(|item| item.message == "no default_model configured"));
    assert!(items.iter().any(|item| {
        item.severity == Severity::Error
            && item.message == "temperature -0.1 is out of range (expected 0.0–2.0)"
    }));
    assert!(items.iter().any(|item| item.message == "gateway port is 0 (invalid)"));
}

#[test]
fn ollama_provider_does_not_warn_about_missing_api_key() {
    let mut config = Config::default();
    config.default_provider = Some("ollama".into());
    config.api_key = None;
    let mut items = Vec::new();

    check_config_semantics(&config, &mut items);

    assert!(!items.iter().any(|item| item.message.contains("no api_key set")));
}

#[test]
fn config_semantics_reports_model_route_hint_provider_and_model_issues() {
    let mut config = Config::default();
    config.model_routes = vec![ModelRouteConfig {
        hint: String::new(),
        provider: "provider-that-does-not-exist".into(),
        model: String::new(),
        max_tokens: None,
        api_key: None,
    }];
    let mut items = Vec::new();

    check_config_semantics(&config, &mut items);

    assert!(items.iter().any(
        |item| item.severity == Severity::Warn && item.message == "model route with empty hint"
    ));
    assert!(
        items.iter().any(|item| item.severity == Severity::Warn
            && item.message.contains("uses invalid provider"))
    );
    assert!(
        items
            .iter()
            .any(|item| item.severity == Severity::Warn && item.message.contains("has empty model"))
    );
}

#[test]
fn config_semantics_reports_invalid_default_provider() {
    let mut config = Config::default();
    config.default_provider = Some("provider-that-does-not-exist".into());
    let mut items = Vec::new();

    check_config_semantics(&config, &mut items);

    assert!(items.iter().any(|item| {
        item.severity == Severity::Error
            && item.message.contains("default provider \"provider-that-does-not-exist\" is invalid")
    }));
}

#[test]
fn config_semantics_reports_embedding_route_dimensions_and_missing_hint() {
    let mut config = Config::default();
    config.embedding_routes = vec![EmbeddingRouteConfig {
        hint: "semantic".into(),
        provider: "openai".into(),
        model: "text-embedding-3-small".into(),
        dimensions: Some(0),
        api_key: None,
    }];
    config.memory.embedding_model = "hint:missing".into();
    let mut items = Vec::new();

    check_config_semantics(&config, &mut items);

    assert!(items.iter().any(|item| item.message.contains("invalid dimensions=0")));
    assert!(
        items
            .iter()
            .any(|item| item.message.contains("memory.embedding_model uses hint \"missing\""))
    );
}

#[test]
fn config_semantics_reports_embedding_route_hint_provider_and_model_issues() {
    let mut config = Config::default();
    config.embedding_routes = vec![EmbeddingRouteConfig {
        hint: "   ".into(),
        provider: "voyage".into(),
        model: "   ".into(),
        dimensions: None,
        api_key: None,
    }];
    let mut items = Vec::new();

    check_config_semantics(&config, &mut items);

    assert!(items.iter().any(|item| item.message == "embedding route with empty hint"));
    assert!(items.iter().any(|item| {
        item.message.contains("embedding route \"   \" uses invalid provider \"voyage\"")
    }));
    assert!(items.iter().any(|item| item.message == "embedding route \"   \" has empty model"));
}

#[test]
fn config_semantics_accepts_matching_embedding_hint() {
    let mut config = Config::default();
    config.embedding_routes = vec![EmbeddingRouteConfig {
        hint: "semantic".into(),
        provider: "openai".into(),
        model: "text-embedding-3-small".into(),
        dimensions: Some(1536),
        api_key: None,
    }];
    config.memory.embedding_model = "hint:semantic".into();
    let mut items = Vec::new();

    check_config_semantics(&config, &mut items);

    assert!(
        !items
            .iter()
            .any(|item| item.message.contains("no matching [[embedding_routes]] entry exists"))
    );
}

#[test]
fn config_semantics_ignores_blank_embedding_hint_reference() {
    let mut config = Config::default();
    config.memory.embedding_model = "hint:   ".into();
    let mut items = Vec::new();

    check_config_semantics(&config, &mut items);

    assert!(
        !items
            .iter()
            .any(|item| item.message.contains("no matching [[embedding_routes]] entry exists"))
    );
}

#[test]
fn config_semantics_skips_disabled_delegate_agents() {
    let mut config = Config::default();
    config.agents.insert(
        "disabled".into(),
        DelegateAgentConfig {
            enabled: false,
            provider: "provider-that-does-not-exist".into(),
            model: "model".into(),
            ..Default::default()
        },
    );
    config.agents.insert(
        "enabled".into(),
        DelegateAgentConfig {
            enabled: true,
            provider: "provider-that-does-not-exist".into(),
            model: "model".into(),
            ..Default::default()
        },
    );
    let mut items = Vec::new();

    check_config_semantics(&config, &mut items);

    assert!(!items.iter().any(|item| item.message.contains("agent \"disabled\"")));
    assert!(items.iter().any(|item| item.message.contains("agent \"enabled\"")));
}
