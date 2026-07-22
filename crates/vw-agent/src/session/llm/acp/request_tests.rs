use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use serde_json::json;
use tokio::sync::watch;

use crate::app::agent::provider::provider;
use crate::app::agent::session::{llm::StreamEvent, message};
use crate::app::agent::{auth, config};

use super::{
    copilot_auth_token, do_stream_request_acp, is_copilot_acp_agent, is_github_copilot_provider,
    is_usable_copilot_token, parse_close_client_after_request, with_copilot_auth_environment,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    _lock: MutexGuard<'static, ()>,
    key: &'static str,
    saved: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let lock = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let saved = std::env::var(key).ok();
        unsafe { std::env::set_var(key, value) };
        Self { _lock: lock, key, saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.saved {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn test_model(provider_id: &str) -> provider::Model {
    serde_json::from_value(json!({
        "id": "gpt-5-mini",
        "providerID": provider_id,
        "api": {
            "id": "gpt-5-mini",
            "url": "http://localhost",
            "adapter": "acp"
        },
        "name": "gpt-5-mini",
        "family": null,
        "capabilities": {
            "temperature": true,
            "reasoning": true,
            "attachment": false,
            "toolcall": true,
            "input": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "output": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "interleaved": false
        },
        "cost": {
            "input": 0.0,
            "output": 0.0,
            "cache": {
                "read": 0.0,
                "write": 0.0
            },
            "experimental_over_200k": null
        },
        "limit": {
            "context": 8192,
            "input": null,
            "output": 4096
        },
        "status": "active",
        "options": {},
        "headers": {},
        "release_date": "2026-01-01",
        "variants": {}
    }))
    .expect("test model should deserialize")
}

fn test_provider(key: Option<&str>) -> provider::Info {
    provider::Info {
        id: "github-copilot".to_string(),
        name: "GitHub Copilot".to_string(),
        source: provider::ProviderSource::Api,
        env: Vec::new(),
        key: key.map(ToString::to_string),
        options: HashMap::new(),
        models: HashMap::new(),
    }
}

fn copilot_config() -> config::schema::AcpAgentConfig {
    config::schema::AcpAgentConfig {
        command: "copilot".to_string(),
        args: vec!["--acp".to_string(), "--stdio".to_string()],
        env: HashMap::new(),
    }
}

#[test]
fn copilot_provider_detection_is_case_and_space_tolerant() {
    assert!(is_github_copilot_provider(" GitHub-Copilot "));
    assert!(is_github_copilot_provider("github-copilot-chat"));
    assert!(!is_github_copilot_provider("openai"));
}

#[test]
fn copilot_agent_detection_checks_name_command_and_args() {
    let plain = config::schema::AcpAgentConfig {
        command: "node".to_string(),
        args: vec!["agent.js".to_string()],
        env: HashMap::new(),
    };
    assert!(!is_copilot_acp_agent("custom", &plain));
    assert!(is_copilot_acp_agent(" GitHub Copilot ", &plain));

    let mut by_command = plain.clone();
    by_command.command = "copilot-agent".to_string();
    assert!(is_copilot_acp_agent("custom", &by_command));

    let mut by_arg = plain;
    by_arg.args.push("--use-copilot".to_string());
    assert!(is_copilot_acp_agent("custom", &by_arg));
}

#[test]
fn usable_copilot_token_rejects_empty_and_oauth_dummy_values() {
    assert!(is_usable_copilot_token("token"));
    assert!(!is_usable_copilot_token(""));
    assert!(!is_usable_copilot_token(auth::OAUTH_DUMMY_KEY));
}

#[test]
fn copilot_auth_environment_uses_provider_key() {
    let cfg = with_copilot_auth_environment(
        &test_model("github-copilot"),
        &test_provider(Some(" provider-token ")),
        None,
        "copilot",
        copilot_config(),
    );

    assert_eq!(cfg.env.get("GH_COPILOT_TOKEN").map(String::as_str), Some("provider-token"));
    assert_eq!(cfg.env.get("GITHUB_COPILOT_TOKEN").map(String::as_str), Some("provider-token"));
}

#[test]
fn copilot_auth_environment_preserves_explicit_agent_env() {
    let mut cfg = copilot_config();
    cfg.env.insert("GH_COPILOT_TOKEN".to_string(), "explicit-token".to_string());

    let cfg = with_copilot_auth_environment(
        &test_model("github-copilot"),
        &test_provider(Some("provider-token")),
        None,
        "copilot",
        cfg,
    );

    assert_eq!(cfg.env.get("GH_COPILOT_TOKEN").map(String::as_str), Some("explicit-token"));
    assert_eq!(cfg.env.get("GITHUB_COPILOT_TOKEN").map(String::as_str), Some("provider-token"));
}

#[test]
fn copilot_auth_environment_preserves_both_explicit_token_variables() {
    let mut cfg = copilot_config();
    cfg.env.insert("GH_COPILOT_TOKEN".to_string(), "explicit-gh-token".to_string());
    cfg.env.insert("GITHUB_COPILOT_TOKEN".to_string(), "explicit-github-token".to_string());

    let cfg = with_copilot_auth_environment(
        &test_model("github-copilot"),
        &test_provider(Some("provider-token")),
        None,
        "copilot",
        cfg,
    );

    assert_eq!(cfg.env.get("GH_COPILOT_TOKEN").map(String::as_str), Some("explicit-gh-token"));
    assert_eq!(
        cfg.env.get("GITHUB_COPILOT_TOKEN").map(String::as_str),
        Some("explicit-github-token")
    );
}

#[test]
fn copilot_auth_token_uses_provider_env_before_auth_fallback() {
    let _guard = EnvGuard::set("VW_AGENT_COPILOT_TEST_TOKEN", " env-token ");
    let mut provider = test_provider(None);
    provider.env = vec!["VW_AGENT_COPILOT_TEST_TOKEN".to_string()];

    let token = copilot_auth_token(
        &provider,
        Some(&auth::Info::Api(auth::ApiInfo { key: "auth-token".to_string() })),
    );

    assert_eq!(token.as_deref(), Some("env-token"));
}

#[test]
fn copilot_auth_token_prefers_provider_key_over_env_and_auth() {
    let _guard = EnvGuard::set("VW_AGENT_COPILOT_PROVIDER_KEY_TEST_TOKEN", "env-token");
    let mut provider = test_provider(Some(" provider-token "));
    provider.env = vec!["VW_AGENT_COPILOT_PROVIDER_KEY_TEST_TOKEN".to_string()];

    let token = copilot_auth_token(
        &provider,
        Some(&auth::Info::Api(auth::ApiInfo { key: "auth-token".to_string() })),
    );

    assert_eq!(token.as_deref(), Some("provider-token"));
}

#[test]
fn copilot_auth_token_falls_back_from_dummy_provider_key_to_env() {
    let _guard = EnvGuard::set("VW_AGENT_COPILOT_DUMMY_KEY_TEST_TOKEN", " env-token ");
    let mut provider = test_provider(Some(auth::OAUTH_DUMMY_KEY));
    provider.env = vec!["VW_AGENT_COPILOT_DUMMY_KEY_TEST_TOKEN".to_string()];

    let token = copilot_auth_token(
        &provider,
        Some(&auth::Info::Api(auth::ApiInfo { key: "auth-token".to_string() })),
    );

    assert_eq!(token.as_deref(), Some("env-token"));
}

#[test]
fn copilot_auth_token_falls_back_from_blank_env_to_api_auth() {
    let _guard = EnvGuard::set("VW_AGENT_COPILOT_BLANK_ENV_TEST_TOKEN", "   ");
    let mut provider = test_provider(None);
    provider.env = vec!["VW_AGENT_COPILOT_BLANK_ENV_TEST_TOKEN".to_string()];

    let token = copilot_auth_token(
        &provider,
        Some(&auth::Info::Api(auth::ApiInfo { key: " auth-token ".to_string() })),
    );

    assert_eq!(token.as_deref(), Some("auth-token"));
}

#[test]
fn copilot_auth_token_skips_dummy_env_before_api_auth() {
    let _guard = EnvGuard::set("VW_AGENT_COPILOT_DUMMY_ENV_TEST_TOKEN", auth::OAUTH_DUMMY_KEY);
    let mut provider = test_provider(None);
    provider.env = vec!["VW_AGENT_COPILOT_DUMMY_ENV_TEST_TOKEN".to_string()];

    let token = copilot_auth_token(
        &provider,
        Some(&auth::Info::Api(auth::ApiInfo { key: " auth-token ".to_string() })),
    );

    assert_eq!(token.as_deref(), Some("auth-token"));
}

#[test]
fn copilot_auth_token_rejects_blank_and_non_api_auth_fallbacks() {
    let provider = test_provider(None);
    let blank_api = auth::Info::Api(auth::ApiInfo { key: "   ".to_string() });
    let oauth = auth::Info::Oauth(auth::OauthInfo {
        refresh: "refresh-token".to_string(),
        access: "access-token".to_string(),
        expires: 0,
        account_id: None,
        enterprise_url: None,
    });
    let well_known = auth::Info::Wellknown(auth::WellKnownInfo {
        key: "well-known-key".to_string(),
        token: "well-known-token".to_string(),
    });

    assert_eq!(copilot_auth_token(&provider, Some(&blank_api)), None);
    assert_eq!(copilot_auth_token(&provider, Some(&oauth)), None);
    assert_eq!(copilot_auth_token(&provider, Some(&well_known)), None);
}

#[test]
fn copilot_auth_environment_uses_api_auth_fallback() {
    let cfg = with_copilot_auth_environment(
        &test_model("github-copilot"),
        &test_provider(None),
        Some(&auth::Info::Api(auth::ApiInfo { key: " auth-token ".to_string() })),
        "github-copilot",
        copilot_config(),
    );

    assert_eq!(cfg.env.get("GH_COPILOT_TOKEN").map(String::as_str), Some("auth-token"));
}

#[test]
fn copilot_auth_environment_ignores_oauth_dummy_key() {
    let cfg = with_copilot_auth_environment(
        &test_model("github-copilot"),
        &test_provider(Some(auth::OAUTH_DUMMY_KEY)),
        None,
        "copilot",
        copilot_config(),
    );

    assert!(cfg.env.is_empty());
}

#[test]
fn copilot_auth_environment_ignores_non_copilot_agent() {
    let cfg = with_copilot_auth_environment(
        &test_model("github-copilot"),
        &test_provider(Some("provider-token")),
        None,
        "custom",
        config::schema::AcpAgentConfig {
            command: "agent".to_string(),
            args: vec!["--stdio".to_string()],
            env: HashMap::new(),
        },
    );

    assert!(cfg.env.is_empty());
}

#[test]
fn copilot_auth_environment_ignores_non_copilot_provider() {
    let cfg = with_copilot_auth_environment(
        &test_model("openai"),
        &test_provider(Some("provider-token")),
        None,
        "copilot",
        copilot_config(),
    );

    assert!(cfg.env.is_empty());
}

#[test]
fn close_client_after_request_option_defaults_to_false() {
    assert!(!parse_close_client_after_request(&json!({})).expect("missing option should parse"));
}

#[test]
fn close_client_after_request_option_treats_non_object_as_missing() {
    assert!(!parse_close_client_after_request(&json!(null)).expect("null should use the default"));
    assert!(!parse_close_client_after_request(&json!([])).expect("array should use the default"));
}

#[test]
fn close_client_after_request_option_accepts_boolean_only() {
    assert!(
        parse_close_client_after_request(&json!({"acp_close_client_after_request": true}))
            .expect("boolean option should parse")
    );
    assert!(
        parse_close_client_after_request(&json!({"acp_close_client_after_request": "true"}))
            .is_err()
    );
}

#[test]
fn close_client_after_request_option_accepts_explicit_false() {
    assert!(
        !parse_close_client_after_request(&json!({"acp_close_client_after_request": false}))
            .expect("false should remain disabled")
    );
}

#[test]
fn close_client_after_request_option_reports_non_boolean_error() {
    let error = parse_close_client_after_request(&json!({
        "acp_close_client_after_request": 1
    }))
    .expect_err("numeric option should be rejected");

    match error {
        crate::app::agent::session::llm::Error::Api(message::AssistantError::Unknown {
            message,
        }) => assert_eq!(
            message,
            "acp option error: acp_close_client_after_request must be a boolean"
        ),
        other => panic!("expected ACP option error, got {other:?}"),
    }
}

#[test]
fn close_client_after_request_option_rejects_explicit_null() {
    let error = parse_close_client_after_request(&json!({
        "acp_close_client_after_request": null
    }))
    .expect_err("explicit null must not silently use the default");

    match error {
        crate::app::agent::session::llm::Error::Api(message::AssistantError::Unknown {
            message,
        }) => assert_eq!(
            message,
            "acp option error: acp_close_client_after_request must be a boolean"
        ),
        other => panic!("expected ACP option error, got {other:?}"),
    }
}

#[tokio::test]
async fn stream_request_reports_abort_before_config_lookup() {
    let (_tx, rx) = watch::channel(true);
    let mut events = Vec::new();

    let result = do_stream_request_acp(
        &test_model("github-copilot"),
        &test_provider(None),
        None,
        &json!({}),
        &json!([{"role": "user", "content": "hello"}]),
        "session-1",
        Some(&rx),
        &mut |event| events.push(event),
    )
    .await;

    assert!(matches!(result, Err(crate::app::agent::session::llm::Error::Aborted)));
    assert_eq!(events.len(), 1);
    match &events[0] {
        StreamEvent::Error(message::AssistantError::MessageAbortedError { message }) => {
            assert_eq!(message, "aborted");
        }
        other => panic!("expected abort stream event, got {other:?}"),
    }
}
