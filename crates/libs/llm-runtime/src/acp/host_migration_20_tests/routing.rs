use serde_json::json;
use std::collections::HashMap;
use tokio::sync::watch;
use model_types as provider;
use message_types as message;
use crate::{Error, StreamEvent};
use super::CacheHost;
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

#[tokio::test]
// 中止信号在配置和认证前检查，取消请求不应产生额外读取。
async fn stream_request_reports_abort_before_config_lookup() {
    let (_tx, rx) = watch::channel(true);
    let mut events = Vec::new();

    let result = super::super::request::do_stream_request_acp::<CacheHost>(
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

    assert!(matches!(result, Err(Error::Aborted)));
    assert_eq!(events.len(), 1);
    match &events[0] {
        StreamEvent::Error(message::AssistantError::MessageAbortedError { message }) => {
            assert_eq!(message, "aborted");
        }
        other => panic!("expected abort stream event, got {other:?}"),
    }
}

#[tokio::test]
async fn agent_model_inheritance_rejects_missing_credentials_before_launch() {
    let result = super::super::request::do_stream_request_acp::<CacheHost>(
        &test_model("github-copilot"),
        &test_provider(None),
        None,
        &json!({"acp_agent": "__must_not_launch__", "acp_model_source": "session_model"}),
        &json!([{"role": "user", "content": "hello"}]),
        "routing-no-auth",
        None,
        &mut |_| {},
    )
    .await;
    assert!(matches!(
        result,
        Err(Error::Api(message::AssistantError::ProviderAuthError { .. }))
    ));
}


#[tokio::test]
async fn acp_model_source_routing_local_reaches_command_lookup_without_client_credentials() {
    let result = super::super::request::do_stream_request_acp::<CacheHost>(
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
        Err(Error::Api(message::AssistantError::Unknown { message })) => {
            assert!(message.contains("acp command not configured"))
        }
        other => panic!("expected command lookup without client auth, got {other:?}"),
    }
}
