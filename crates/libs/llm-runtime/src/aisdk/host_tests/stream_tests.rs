//! 验证 AI SDK 流事件、用量、结束原因与错误传播。

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use aisdk::Error as AiError;
use aisdk::core::DynamicModel;
use aisdk::core::capabilities::ToolCallSupport;
use aisdk::core::language_model::{
    LanguageModel, LanguageModelOptions, LanguageModelResponse,
    LanguageModelResponseContentType as AiContent, LanguageModelStreamChunk as AiStreamChunk,
    LanguageModelStreamChunkType, ReasoningEffort, Usage,
};
use aisdk::core::tools::ToolCallInfo;
use aisdk::core::{AssistantMessage, Message};
use aisdk::providers::{AlibabaCn, OpenAICompatible};
use async_trait::async_trait;
use futures_util::Stream;
use serde_json::{Value, json};

use crate::app::agent::session::llm::types::{Error, StreamEvent};
use crate::app::agent::session::message::AssistantError;
use crate::app::agent::tools;

use super::do_stream_request_aisdk_with_model;

#[path = "../stream_test_support.rs"]
mod support;
use support::*;

#[tokio::test]
async fn request_options_scale_values_use_stop_fallback_and_ignore_wrong_types() {
    let (model, captured) = FakeModel::with_items(vec![]);
    let mut events = Vec::new();
    let options = json!({
        "presence_penalty": "0.25",
        "frequency_penalty": true,
        "seed": -1,
        "top_k": 1.5,

        "stop": " END "
    });

    let result = do_stream_request_aisdk_with_model(
        "openai".to_string(),
        model,
        "https://api.openai.com/v1/chat/completions".to_string(),
        false,
        false,
        vec![Message::User("hello".into())],
        None,
        &HashMap::new(),
        Some(0.555),
        Some(0.444),
        None,
        &options,
        7,
        None,
        &mut |event| events.push(event),
    )
    .await;

    assert!(result.is_ok());
    assert!(matches!(events.as_slice(), [StreamEvent::Done { .. }]));

    let captured =
        captured.lock().expect("capture lock").clone().expect("stream options should be captured");
    assert_eq!(captured.temperature, Some(56));
    assert_eq!(captured.top_p, Some(44));
    assert_eq!(captured.max_retries, Some(7));
    assert_eq!(captured.max_output_tokens, None);
    assert_eq!(captured.presence_penalty, None);
    assert_eq!(captured.frequency_penalty, None);
    assert_eq!(captured.seed, None);
    assert_eq!(captured.top_k, None);
    assert!(captured.reasoning_effort.is_none());
    assert_eq!(captured.stop_sequences, Some(vec!["END".to_string()]));
}

#[tokio::test]
async fn stop_sequences_field_prevents_fallback_to_stop_when_empty() {
    let (model, captured) = FakeModel::with_items(vec![]);
    let mut events = Vec::new();

    let result = do_stream_request_aisdk_with_model(
        "openai".to_string(),
        model,
        "https://api.openai.com/v1/chat/completions".to_string(),
        false,
        false,
        vec![Message::User("hello".into())],
        None,
        &HashMap::new(),
        None,
        None,
        None,
        &json!({ "stop_sequences": [], "stop": "fallback" }),
        0,
        None,
        &mut |event| events.push(event),
    )
    .await;

    assert!(result.is_ok());
    let captured =
        captured.lock().expect("capture lock").clone().expect("stream options should be captured");
    assert_eq!(captured.stop_sequences, None);
}

#[tokio::test]
// 工具定义按键稳定排序，保证请求快照和测试结果可复现。
async fn tool_definitions_are_added_in_sorted_key_order() {
    let (model, captured) = FakeModel::with_items(vec![]);
    let tools = HashMap::from([
        (
            "z-key".to_string(),
            tools::ToolSpec::new("zeta", "Zeta tool", json!({ "type": "object" })),
        ),
        (
            "a-key".to_string(),
            tools::ToolSpec::new("alpha", "Alpha tool", json!({ "type": "object" })),
        ),
    ]);
    let mut events = Vec::new();

    let result = do_stream_request_aisdk_with_model(
        "openai".to_string(),
        model,
        "https://api.openai.com/v1/chat/completions".to_string(),
        false,
        false,
        vec![Message::User("hello".into())],
        None,
        &tools,
        None,
        None,
        None,
        &Value::Null,
        0,
        None,
        &mut |event| events.push(event),
    )
    .await;

    assert!(result.is_ok());
    let captured =
        captured.lock().expect("capture lock").clone().expect("stream options should be captured");
    let debug = format!("{captured:?}");
    let alpha = debug.find("alpha").expect("alpha tool should be present");
    let zeta = debug.find("zeta").expect("zeta tool should be present");
    assert!(alpha < zeta, "tools were not sorted: {debug}");
}

#[tokio::test]
async fn dashscope_reasoning_option_is_sent_in_extra_body() {
    let (model, captured) = FakeModel::with_items(vec![]);
    let mut events = Vec::new();

    let result = do_stream_request_aisdk_with_model(
        "alibaba-cn".to_string(),
        model,
        "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions".to_string(),
        false,
        false,
        vec![Message::User("hello".into())],
        None,
        &HashMap::new(),
        None,
        None,
        None,
        &json!({ "reasoning_effort": "low" }),
        0,
        None,
        &mut |event| events.push(event),
    )
    .await;

    assert!(result.is_ok());
    assert!(matches!(
        events.as_slice(),
        [StreamEvent::Done { finish_reason, .. }] if finish_reason.as_deref() == Some("stop")
    ));

    let captured =
        captured.lock().expect("capture lock").clone().expect("stream options should be captured");
    assert_eq!(captured.reasoning_effort, Some(ReasoningEffort::Low));
    assert_eq!(
        captured.body.as_ref().and_then(|body| body.get("enable_thinking")),
        Some(&json!(true))
    );
}

#[tokio::test]
async fn stream_open_failure_maps_aisdk_error() {
    let model = FakeModel::with_open_error(AiError::MissingField("api_key".to_string()));

    let (result, events) = run_fake_stream(
        "openai",
        "https://api.openai.com/v1/chat/completions",
        model,
        &json!({}),
        None,
    )
    .await;

    assert!(events.is_empty());
    assert!(matches!(
        result,
        Err(Error::Api(AssistantError::ProviderAuthError { provider_id, message }))
            if provider_id == "openai" && message == "api_key"
    ));
}

#[tokio::test]
async fn dynamic_provider_stream_open_failures_execute_stream_setup() {
    let openai_model = OpenAICompatible::<DynamicModel>::builder()
        .provider_name("openai")
        .api_key("test-token")
        .model_name("gpt-test")
        .base_url("http://127.0.0.1:9/v1")
        .build()
        .expect("openai compatible model");
    let alibaba_model = AlibabaCn::<DynamicModel>::builder()
        .provider_name("alibaba-cn")
        .api_key("test-token")
        .model_name("gpt-test")
        .base_url("http://127.0.0.1:9/compatible-mode/v1")
        .build()
        .expect("alibaba model");

    let mut events = Vec::new();
    let openai_result = do_stream_request_aisdk_with_model(
        "openai".to_string(),
        openai_model,
        "http://127.0.0.1:9/v1/chat/completions".to_string(),
        false,
        false,
        vec![Message::User("hello".into())],
        None,
        &HashMap::new(),
        Some(0.2),
        Some(0.8),
        Some(16),
        &json!({ "reasoning_effort": "low" }),
        0,
        None,
        &mut |event| events.push(event),
    )
    .await;
    assert!(matches!(openai_result, Err(Error::Api(_))));

    let alibaba_result = do_stream_request_aisdk_with_model(
        "alibaba-cn".to_string(),
        alibaba_model,
        "http://127.0.0.1:9/compatible-mode/v1/chat/completions".to_string(),
        false,
        false,
        vec![Message::User("hello".into())],
        None,
        &HashMap::new(),
        Some(0.2),
        Some(0.8),
        Some(16),
        &json!({ "reasoning_effort": "low" }),
        0,
        None,
        &mut |event| events.push(event),
    )
    .await;
    assert!(matches!(alibaba_result, Err(Error::Api(_))));
    assert!(events.is_empty());
}

#[tokio::test]
async fn stream_read_failure_maps_aisdk_api_error() {
    let (model, _) = FakeModel::with_items(vec![Err(AiError::ApiError {
        details: "read failed".to_string(),
        status_code: Some(reqwest::StatusCode::INTERNAL_SERVER_ERROR),
    })]);

    let (result, events) = run_fake_stream(
        "openai",
        "https://api.openai.com/v1/chat/completions",
        model,
        &json!({}),
        None,
    )
    .await;

    assert!(events.is_empty());
    assert!(matches!(
        result,
        Err(Error::Api(AssistantError::APIError {
            message,
            status_code: Some(500),
            is_retryable: true,
            ..
        })) if message == "read failed"
    ));
}

#[tokio::test]
async fn stream_chunk_failure_variants_return_api_errors() {
    for (chunk, expected_kind, expected_message) in [
        (LanguageModelStreamChunkType::Failed("failed".to_string()), "failed", "failed"),
        (
            LanguageModelStreamChunkType::Incomplete("incomplete".to_string()),
            "incomplete",
            "incomplete",
        ),
        (
            LanguageModelStreamChunkType::NotSupported("unsupported".to_string()),
            "not_supported",
            "unsupported",
        ),
    ] {
        let (model, _) = FakeModel::with_items(vec![Ok(vec![AiStreamChunk::Delta(chunk.clone())])]);

        let (result, events) = run_fake_stream(
            "openai",
            "https://api.openai.com/v1/chat/completions",
            model,
            &json!({}),
            None,
        )
        .await;

        assert!(events.is_empty());
        match result {
            Err(Error::Api(AssistantError::APIError { message, metadata, .. })) => {
                assert_eq!(message, expected_message);
                assert_eq!(
                    metadata
                        .as_ref()
                        .and_then(|map| map.get("stream_failure_kind"))
                        .map(String::as_str),
                    Some(expected_kind)
                );
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }
}

#[tokio::test]
async fn stream_returns_aborted_when_abort_flag_is_already_set() {
    let (model, _) = FakeModel::with_items(vec![Ok(vec![AiStreamChunk::Delta(
        LanguageModelStreamChunkType::TextDelta("never emitted".to_string()),
    )])]);
    let (_tx, rx) = tokio::sync::watch::channel(true);

    let (result, events) = run_fake_stream(
        "openai",
        "https://api.openai.com/v1/chat/completions",
        model,
        &json!({}),
        Some(&rx),
    )
    .await;

    assert!(matches!(result, Err(Error::Aborted)));
    assert!(events.is_empty());
}

#[tokio::test]
// 中止监听必须能打断尚无数据的挂起流，而非等待下一块数据。
async fn abort_change_interrupts_a_pending_stream() {
    let model = FakeModel::with_pending_stream();
    let (tx, rx) = tokio::sync::watch::channel(false);
    let options = Value::Null;

    let request = run_fake_stream(
        "openai",
        "https://api.openai.com/v1/chat/completions",
        model,
        &options,
        Some(&rx),
    );
    let trigger_abort = async move {
        tokio::task::yield_now().await;
        tx.send(true)
    };

    let ((result, events), send_result) = tokio::join!(request, trigger_abort);

    assert!(send_result.is_ok());
    assert!(matches!(result, Err(Error::Aborted)));
    assert!(events.is_empty());
}

#[tokio::test]
async fn native_tool_search_request_namespace_calls_resolve_stable_ids_and_reject_unknown() {
    for namespace in ["alpha", "beta", "unknown"] {
        let mut call = tool_call("call_fixture", "read", json!({}));
        call.tool.namespace = Some(namespace.into());
        let (model, _) =
            FakeModel::with_items(vec![Ok(vec![done(AiContent::ToolCall(call), None)])]);
        let specs = ["alpha", "beta"]
            .into_iter()
            .map(|namespace| {
                let id = format!("stable_{namespace}");
                let mut spec = tools::ToolSpec::new(&id, "Read", json!({"type":"object"}));
                spec.identity.name = "read".into();
                spec.identity.namespace = Some(namespace.into());
                (id, spec)
            })
            .collect();
        let mut calls = Vec::new();
        let result = do_stream_request_aisdk_with_model(
            "fixture".into(),
            model,
            "http://fixture/v1/responses".into(),
            false,
            true,
            vec![Message::User("read".into())],
            None,
            &specs,
            None,
            None,
            None,
            &json!({}),
            0,
            None,
            &mut |event| {
                if let StreamEvent::ToolCalls(found) = event {
                    calls.extend(found);
                }
            },
        )
        .await;
        if namespace == "unknown" {
            assert!(result.unwrap_err().to_string().contains("unauthorized tool identity"));
            assert!(calls.is_empty());
        } else {
            assert!(result.is_ok(), "{result:?}");
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].name, format!("stable_{namespace}"));
        }
    }
}

#[tokio::test]
async fn native_tool_search_event_stream_deduplicates_and_preserves_function_namespace() {
    use model_types::ToolCallKind;
    use aisdk::core::{ToolSearchCall, ToolSearchExecution};
    let search = ToolSearchCall {
        id: Some("search-item".into()),
        call_id: Some("search-call".into()),
        status: Some("completed".into()),
        execution: ToolSearchExecution::Client,
        arguments: json!({"query":"read"}),
        encrypted_content: None,
    };
    let mut function = tool_call("function-call", "read", json!({}));
    function.tool.namespace = Some("files".into());
    let (model, _) = FakeModel::with_items(vec![Ok(vec![
        AiStreamChunk::Delta(LanguageModelStreamChunkType::ToolCallAvailable(function.clone())),
        done(AiContent::ToolCall(function), None),
        done(AiContent::ToolSearchCall(search.clone()), None),
        done(AiContent::ToolSearchCall(search), None),
    ])]);
    let mut spec = tools::ToolSpec::new("stable_read", "Read", json!({"type":"object"}));
    spec.identity.name = "read".into();
    spec.identity.namespace = Some("files".into());
    let search_spec = tools::ToolSpec::new("tool_search", "search", json!({"type":"object"}));
    let specs = HashMap::from([(spec.id.clone(), spec), (search_spec.id.clone(), search_spec)]);
    let mut calls = Vec::new();
    let mut finish_reason = None;
    do_stream_request_aisdk_with_model(
        "fixture".into(),
        model,
        "http://fixture/v1/responses".into(),
        false,
        true,
        vec![Message::User("read".into())],
        None,
        &specs,
        None,
        None,
        None,
        &json!({}),
        0,
        None,
        &mut |event| match event {
            StreamEvent::ToolCalls(found) => calls.extend(found),
            StreamEvent::Done { finish_reason: reason, .. } => finish_reason = reason,
            _ => {}
        },
    )
    .await
    .unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].id, "function-call");
    assert_eq!(calls[0].kind, ToolCallKind::Function);
    assert_eq!(calls[0].namespace.as_deref(), Some("files"));
    assert_eq!(calls[0].name, "stable_read");
    assert_eq!(calls[1].id, "search-call");
    assert_eq!(calls[1].kind, ToolCallKind::Search);
    assert_eq!(finish_reason.as_deref(), Some("tool_calls"));
}

#[tokio::test]
async fn native_tool_search_event_stream_rejects_unadvertised_search() {
    let call = aisdk::core::ToolSearchCall {
        id: None,
        call_id: Some("call".into()),
        status: Some("completed".into()),
        execution: aisdk::core::ToolSearchExecution::Client,
        arguments: json!({"query":"read"}),
        encrypted_content: None,
    };
    let (model, _) =
        FakeModel::with_items(vec![Ok(vec![done(AiContent::ToolSearchCall(call), None)])]);
    let (result, events) =
        run_fake_stream("fixture", "http://fixture", model, &json!({}), None).await;
    assert!(result.unwrap_err().to_string().contains("not authorized"));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, StreamEvent::ToolCalls(_) | StreamEvent::Done { .. }))
    );
}
