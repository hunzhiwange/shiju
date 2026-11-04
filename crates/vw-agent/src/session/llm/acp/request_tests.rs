//! 验证 ACP 请求选项、模型和消息载荷的构造规则。

use std::collections::HashMap;

use serde_json::json;
use tokio::sync::watch;

use crate::app::agent::auth;
use crate::app::agent::provider::provider;
use crate::app::agent::session::{llm::StreamEvent, message};

use super::{do_stream_request_acp, parse_close_client_after_request};

fn test_model(provider_id: &str) -> provider::Model {
    serde_json::from_value(json!({
        "id": "gpt-5-mini",
        "providerID": provider_id,
        "api": {
            "id": "gpt-5-mini",
            "url": "http://localhost",
            "adapter": "openai-compatible"
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
// 中止信号在配置和认证前检查，取消请求不应产生额外读取。
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

#[tokio::test]
async fn acp_model_source_routing_rejects_missing_credentials_before_launch() {
    let result = do_stream_request_acp(
        &test_model("github-copilot"),
        &test_provider(None),
        None,
        &json!({"acp_agent": "__must_not_launch__"}),
        &json!([{"role": "user", "content": "hello"}]),
        "routing-no-auth",
        None,
        &mut |_| {},
    )
    .await;
    assert!(matches!(
        result,
        Err(super::Error::Api(message::AssistantError::ProviderAuthError { .. }))
    ));
}

#[test]
fn acp_model_source_routing_validates_credentials_and_model_snapshot() {
    let mut model = test_model("github-copilot");
    let info = test_provider(Some("fixture-provider-key"));
    assert!(super::validate_session_model(&model, &info, None).is_ok());
    assert!(
        super::validate_session_model(&model, &test_provider(Some(auth::OAUTH_DUMMY_KEY)), None)
            .is_err()
    );
    let oauth = auth::Info::Oauth(auth::OauthInfo {
        access: String::new(),
        refresh: "fixture-refresh".into(),
        expires: 0,
        account_id: None,
        enterprise_url: None,
    });
    assert!(super::validate_session_model(&model, &test_provider(None), Some(&oauth)).is_ok());
    model.api.adapter = "acp".into();
    assert!(super::validate_session_model(&model, &info, None).is_err());
    model.api.adapter = "openai-compatible".into();
    model.status = "disabled".into();
    assert!(super::validate_session_model(&model, &info, None).is_err());
    model.status = "active".into();
    model.provider_id = "different-provider".into();
    assert!(super::validate_session_model(&model, &info, None).is_err());
}

#[tokio::test]
async fn acp_model_source_routing_local_reaches_command_lookup_without_client_credentials() {
    let result = do_stream_request_acp(
        &test_model("github-copilot"),
        &test_provider(None),
        None,
        &json!({"acp_agent": "__unconfigured_routing_agent__", "acp_model_source": "cli_local"}),
        &json!([{"role": "user", "content": "hello"}]),
        "routing-local",
        None,
        &mut |_| {},
    )
    .await;
    match result {
        Err(super::Error::Api(message::AssistantError::Unknown { message })) => {
            assert!(message.contains("acp command not configured"))
        }
        other => panic!("expected command lookup without client auth, got {other:?}"),
    }
}

#[cfg(unix)]
#[tokio::test]
async fn acp_model_source_launch_profile_production_request_rejects_unverified_driver() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    let directory = tempfile::tempdir().unwrap();
    let tenant = crate::session::ui_store::current_tenant_scope();
    let (project, _) = crate::project::from_directory(&tenant, directory.path()).await.unwrap();
    crate::config::update_project_for_tenant(&tenant, directory.path(), json!({
        "providers": {"github-copilot": {"api": "http://127.0.0.1:1/v1", "models": {"gpt-5-mini": {"status": "active"}}}}
    })).await.unwrap();
    let context = crate::project::instance::Context {
        directory: directory.path().to_string_lossy().into(),
        worktree: project.worktree.clone(),
        project,
    };
    let result = crate::project::instance::provide_context(
        context,
        do_stream_request_acp(
            &test_model("github-copilot"),
            &test_provider(Some("provider-secret-never-distributed")),
            None,
            &json!({"acp_agent": "codex"}),
            &json!([{"role": "user", "content": "hello"}]),
            "task9-production",
            None,
            &mut |_| {},
        ),
    )
    .await;
    match result {
        Err(super::Error::Api(message::AssistantError::Unknown { message })) => {
            assert!(message.contains("ACP codex session_model launch has not been verified"));
            assert!(!message.contains("provider-secret-never-distributed"));
        }
        other => panic!("unexpected production result: {other:?}"),
    }
}
