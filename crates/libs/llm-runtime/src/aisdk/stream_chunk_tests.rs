//! 验证 AI SDK 流事件、用量、结束原因与错误传播。

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
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
use aisdk::core::tools::{ToolCallInfo, ToolDetails, ToolResultInfo};
use aisdk::core::{AssistantMessage, Message};
use aisdk::providers::OpenAICompatible;
use async_trait::async_trait;
use futures_util::Stream;
use serde_json::{Value, json};

use crate::{Error, StreamEvent};
use message_types::AssistantError;
use tool_types as tools;

use super::{aisdk_stream_chunk_error, normalize_tool_call_id};

#[path = "stream_test_support.rs"]
mod support;
use support::*;

// These fixtures exercise SDK mapping, not application authorization or usage storage.
struct StreamHost;
impl super::super::host::Host for StreamHost {
    fn begin_usage(&self, _: &str, _: &str) -> Option<agent_usage::native::NativeRequest> {
        None
    }
    fn authorize_model_request<'a>(
        &'a self,
        _: &'a str,
    ) -> Pin<Box<dyn std::future::Future<Output = anyhow::Result<bool>> + Send + 'a>> {
        Box::pin(async { Ok(false) })
    }
}

async fn do_stream_request_aisdk_with_model<
    M: aisdk::core::language_model::LanguageModel + ToolCallSupport,
>(
    provider_id: String,
    model: M,
    request_url: String,
    enforce_strict_tool_schema: bool,
    native_tool_search: bool,
    messages: Vec<aisdk::core::Message>,
    wire_messages: Option<Value>,
    tools: &HashMap<String, tools::ToolSpec>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    max_output_tokens: Option<u64>,
    merged_options: &Value,
    retries: u64,
    abort: Option<&tokio::sync::watch::Receiver<bool>>,
    on_event: &mut (dyn FnMut(StreamEvent) + Send),
) -> Result<(), Error> {
    super::do_stream_request_aisdk_with_model(&StreamHost, provider_id, model, request_url, enforce_strict_tool_schema, native_tool_search, messages, wire_messages, tools, temperature, top_p, max_output_tokens, merged_options, retries, abort, on_event).await
}

fn spawn_openai_stream_server(
    request_count: usize,
) -> (String, Arc<Mutex<Vec<Value>>>, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("stream server should bind");
    let address = listener.local_addr().expect("stream server address");
    let requests = Arc::new(Mutex::new(Vec::new()));
    let server_requests = Arc::clone(&requests);
    let server = std::thread::spawn(move || {
        for _ in 0..request_count {
            let (mut stream, _) = listener.accept().expect("stream server should accept request");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let read = stream.read(&mut buffer).expect("stream server should read request");
                assert!(read > 0, "request should not close before its body");
                request.extend_from_slice(&buffer[..read]);
                let Some(headers_end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n")
                else {
                    continue;
                };
                let headers =
                    std::str::from_utf8(&request[..headers_end]).expect("request headers utf8");
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("Content-Length: ")
                            .or_else(|| line.strip_prefix("content-length: "))
                    })
                    .expect("request content length")
                    .parse::<usize>()
                    .expect("numeric content length");
                if request.len() >= headers_end + 4 + content_length {
                    break;
                }
            }

            let headers_end = request.windows(4).position(|bytes| bytes == b"\r\n\r\n").unwrap();
            let request_body: Value = serde_json::from_slice(&request[headers_end + 4..])
                .expect("request body should be json");
            let includes_usage =
                request_body.pointer("/stream_options/include_usage").and_then(Value::as_bool)
                    == Some(true);
            server_requests.lock().expect("request capture lock").push(request_body);

            let usage = if includes_usage {
                concat!(
                    ",\"usage\":{\"prompt_tokens\":11,\"completion_tokens\":22,\"total_tokens\":33,",
                    "\"prompt_tokens_details\":{\"cached_tokens\":3},",
                    "\"completion_tokens_details\":{\"reasoning_tokens\":4}}"
                )
            } else {
                ""
            };
            let body = format!(
                "data: {{\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"stop\"}}]{usage}}}\n\ndata: [DONE]\n\n"
            );
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("stream server should respond");
        }
    });

    (format!("http://{address}/v1"), requests, server)
}

async fn run_openai_compatible_stream(
    provider_id: &str,
    base_url: &str,
) -> (Result<(), Error>, Vec<StreamEvent>) {
    let model = OpenAICompatible::<DynamicModel>::builder()
        .provider_name(provider_id)
        .api_key("test-token")
        .model_name("test-model")
        .base_url(base_url)
        .build()
        .expect("openai compatible model");
    let mut events = Vec::new();
    let result = do_stream_request_aisdk_with_model(
        provider_id.to_string(),
        model,
        format!("{base_url}/chat/completions"),
        false,
        false,
        vec![Message::User("hello".into())],
        None,
        &HashMap::new(),
        None,
        None,
        None,
        &Value::Null,
        0,
        None,
        &mut |event| events.push(event),
    )
    .await;
    (result, events)
}

#[tokio::test]
async fn token_plan_stream_requests_and_emits_usage() {
    let (base_url, requests, server) = spawn_openai_stream_server(2);

    let (token_plan_result, token_plan_events) =
        run_openai_compatible_stream("token-plan", &base_url).await;
    let (other_result, other_events) = run_openai_compatible_stream("openai", &base_url).await;
    server.join().expect("stream server should finish");

    assert!(token_plan_result.is_ok());
    assert!(other_result.is_ok());
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].pointer("/stream_options/include_usage"), Some(&json!(true)));
    assert_eq!(requests[1].get("stream_options"), None);
    assert!(matches!(
        token_plan_events.as_slice(),
        [StreamEvent::Done { usage, .. }]
            if usage.input_tokens == 11
                && usage.output_tokens == 22
                && usage.cached_tokens == 3
                && usage.reasoning_tokens == 4
    ));
    assert!(matches!(
        other_events.as_slice(),
        [StreamEvent::Done { usage, .. }]
            if usage.input_tokens == 0
                && usage.output_tokens == 0
                && usage.cached_tokens == 0
                && usage.reasoning_tokens == 0
    ));
}

#[test]
fn stream_chunk_error_builds_non_retryable_api_error_metadata() {
    let error = aisdk_stream_chunk_error("failed", "bad chunk".to_string());

    match error {
        AssistantError::APIError {
            message,
            status_code,
            is_retryable,
            response_body,
            metadata,
            ..
        } => {
            assert_eq!(message, "bad chunk");
            assert_eq!(status_code, None);
            assert!(!is_retryable);
            assert_eq!(response_body.as_deref(), Some("bad chunk"));
            let metadata = metadata.expect("metadata");
            assert_eq!(metadata.get("source").map(String::as_str), Some("aisdk"));
            assert_eq!(metadata.get("raw_error").map(String::as_str), Some("bad chunk"));
            assert_eq!(metadata.get("stream_failure_kind").map(String::as_str), Some("failed"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn synthetic_openai_tool_call_ids_are_namespaced_per_request() {
    assert_eq!(normalize_tool_call_id("provider-call-1", "request-a"), "provider-call-1");
    assert_eq!(
        normalize_tool_call_id("openai-tool-call-custom", "request-a"),
        "openai-tool-call-custom"
    );
    assert_eq!(
        normalize_tool_call_id("openai-tool-call-0", "request-a"),
        "openai-tool-call-request-a-0"
    );
    assert_eq!(
        normalize_tool_call_id("openai-tool-call-0", "request-b"),
        "openai-tool-call-request-b-0"
    );
}

#[tokio::test]
async fn stream_sends_deltas_tool_calls_usage_and_clamped_request_options() {
    let available = tool_call("call-1", "lookup", json!({ "q": "rust" }));
    let duplicate_done = available.clone();
    let (model, captured) = FakeModel::with_items(vec![Ok(vec![
        AiStreamChunk::Delta(LanguageModelStreamChunkType::TextStart),
        AiStreamChunk::Delta(LanguageModelStreamChunkType::TextDelta("hello".to_string())),
        AiStreamChunk::Delta(LanguageModelStreamChunkType::TextEnd),
        AiStreamChunk::Delta(LanguageModelStreamChunkType::ReasoningStart),
        AiStreamChunk::Delta(LanguageModelStreamChunkType::ReasoningDelta("why".to_string())),
        AiStreamChunk::Delta(LanguageModelStreamChunkType::ReasoningEnd),
        AiStreamChunk::Delta(LanguageModelStreamChunkType::ToolCallStart(ToolDetails {
            id: "call-1".to_string(),
            name: "lookup".to_string(),
            namespace: None,
        })),
        AiStreamChunk::Delta(LanguageModelStreamChunkType::ToolCallDelta {
            id: "call-1".to_string(),
            delta: "{}".to_string(),
        }),
        AiStreamChunk::Delta(LanguageModelStreamChunkType::ToolCallAvailable(available)),
        AiStreamChunk::Delta(LanguageModelStreamChunkType::ToolCallEnd(ToolResultInfo::new(
            "lookup",
        ))),
        done(
            AiContent::Text("ignored because text delta was already sent".to_string()),
            Some(Usage {
                input_tokens: Some(11),
                output_tokens: Some(22),
                cached_tokens: Some(3),
                reasoning_tokens: Some(4),
            }),
        ),
        done(AiContent::ToolCall(duplicate_done), None),
    ])]);

    let tools = HashMap::from([(
        "lookup".to_string(),
        tools::ToolSpec::new(
            "lookup",
            "Lookup docs",
            json!({
                "type": "object",
                "properties": {
                    "q": { "type": "string" }
                }
            }),
        ),
    )]);
    let mut events = Vec::new();
    let options = json!({
        "presence_penalty": 0.25,
        "frequency_penalty": 0.5,
        "seed": u64::MAX,
        "top_k": u64::MAX,
        "reasoning_effort": "high",
        "stop_sequences": [" END ", ""]
    });

    let result = do_stream_request_aisdk_with_model(
        "openai".to_string(),
        model,
        "https://api.openai.com/v1/chat/completions".to_string(),
        true,
        false,
        vec![Message::User("hello".into())],
        None,
        &tools,
        Some(2.0),
        Some(-1.0),
        Some(u64::MAX),
        &options,
        u64::MAX,
        None,
        &mut |event| events.push(event),
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(events.len(), 4);
    assert!(matches!(&events[0], StreamEvent::Delta(text) if text == "hello"));
    assert!(matches!(&events[1], StreamEvent::ReasoningDelta(text) if text == "why"));
    assert!(matches!(
        &events[2],
        StreamEvent::ToolCalls(calls)
            if calls.len() == 1
                && calls[0].id == "call-1"
                && calls[0].name == "lookup"
                && calls[0].arguments == "{\"q\":\"rust\"}"
    ));
    assert!(matches!(
        &events[3],
        StreamEvent::Done { finish_reason, usage }
            if finish_reason.as_deref() == Some("tool_calls")
                && usage.input_tokens == 11
                && usage.output_tokens == 22
                && usage.cached_tokens == 3
                && usage.reasoning_tokens == 4
    ));

    let captured =
        captured.lock().expect("capture lock").clone().expect("stream options should be captured");
    assert_eq!(captured.temperature, Some(100));
    assert_eq!(captured.top_p, Some(0));
    assert_eq!(captured.max_output_tokens, Some(u32::MAX));
    assert_eq!(captured.max_retries, Some(u32::MAX));
    assert_eq!(captured.presence_penalty, Some(0.25));
    assert_eq!(captured.frequency_penalty, Some(0.5));
    assert_eq!(captured.seed, Some(u32::MAX));
    assert_eq!(captured.top_k, Some(u32::MAX));
    assert_eq!(captured.stop_sequences, Some(vec!["END".to_string()]));
    assert!(matches!(captured.reasoning_effort, Some(ReasoningEffort::High)));
    assert!(captured.body.is_none());
    assert_eq!(captured.messages().len(), 1);
    assert!(format!("{captured:?}").contains("lookup"));
}

#[tokio::test]
// 结束帧仅补发此前未流出的内容，避免正文或推理重复。
async fn done_content_emits_when_no_prior_delta_and_ignores_other_done_content() {
    let (model, _) = FakeModel::with_items(vec![Ok(vec![
        done(AiContent::Text("final text".to_string()), None),
        done(
            AiContent::Reasoning { content: "because".to_string(), extensions: Default::default() },
            None,
        ),
        done(AiContent::NotSupported("nope".to_string()), None),
    ])]);

    let (result, events) = run_fake_stream(
        "openai",
        "https://api.openai.com/v1/chat/completions",
        model,
        &Value::Null,
        None,
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(events.len(), 3);
    assert!(matches!(&events[0], StreamEvent::Delta(text) if text == "final text"));
    assert!(matches!(&events[1], StreamEvent::ReasoningDelta(text) if text == "because"));
    assert!(matches!(
        &events[2],
        StreamEvent::Done { finish_reason, usage }
            if finish_reason.as_deref() == Some("stop")
                && usage.input_tokens == 0
                && usage.output_tokens == 0
    ));
}

#[tokio::test]
async fn empty_deltas_do_not_suppress_non_empty_done_content() {
    let (model, _) = FakeModel::with_items(vec![Ok(vec![
        AiStreamChunk::Delta(LanguageModelStreamChunkType::TextDelta(String::new())),
        AiStreamChunk::Delta(LanguageModelStreamChunkType::ReasoningDelta(String::new())),
        done(AiContent::Text("final text".to_string()), None),
        done(
            AiContent::Reasoning {
                content: "final reasoning".to_string(),
                extensions: Default::default(),
            },
            None,
        ),
    ])]);

    let (result, events) = run_fake_stream(
        "openai",
        "https://api.openai.com/v1/chat/completions",
        model,
        &Value::Null,
        None,
    )
    .await;

    assert!(result.is_ok());
    assert!(matches!(&events[0], StreamEvent::Delta(text) if text == "final text"));
    assert!(matches!(
        &events[1],
        StreamEvent::ReasoningDelta(text) if text == "final reasoning"
    ));
    assert!(matches!(&events[2], StreamEvent::Done { .. }));
}

#[tokio::test]
async fn prior_reasoning_delta_suppresses_done_reasoning_content() {
    let (model, _) = FakeModel::with_items(vec![Ok(vec![
        AiStreamChunk::Delta(LanguageModelStreamChunkType::ReasoningDelta("partial".to_string())),
        done(
            AiContent::Reasoning {
                content: "complete".to_string(),
                extensions: Default::default(),
            },
            None,
        ),
    ])]);

    let (result, events) = run_fake_stream(
        "openai",
        "https://api.openai.com/v1/chat/completions",
        model,
        &Value::Null,
        None,
    )
    .await;

    assert!(result.is_ok());
    assert!(matches!(
        events.as_slice(),
        [StreamEvent::ReasoningDelta(text), StreamEvent::Done { .. }] if text == "partial"
    ));
}

#[tokio::test]
async fn done_tool_call_is_emitted_and_sets_tool_call_finish_reason() {
    let (model, _) = FakeModel::with_items(vec![Ok(vec![done(
        AiContent::ToolCall(tool_call("call-done", "search", json!({ "q": "rust" }))),
        None,
    )])]);

    let (result, events) = run_fake_stream(
        "openai",
        "https://api.openai.com/v1/chat/completions",
        model,
        &Value::Null,
        None,
    )
    .await;

    assert!(result.is_ok());
    assert!(matches!(
        &events[0],
        StreamEvent::ToolCalls(calls)
            if calls.len() == 1
                && calls[0].id == "call-done"
                && calls[0].name == "search"
                && calls[0].arguments == "{\"q\":\"rust\"}"
    ));
    assert!(matches!(
        &events[1],
        StreamEvent::Done { finish_reason, .. }
            if finish_reason.as_deref() == Some("tool_calls")
    ));
}

#[tokio::test]
async fn duplicate_available_tool_calls_and_empty_done_content_emit_once() {
    let available = tool_call("call-available", "search", json!({ "q": "rust" }));
    let (model, _) = FakeModel::with_items(vec![Ok(vec![
        AiStreamChunk::Delta(LanguageModelStreamChunkType::ToolCallAvailable(available.clone())),
        AiStreamChunk::Delta(LanguageModelStreamChunkType::ToolCallAvailable(available)),
        done(AiContent::Text(String::new()), None),
        done(AiContent::Reasoning { content: String::new(), extensions: Default::default() }, None),
    ])]);

    let (result, events) = run_fake_stream(
        "openai",
        "https://api.openai.com/v1/chat/completions",
        model,
        &Value::Null,
        None,
    )
    .await;

    assert!(result.is_ok());
    assert!(matches!(
        events.as_slice(),
        [StreamEvent::ToolCalls(calls), StreamEvent::Done { finish_reason, .. }]
            if calls.len() == 1
                && calls[0].id == "call-available"
                && finish_reason.as_deref() == Some("tool_calls")
    ));
}

#[tokio::test]
async fn latest_reported_usage_is_used_for_done_event() {
    let (model, _) = FakeModel::with_items(vec![Ok(vec![
        done(
            AiContent::NotSupported("first".to_string()),
            Some(Usage {
                input_tokens: Some(1),
                output_tokens: Some(2),
                cached_tokens: Some(3),
                reasoning_tokens: Some(4),
            }),
        ),
        done(
            AiContent::NotSupported("second".to_string()),
            Some(Usage {
                input_tokens: Some(10),
                output_tokens: Some(20),
                cached_tokens: Some(30),
                reasoning_tokens: Some(40),
            }),
        ),
    ])]);

    let (result, events) = run_fake_stream(
        "openai",
        "https://api.openai.com/v1/chat/completions",
        model,
        &Value::Null,
        None,
    )
    .await;

    assert!(result.is_ok());
    assert!(matches!(
        events.as_slice(),
        [StreamEvent::Done { usage, .. }]
            if usage.input_tokens == 10
                && usage.output_tokens == 20
                && usage.cached_tokens == 30
                && usage.reasoning_tokens == 40
    ));
}
