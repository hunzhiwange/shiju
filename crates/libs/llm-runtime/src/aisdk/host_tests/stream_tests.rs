//! 验证 AI SDK 流事件、用量、结束原因与错误传播。

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use aisdk::Error as AiError;
use aisdk::core::capabilities::ToolCallSupport;
use aisdk::core::language_model::{
    LanguageModel, LanguageModelOptions, LanguageModelResponse,
    LanguageModelResponseContentType as AiContent, LanguageModelStreamChunk as AiStreamChunk,
    LanguageModelStreamChunkType, Usage,
};
use aisdk::core::tools::ToolCallInfo;
use aisdk::core::{AssistantMessage, Message};
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
