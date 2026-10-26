//! 验证 LLM 请求、流事件、用量和错误类型的克隆与显示契约。

use crate::app::agent::provider::provider;
use crate::app::agent::session::message;
use crate::app::agent::tools::ToolSpec;
use crate::session::ui_types as models;
use serde_json::json;
use std::collections::HashMap;

use super::{AgentInfo, Error, PromptStreamEvent, StreamEvent, StreamInput, ToolCall};

fn test_model() -> provider::Model {
    serde_json::from_value(json!({
        "id": "test-model",
        "providerID": "test-provider",
        "api": {
            "id": "test-model",
            "url": "http://localhost",
            "adapter": "openai-compatible"
        },
        "name": "Test Model",
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

fn user_info() -> message::UserInfo {
    message::UserInfo {
        id: "user-message".to_string(),
        session_id: "session-1".to_string(),
        time: message::UserTime { created: 10 },
        summary: None,
        agent: "agent".to_string(),
        model: message::ModelRef {
            provider_id: "test-provider".to_string(),
            model_id: "test-model".to_string(),
        },
        system: None,
        tools: None,
        variant: None,
    }
}

fn token_usage() -> models::TokenUsage {
    models::TokenUsage { input_tokens: 1, output_tokens: 2, cached_tokens: 3, reasoning_tokens: 4 }
}

#[test]
fn stream_input_keeps_all_request_fields() {
    let (_abort_tx, abort_rx) = tokio::sync::watch::channel(false);
    let mut agent_options = HashMap::new();
    agent_options.insert("top_k".to_string(), json!(5));
    let agent = AgentInfo {
        name: "agent".to_string(),
        mode: "chat".to_string(),
        prompt: Some("system prompt".to_string()),
        temperature: Some(0.7),
        top_p: Some(0.9),
        options: agent_options,
        permission: Vec::new(),
    };
    let mut tools = HashMap::new();
    tools.insert("test_tool".to_string(), ToolSpec::new("test_tool", "Test tool", json!({})));

    let input = StreamInput {
        user: user_info(),
        session_id: "session-1".to_string(),
        model: test_model(),
        agent: agent.clone(),
        system: vec!["system one".to_string(), "system two".to_string()],
        abort: Some(abort_rx),
        messages: vec![json!({"role": "user", "content": "hello"})],
        small: true,
        tools,
        retries: 3,
        retry_in_caller: false,
    };

    assert_eq!(input.user.id, "user-message");
    assert_eq!(input.session_id, "session-1");
    assert_eq!(input.model.id, "test-model");
    assert_eq!(input.agent.name, agent.name);
    assert_eq!(input.agent.options["top_k"], 5);
    assert_eq!(input.system.len(), 2);
    assert!(input.abort.is_some());
    assert_eq!(input.messages[0]["content"], "hello");
    assert!(input.small);
    assert!(input.tools.contains_key("test_tool"));
    assert_eq!(input.retries, 3);
}

#[test]
// 克隆请求保留显式空选项，并共享可观察的中止接收端状态。
fn stream_input_clone_preserves_empty_options_and_observes_abort_updates() {
    let (abort_tx, abort_rx) = tokio::sync::watch::channel(false);
    let input = StreamInput {
        user: user_info(),
        session_id: "session-clone".to_string(),
        model: test_model(),
        agent: AgentInfo {
            name: "clone-agent".to_string(),
            mode: "chat".to_string(),
            prompt: None,
            temperature: None,
            top_p: None,
            options: HashMap::new(),
            permission: Vec::new(),
        },
        system: Vec::new(),
        abort: Some(abort_rx),
        messages: Vec::new(),
        small: false,
        tools: HashMap::new(),
        retries: 0,
        retry_in_caller: false,
    };

    let cloned = input.clone();

    assert_eq!(cloned.session_id, "session-clone");
    assert_eq!(cloned.agent.name, "clone-agent");
    assert!(cloned.agent.prompt.is_none());
    assert!(cloned.agent.temperature.is_none());
    assert!(cloned.agent.top_p.is_none());
    assert!(cloned.agent.options.is_empty());
    assert!(cloned.system.is_empty());
    assert!(cloned.messages.is_empty());
    assert!(!cloned.small);
    assert!(cloned.tools.is_empty());
    assert_eq!(cloned.retries, 0);

    abort_tx.send(true).expect("cloned abort receivers should remain open");
    assert!(*input.abort.as_ref().expect("original abort receiver").borrow());
    assert!(*cloned.abort.as_ref().expect("cloned abort receiver").borrow());
}

#[test]
fn stream_event_variants_clone_with_payloads() {
    let tool_call = ToolCall {
        id: "call-1".to_string(),
        name: "tool".to_string(),
        arguments: "{}".to_string(),
    };
    let events = vec![
        StreamEvent::Delta("delta".to_string()),
        StreamEvent::ReasoningDelta("reasoning".to_string()),
        StreamEvent::ToolCalls(vec![tool_call.clone()]),
        StreamEvent::Done { finish_reason: Some("stop".to_string()), usage: token_usage() },
        StreamEvent::Error(message::AssistantError::Unknown { message: "boom".to_string() }),
        StreamEvent::FullMessages(vec![json!({"role": "assistant", "content": "done"})]),
    ];

    assert_eq!(tool_call.id, "call-1");
    match events[0].clone() {
        StreamEvent::Delta(text) => assert_eq!(text, "delta"),
        other => panic!("unexpected event: {other:?}"),
    }
    match events[1].clone() {
        StreamEvent::ReasoningDelta(text) => assert_eq!(text, "reasoning"),
        other => panic!("unexpected event: {other:?}"),
    }
    match events[2].clone() {
        StreamEvent::ToolCalls(calls) => assert_eq!(calls[0].name, "tool"),
        other => panic!("unexpected event: {other:?}"),
    }
    match events[3].clone() {
        StreamEvent::Done { finish_reason, usage } => {
            assert_eq!(finish_reason.as_deref(), Some("stop"));
            assert_eq!(usage.reasoning_tokens, 4);
        }
        other => panic!("unexpected event: {other:?}"),
    }
    match events[4].clone() {
        StreamEvent::Error(message::AssistantError::Unknown { message }) => {
            assert_eq!(message, "boom");
        }
        other => panic!("unexpected event: {other:?}"),
    }
    match events[5].clone() {
        StreamEvent::FullMessages(messages) => assert_eq!(messages[0]["role"], "assistant"),
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn prompt_stream_event_variants_clone_with_payloads() {
    match PromptStreamEvent::Delta("chunk".to_string()).clone() {
        PromptStreamEvent::Delta(text) => assert_eq!(text, "chunk"),
        other => panic!("unexpected event: {other:?}"),
    }
    match PromptStreamEvent::Done(token_usage()).clone() {
        PromptStreamEvent::Done(usage) => assert_eq!(usage.output_tokens, 2),
        other => panic!("unexpected event: {other:?}"),
    }
    match PromptStreamEvent::Error("prompt failed".to_string()).clone() {
        PromptStreamEvent::Error(message) => assert_eq!(message, "prompt failed"),
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn error_display_formats_provider_abort_and_unknown_variants() {
    assert_eq!(
        Error::ProviderNotFound("missing-provider".to_string()).to_string(),
        "provider not found: missing-provider"
    );
    assert_eq!(Error::Aborted.to_string(), "aborted");
    assert_eq!(
        Error::Api(message::AssistantError::Unknown {
            message: "acp: session changed".to_string(),
        })
        .to_string(),
        "session changed"
    );
    assert_eq!(
        Error::Api(message::AssistantError::Unknown { message: "plain".to_string() }).to_string(),
        "plain"
    );
    assert_eq!(
        Error::Api(message::AssistantError::Unknown { message: "acp: acp: nested".to_string() })
            .to_string(),
        "acp: nested"
    );
}

#[test]
// 结构化助手错误的显示文本属于跨层诊断契约。
fn error_display_serializes_each_structured_assistant_error_variant() {
    let cases = [
        (
            message::AssistantError::ProviderAuthError {
                provider_id: "test-provider".to_string(),
                message: "authentication failed".to_string(),
            },
            json!({
                "name": "ProviderAuthError",
                "provider_id": "test-provider",
                "message": "authentication failed"
            }),
        ),
        (
            message::AssistantError::MessageOutputLengthError,
            json!({"name": "MessageOutputLengthError"}),
        ),
        (
            message::AssistantError::MessageAbortedError { message: "cancelled".to_string() },
            json!({"name": "MessageAbortedError", "message": "cancelled"}),
        ),
        (
            message::AssistantError::ContextOverflowError {
                message: "context is full".to_string(),
                response_body: None,
            },
            json!({"name": "ContextOverflowError", "message": "context is full"}),
        ),
        (
            message::AssistantError::APIError {
                message: "rate limited".to_string(),
                status_code: Some(429),
                is_retryable: true,
                response_headers: Some(HashMap::from([(
                    "retry-after".to_string(),
                    "5".to_string(),
                )])),
                response_body: Some("slow down".to_string()),
                metadata: Some(HashMap::from([("request_id".to_string(), "req-1".to_string())])),
            },
            json!({
                "name": "APIError",
                "message": "rate limited",
                "status_code": 429,
                "is_retryable": true,
                "response_headers": {"retry-after": "5"},
                "response_body": "slow down",
                "metadata": {"request_id": "req-1"}
            }),
        ),
    ];

    for (error, expected) in cases {
        let rendered = Error::Api(error).to_string();
        let serialized: serde_json::Value =
            serde_json::from_str(&rendered).expect("structured error should render as JSON");
        assert_eq!(serialized, expected);
    }
}

#[test]
fn reqwest_error_conversion_preserves_http_display() {
    let reqwest_error =
        reqwest::Client::new().get("http://[::1").build().err().expect("invalid URL should fail");
    let expected = reqwest_error.to_string();
    let wrapped: Error = reqwest_error.into();
    assert_eq!(wrapped.to_string(), expected);
}
