//! 验证流式请求中止、重试判定、ACP 路由和供应商解析。

use super::super::types::{AgentInfo, Error, StreamInput};
use super::*;

use crate::app::agent::provider::provider;
use crate::app::agent::session::message;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

fn test_model(provider_id: &str, api_id: &str, adapter: &str) -> provider::Model {
    serde_json::from_value(json!({
        "id": api_id,
        "providerID": provider_id,
        "api": {
            "id": api_id,
            "url": "http://localhost",
            "adapter": adapter
        },
        "name": api_id,
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

fn test_input(provider_id: &str) -> StreamInput {
    let model = test_model(provider_id, "model-1", "openai-compatible");
    StreamInput {
        user: message::UserInfo {
            id: "user-1".to_string(),
            session_id: "session-1".to_string(),
            time: message::UserTime { created: 123 },
            summary: None,
            agent: "build".to_string(),
            model: message::ModelRef {
                provider_id: model.provider_id.clone(),
                model_id: model.id.clone(),
            },
            system: None,
            tools: None,
            variant: None,
        },
        session_id: "session-1".to_string(),
        model,
        agent: AgentInfo {
            name: "build".to_string(),
            mode: "build".to_string(),
            prompt: None,
            temperature: None,
            top_p: None,
            options: HashMap::new(),
            permission: Default::default(),
        },
        system: Vec::new(),
        abort: None,
        messages: vec![json!({ "role": "user", "content": "hello" })],
        small: false,
        tools: HashMap::new(),
        retries: 0,
    }
}

#[test]
fn should_abort_reflects_watch_receiver_state() {
    assert!(!should_abort(None));

    let (tx, rx) = tokio::sync::watch::channel(false);
    assert!(!should_abort(Some(&rx)));

    tx.send(true).expect("abort signal should send");
    assert!(should_abort(Some(&rx)));
}

#[test]
fn should_abort_reads_the_latest_watch_value() {
    let (tx, rx) = tokio::sync::watch::channel(true);
    assert!(should_abort(Some(&rx)));

    tx.send(false).expect("resume signal should send");
    assert!(!should_abort(Some(&rx)));
}

#[test]
// 重试只接受供应商显式可重试错误或 ACP 会话变化信号。
fn is_retryable_assistant_error_uses_api_retry_flag_and_acp_session_change() {
    assert!(is_retryable_assistant_error(&message::AssistantError::APIError {
        message: "temporary".to_string(),
        status_code: Some(503),
        is_retryable: true,
        response_headers: None,
        response_body: None,
        metadata: None,
    }));
    assert!(!is_retryable_assistant_error(&message::AssistantError::APIError {
        message: "bad request".to_string(),
        status_code: Some(400),
        is_retryable: false,
        response_headers: None,
        response_body: None,
        metadata: None,
    }));
    assert!(is_retryable_assistant_error(&message::AssistantError::Unknown {
        message: "acp session changed: expected=a actual=b".to_string(),
    }));
    assert!(!is_retryable_assistant_error(&message::AssistantError::MessageAbortedError {
        message: "aborted".to_string(),
    }));
}

#[test]
fn is_retryable_assistant_error_rejects_non_matching_unknown_errors() {
    assert!(!is_retryable_assistant_error(&message::AssistantError::Unknown {
        message: "session changed without the ACP marker".to_string(),
    }));
}

#[test]
fn is_acp_retryable_assistant_error_accepts_transient_acp_failures() {
    for message in [
        "timed out waiting for agent",
        "timeout while prompting",
        "ACP agent disconnected during request",
        "queue owner disconnected before prompt completion",
        "acp session changed: expected=a actual=b",
        "API error: None - error decoding response body",
    ] {
        assert!(is_acp_retryable_assistant_error(&message::AssistantError::Unknown {
            message: message.to_string(),
        }));
    }

    assert!(is_acp_retryable_assistant_error(&message::AssistantError::APIError {
        message: "EOF while parsing a value".to_string(),
        status_code: Some(504),
        is_retryable: false,
        response_headers: None,
        response_body: None,
        metadata: None,
    }));
    assert!(!is_acp_retryable_assistant_error(&message::AssistantError::ProviderAuthError {
        provider_id: "openai".to_string(),
        message: "missing key".to_string(),
    }));
}

#[test]
fn is_acp_retryable_assistant_error_rejects_persistent_api_failures() {
    assert!(!is_acp_retryable_assistant_error(&message::AssistantError::APIError {
        message: "invalid request".to_string(),
        status_code: Some(400),
        is_retryable: true,
        response_headers: None,
        response_body: None,
        metadata: None,
    }));
    assert!(!is_acp_retryable_assistant_error(&message::AssistantError::Unknown {
        message: "agent returned a validation error".to_string(),
    }));
}

#[test]
fn is_acp_retryable_assistant_error_accepts_transient_api_error_messages() {
    for error_message in [
        "ACP SESSION CHANGED: expected=a actual=b",
        "request timed out while waiting for the agent",
        "timeout while prompting",
        "ACP agent disconnected during request",
        "API error: None - error decoding response body",
    ] {
        let error = message::AssistantError::APIError {
            message: error_message.to_string(),
            status_code: None,
            is_retryable: false,
            response_headers: None,
            response_body: None,
            metadata: None,
        };

        assert!(is_acp_retryable_assistant_error(&error), "message: {error_message}");
    }
}

#[test]
fn is_acp_session_changed_message_is_case_insensitive() {
    assert!(is_acp_session_changed_message("ACP SESSION CHANGED: expected=a actual=b"));
    assert!(is_acp_session_changed_message("prefix acp session changed: expected=a actual=b"));
    assert!(!is_acp_session_changed_message("session changed without acp prefix"));
}

#[test]
fn resolve_stream_provider_info_allows_acp_route_provider_for_acp_requests() {
    let info = resolve_stream_provider_info("acp", None, true)
        .expect("acp route provider should be synthesized for acp requests");

    assert_eq!(info.id, "acp");
    assert!(matches!(info.source, provider::ProviderSource::Config));
}

#[test]
fn resolve_stream_provider_info_preserves_an_existing_provider() {
    let mut existing = acp_route_provider_info();
    existing.id = "configured-provider".to_string();
    existing.name = "Configured Provider".to_string();

    let info = resolve_stream_provider_info("configured-provider", Some(existing), false)
        .expect("an existing provider should be returned");

    assert_eq!(info.id, "configured-provider");
    assert_eq!(info.name, "Configured Provider");
}

#[test]
// 缺供应商时只有显式 ACP 请求可使用虚拟路由，普通请求必须报错。
fn resolve_stream_provider_info_requires_an_explicit_acp_request_for_fallback() {
    let error = resolve_stream_provider_info("acp", None, false)
        .expect_err("the ACP fallback should not be enabled implicitly");

    match error {
        Error::ProviderNotFound(provider_id) => assert_eq!(provider_id, "acp"),
        other => panic!("expected provider not found, got {other:?}"),
    }
}

#[test]
fn resolve_stream_provider_info_rejects_non_acp_missing_provider_for_acp_requests() {
    let error = resolve_stream_provider_info("__missing_provider__", None, true)
        .expect_err("explicit missing providers should still fail");

    match error {
        Error::ProviderNotFound(provider_id) => assert_eq!(provider_id, "__missing_provider__"),
        other => panic!("expected provider not found, got {other:?}"),
    }
}

#[tokio::test]
async fn stream_returns_provider_not_found_before_requesting() {
    let input = test_input("__missing_provider_for_llm_stream_test__");
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_for_callback = events.clone();

    let result = stream(input, move |event| {
        events_for_callback.lock().unwrap().push(event);
    })
    .await;

    match result {
        Err(Error::ProviderNotFound(provider_id)) => {
            assert_eq!(provider_id, "__missing_provider_for_llm_stream_test__");
        }
        other => panic!("expected provider not found, got {other:?}"),
    }
    assert!(events.lock().unwrap().is_empty());
}

// =============================================================================
// request_options_map_enable_acp
// =============================================================================

#[test]
fn request_options_map_enable_acp_false_when_key_is_missing() {
    assert!(!request_options_map_enable_acp(&HashMap::new()));
}

#[test]
fn request_options_map_enable_acp_true_when_acp_test_is_true() {
    let mut options = HashMap::new();
    options.insert("acp_test".to_string(), Value::Bool(true));
    assert!(request_options_map_enable_acp(&options));
}

#[test]
fn request_options_map_enable_acp_false_when_acp_test_is_false() {
    let mut options = HashMap::new();
    options.insert("acp_test".to_string(), Value::Bool(false));
    assert!(!request_options_map_enable_acp(&options));
}

#[test]
fn request_options_map_enable_acp_true_when_acp_agent_is_non_empty() {
    let mut options = HashMap::new();
    options.insert("acp_agent".to_string(), Value::String("codex".to_string()));
    assert!(request_options_map_enable_acp(&options));
}

#[test]
fn request_options_map_enable_acp_false_when_acp_agent_is_empty() {
    let mut options = HashMap::new();
    options.insert("acp_agent".to_string(), Value::String(String::new()));
    assert!(!request_options_map_enable_acp(&options));
}

#[test]
fn request_options_map_enable_acp_false_when_acp_agent_is_whitespace() {
    let mut options = HashMap::new();
    options.insert("acp_agent".to_string(), Value::String("   ".to_string()));
    assert!(!request_options_map_enable_acp(&options));
}

#[test]
fn request_options_map_enable_acp_accepts_agent_when_test_flag_is_false() {
    let mut options = HashMap::new();
    options.insert("acp_test".to_string(), Value::Bool(false));
    options.insert("acp_agent".to_string(), Value::String("codex".to_string()));

    assert!(request_options_map_enable_acp(&options));
}

#[test]
fn request_options_map_enable_acp_rejects_wrong_value_types() {
    let mut options = HashMap::new();
    options.insert("acp_test".to_string(), Value::String("true".to_string()));
    options.insert("acp_agent".to_string(), Value::Bool(true));

    assert!(!request_options_map_enable_acp(&options));
}

// =============================================================================
// request_options_value_enable_acp
// =============================================================================

#[test]
fn request_options_value_enable_acp_false_when_no_relevant_keys() {
    assert!(!request_options_value_enable_acp(&json!({})));
    assert!(!request_options_value_enable_acp(&json!({ "unrelated": "value" })));
}

#[test]
fn request_options_value_enable_acp_true_when_acp_test_is_true() {
    assert!(request_options_value_enable_acp(&json!({ "acp_test": true })));
}

#[test]
fn request_options_value_enable_acp_false_when_acp_test_is_false() {
    assert!(!request_options_value_enable_acp(&json!({ "acp_test": false })));
}

#[test]
fn request_options_value_enable_acp_true_when_acp_agent_is_non_empty() {
    assert!(request_options_value_enable_acp(&json!({ "acp_agent": "opencode" })));
}

#[test]
fn request_options_value_enable_acp_false_when_acp_agent_is_empty_string() {
    assert!(!request_options_value_enable_acp(&json!({ "acp_agent": "" })));
}

#[test]
fn request_options_value_enable_acp_false_when_acp_agent_is_whitespace() {
    assert!(!request_options_value_enable_acp(&json!({ "acp_agent": "   " })));
}

#[test]
fn request_options_value_enable_acp_accepts_agent_when_test_flag_is_false() {
    assert!(request_options_value_enable_acp(&json!({
        "acp_test": false,
        "acp_agent": "opencode"
    })));
}

#[test]
fn request_options_value_enable_acp_rejects_non_object_and_wrong_value_types() {
    for options in [
        json!(null),
        json!([]),
        json!("acp"),
        json!({
            "acp_test": "true",
            "acp_agent": true
        }),
    ] {
        assert!(!request_options_value_enable_acp(&options));
    }
}

// =============================================================================
// acp_route_provider_info
// =============================================================================

#[test]
fn acp_route_provider_info_has_expected_structure() {
    let info = acp_route_provider_info();

    assert_eq!(info.id, "acp");
    assert_eq!(info.name, "ACP");
    assert!(matches!(info.source, provider::ProviderSource::Config));
    assert!(info.env.is_empty());
    assert!(info.key.is_none());
    assert!(info.options.is_empty());
    assert!(info.models.is_empty());
}

#[tokio::test]
async fn stream_cancellation_prevents_provider_setup() {
    let (sender, receiver) = tokio::sync::watch::channel(true);
    let mut input = test_input("missing-provider");
    input.abort = Some(receiver);
    assert!(matches!(
        super::stream(input, |_| panic!("cancelled request must not emit events")).await,
        Err(Error::Aborted)
    ));
    drop(sender);
}
