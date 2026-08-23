//! 验证模型解析、ACP 路由和提示入口的前置错误处理。

use super::super::Error;
use super::super::types::{PromptStreamEvent, StreamEvent};
use super::*;

use crate::app::agent::provider::provider;
use crate::app::agent::session::message;
use serde_json::json;
use std::sync::{Arc, Mutex};

fn test_model(provider_id: &str, api_id: &str) -> provider::Model {
    serde_json::from_value(json!({
        "id": api_id,
        "providerID": provider_id,
        "api": {
            "id": api_id,
            "url": "http://localhost",
            "adapter": "openai-compatible"
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

#[test]
fn now_ms_returns_a_unix_timestamp() {
    assert!(now_ms() > 0);
}

#[test]
fn dummy_user_carries_session_and_model_references() {
    let model = test_model("provider-1", "model-1");

    let user = dummy_user("session-1", &model);

    assert!(user.id.starts_with("prompt-"));
    assert_eq!(user.session_id, "session-1");
    assert_eq!(user.agent, "main");
    assert_eq!(user.model.provider_id, "provider-1");
    assert_eq!(user.model.model_id, "model-1");
    assert!(user.time.created > 0);
    assert!(user.summary.is_none());
    assert!(user.system.is_none());
    assert!(user.tools.is_none());
    assert!(user.variant.is_none());
}

#[test]
fn dummy_user_uses_public_model_id_instead_of_adapter_api_id() {
    let mut model = test_model("provider-1", "adapter-model");
    model.id = "public-model".to_string();

    let user = dummy_user("session-1", &model);

    assert_eq!(user.model.provider_id, "provider-1");
    assert_eq!(user.model.model_id, "public-model");
}

#[test]
fn request_options_enable_acp_requires_true_test_flag_or_non_blank_agent() {
    for options in [
        json!({}),
        json!({ "acp_test": false }),
        json!({ "acp_test": "true" }),
        json!({ "acp_agent": null }),
        json!({ "acp_agent": 42 }),
        json!({ "acp_agent": "   " }),
    ] {
        assert!(!request_options_enable_acp(&options), "options: {options}");
    }

    assert!(request_options_enable_acp(&json!({ "acp_test": true })));
    assert!(request_options_enable_acp(&json!({ "acp_agent": " codex " })));
}

#[test]
fn request_options_enable_acp_rejects_non_object_options() {
    for options in [json!(null), json!(false), json!([]), json!("acp")] {
        assert!(!request_options_enable_acp(&options), "options: {options}");
    }
}

#[test]
fn acp_route_model_defaults_to_acp_with_restricted_capabilities() {
    let model = acp_route_model(&json!({ "acp_test": true, "acp_agent": "   " }));

    assert_eq!(model.id, "acp");
    assert_eq!(model.provider_id, "acp");
    assert_eq!(model.api.id, "acp");
    assert_eq!(model.api.adapter, "acp");
    assert!(model.api.url.is_empty());
    assert_eq!(model.name, "acp");
    assert!(model.family.is_none());
    assert!(!model.capabilities.temperature);
    assert!(!model.capabilities.reasoning);
    assert!(!model.capabilities.attachment);
    assert!(model.capabilities.toolcall);
    assert!(model.capabilities.input.text);
    assert!(!model.capabilities.input.audio);
    assert!(!model.capabilities.input.image);
    assert!(!model.capabilities.input.video);
    assert!(!model.capabilities.input.pdf);
    assert!(model.capabilities.output.text);
    assert!(!model.capabilities.output.audio);
    assert!(!model.capabilities.output.image);
    assert!(!model.capabilities.output.video);
    assert!(!model.capabilities.output.pdf);
    assert!(matches!(model.capabilities.interleaved, provider::InterleavedCapability::Bool(false)));
    assert_eq!(model.cost.input, 0.0);
    assert_eq!(model.cost.output, 0.0);
    assert_eq!(model.cost.cache.read, 0.0);
    assert_eq!(model.cost.cache.write, 0.0);
    assert!(model.cost.experimental_over_200k.is_none());
    assert_eq!(model.limit.context, 8192);
    assert!(model.limit.input.is_none());
    assert_eq!(model.limit.output, 4096);
    assert_eq!(model.status, "active");
    assert!(model.options.is_empty());
    assert!(model.headers.is_empty());
    assert_eq!(model.release_date, "1970-01-01");
    assert!(model.variants.is_empty());
}

#[test]
fn acp_route_model_trims_named_agent_for_all_model_identifiers() {
    let model = acp_route_model(&json!({ "acp_agent": "  codex  " }));

    assert_eq!(model.id, "codex");
    assert_eq!(model.api.id, "codex");
    assert_eq!(model.name, "codex");
}

#[test]
fn acp_route_model_uses_default_for_non_string_agent() {
    for options in [json!({ "acp_agent": null }), json!({ "acp_agent": 7 })] {
        let model = acp_route_model(&options);
        assert_eq!(model.id, "acp");
        assert_eq!(model.api.id, "acp");
        assert_eq!(model.name, "acp");
    }
}

#[test]
fn explicit_model_bypasses_auto() {
    assert!(!is_auto_model_request(Some("provider/model")));
    assert!(is_auto_model_request(Some("auto")));
    assert!(is_auto_model_request(None));
}

#[test]
fn acp_route_bypasses_auto_selector() {
    assert!(request_bypasses_auto_selector(None, &json!({ "acp_agent": "codex" })));
    assert!(request_bypasses_auto_selector(
        Some("provider/model"),
        &json!({ "acp_agent": "codex" })
    ));
    assert!(!request_bypasses_auto_selector(Some("auto"), &json!({})));
}

#[tokio::test]
async fn resolve_model_for_options_uses_acp_route_model_without_explicit_model() {
    let model = resolve_model_for_options(None, &json!({ "acp_agent": "codex" }))
        .await
        .expect("acp request without model should use acp route model");

    assert_eq!(model.provider_id, "acp");
    assert_eq!(model.id, "codex");
    assert_eq!(model.api.adapter, "acp");
}

#[tokio::test]
async fn resolve_model_for_options_treats_blank_model_as_absent_for_acp_test() {
    let model = resolve_model_for_options(Some("   ".to_string()), &json!({ "acp_test": true }))
        .await
        .expect("blank model should allow the ACP test route");

    assert_eq!(model.provider_id, "acp");
    assert_eq!(model.id, "acp");
}

#[tokio::test]
// ACP 路由是协议边界，启用时优先于普通显式模型选择。
async fn resolve_model_for_options_prioritizes_acp_over_explicit_model() {
    let model = resolve_model_for_options(
        Some("__missing_provider_for_acp__/missing-model".to_string()),
        &json!({ "acp_agent": "codex" }),
    )
    .await
    .expect("ACP should bypass normal explicit model parsing");

    assert_eq!(model.provider_id, "acp");
    assert_eq!(model.id, "codex");
}

#[tokio::test]
async fn resolve_model_rejects_unknown_bare_model() {
    let model_id = "__vw_prompt_test_missing_model__";

    let error = resolve_model(Some(model_id.to_string()))
        .await
        .expect_err("an unknown bare model should not be resolved");

    assert_eq!(error, format!("未找到模型：{model_id}"));
}

#[tokio::test]
async fn resolve_model_trims_malformed_qualified_model_in_error() {
    let error = resolve_model(Some("  provider/  ".to_string()))
        .await
        .expect_err("a missing model component should be rejected");

    assert_eq!(error, "模型格式错误：provider/，请使用 provider/model");
}

#[test]
fn block_on_runs_futures_without_an_existing_runtime() {
    let value = block_on(async { 42 });

    assert_eq!(value, 42);
}

#[tokio::test(flavor = "multi_thread")]
// 同步入口在已有运行时内使用兼容路径，避免嵌套启动 Tokio。
async fn block_on_runs_futures_inside_a_multi_thread_runtime() {
    let value = block_on(async { "inside runtime".to_string() });

    assert_eq!(value, "inside runtime");
}

#[test]
fn assistant_error_to_string_formats_all_variants() {
    assert_eq!(
        assistant_error_to_string(&message::AssistantError::ProviderAuthError {
            provider_id: "openai".to_string(),
            message: "missing key".to_string(),
        }),
        "未配置 openai 的 API Key：missing key"
    );
    assert_eq!(
        assistant_error_to_string(&message::AssistantError::MessageOutputLengthError),
        "模型输出过长"
    );
    assert_eq!(
        assistant_error_to_string(&message::AssistantError::MessageAbortedError {
            message: "stopped".to_string(),
        }),
        "stopped"
    );
    assert_eq!(
        assistant_error_to_string(&message::AssistantError::ContextOverflowError {
            message: "too long".to_string(),
            response_body: Some("body".to_string()),
        }),
        "too long"
    );
    assert_eq!(
        assistant_error_to_string(&message::AssistantError::APIError {
            message: "rate limited".to_string(),
            status_code: Some(429),
            is_retryable: true,
            response_headers: None,
            response_body: None,
            metadata: None,
        }),
        "rate limited"
    );
    assert_eq!(
        assistant_error_to_string(&message::AssistantError::Unknown {
            message: "unknown".to_string(),
        }),
        "unknown"
    );
}

#[test]
fn missing_stream_error_message_maps_outer_errors_only() {
    assert!(missing_stream_error_message(&Ok(())).is_none());
    assert!(missing_stream_error_message(&Err(Error::Aborted)).is_none());
    assert!(
        missing_stream_error_message(&Err(Error::Api(message::AssistantError::Unknown {
            message: "already emitted".to_string(),
        })))
        .is_none()
    );
    assert_eq!(
        missing_stream_error_message(&Err(Error::ProviderNotFound("missing-provider".to_string()))),
        Some("未找到 provider：missing-provider".to_string())
    );
}

#[test]
fn missing_stream_error_message_exposes_http_build_error() {
    let error = reqwest::Client::new()
        .get("://")
        .build()
        .expect_err("invalid URL should fail before any request is sent");
    let expected = error.to_string();

    assert_eq!(missing_stream_error_message(&Err(Error::Http(error))), Some(expected));
}

#[test]
fn prompt_entrypoints_report_parse_errors_without_streaming() {
    let prompt_events = Arc::new(Mutex::new(Vec::new()));
    let prompt_events_for_callback = prompt_events.clone();
    stream_prompt("hello", Some("/model".to_string()), move |event| {
        prompt_events_for_callback.lock().unwrap().push(event);
    });

    let events = prompt_events.lock().unwrap();
    assert_eq!(events.len(), 1);
    match &events[0] {
        PromptStreamEvent::Error(message) => assert!(message.contains("模型格式错误")),
        other => panic!("expected error event, got {other:?}"),
    }
    drop(events);

    let tool_events = Arc::new(Mutex::new(Vec::new()));
    let tool_events_for_callback = tool_events.clone();
    stream_prompt_with_tools(
        "hello",
        Some("provider/".to_string()),
        Default::default(),
        move |event| {
            tool_events_for_callback.lock().unwrap().push(event);
        },
    );

    let events = tool_events.lock().unwrap();
    assert_eq!(events.len(), 1);
    match &events[0] {
        StreamEvent::Error(message::AssistantError::Unknown { message }) => {
            assert!(message.contains("模型格式错误"));
        }
        other => panic!("expected stream error event, got {other:?}"),
    }
}

#[test]
fn chat_entrypoint_reports_parse_errors_without_streaming() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_for_callback = events.clone();

    stream_chat_with_tools_for_session(
        "session-1",
        vec![json!({ "role": "user", "content": "hello" })],
        vec!["system".to_string()],
        Some("provider/".to_string()),
        json!({ "temperature": 0.5 }),
        Default::default(),
        move |event| {
            events_for_callback.lock().unwrap().push(event);
        },
    );

    let events = events.lock().unwrap();
    assert_eq!(events.len(), 1);
    match &events[0] {
        StreamEvent::Error(message::AssistantError::Unknown { message }) => {
            assert!(message.contains("模型格式错误"));
        }
        other => panic!("expected stream error event, got {other:?}"),
    }
}

#[test]
fn public_chat_entrypoint_reports_parse_error_without_streaming() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_for_callback = events.clone();

    stream_chat_with_tools(
        vec![json!({ "role": "user", "content": "hello" })],
        vec!["system".to_string()],
        Some("provider/".to_string()),
        json!({}),
        Default::default(),
        move |event| {
            events_for_callback.lock().unwrap().push(event);
        },
    );

    let events = events.lock().unwrap();
    assert_eq!(events.len(), 1);
    match &events[0] {
        StreamEvent::Error(message::AssistantError::Unknown { message }) => {
            assert!(message.contains("模型格式错误"));
        }
        other => panic!("expected stream error event, got {other:?}"),
    }
}

#[test]
fn send_prompt_returns_parse_errors_without_streaming() {
    let err = send_prompt("hello", Some("provider/".to_string()))
        .expect_err("invalid model ref should fail before streaming");

    assert!(err.contains("模型格式错误"));
}
