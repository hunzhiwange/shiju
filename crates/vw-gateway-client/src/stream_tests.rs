use serde_json::json;

#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::net::{SocketAddr, TcpListener};
#[cfg(not(target_arch = "wasm32"))]
use std::thread::JoinHandle;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

use super::{
    GatewayChatStreamEvent, GatewayChatStreamRequest, GatewayChatUsage, GatewayStreamAttemptResult,
    GatewayStreamEvent, GatewayStreamResume, GatewayTypedChatStreamEvent,
};
#[cfg(not(target_arch = "wasm32"))]
use vw_api_types::id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, TenantScope};

#[test]
fn stream_http_failure_message_humanizes_payload_too_large() {
    let message = super::stream_http_failure_message(
        "413 Payload Too Large",
        "Failed to buffer the request body: length limit exceeded",
    );
    assert!(message.contains("请求内容过大"));
    assert!(!message.contains("length limit exceeded"));
}

#[test]
fn stream_http_failure_message_keeps_other_http_errors() {
    let message = super::stream_http_failure_message("500 Internal Server Error", "down");
    assert_eq!(message, "gateway stream failed: 500 Internal Server Error down");
}

#[cfg(not(target_arch = "wasm32"))]
struct EnabledStreamTraceSubscriber;

#[cfg(not(target_arch = "wasm32"))]
impl tracing::Subscriber for EnabledStreamTraceSubscriber {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        metadata.target() == "vw_gateway_client"
    }

    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::Id {
        tracing::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::Id, _follows: &tracing::Id) {}

    fn event(&self, _event: &tracing::Event<'_>) {}

    fn enter(&self, _span: &tracing::Id) {}

    fn exit(&self, _span: &tracing::Id) {}

    fn register_callsite(
        &self,
        metadata: &'static tracing::Metadata<'static>,
    ) -> tracing::subscriber::Interest {
        if metadata.target() == "vw_gateway_client" {
            tracing::subscriber::Interest::always()
        } else {
            tracing::subscriber::Interest::never()
        }
    }

    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(tracing::metadata::LevelFilter::TRACE)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn enable_stream_trace() -> tracing::dispatcher::DefaultGuard {
    tracing::subscriber::set_default(EnabledStreamTraceSubscriber)
}

#[cfg(not(target_arch = "wasm32"))]
struct StreamServer {
    addr: SocketAddr,
    handle: Option<JoinHandle<String>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl StreamServer {
    fn respond(status: &str, body: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let addr = listener.local_addr().expect("test server addr should be available");
        let status = status.to_string();
        let body = body.as_bytes().to_vec();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("test connection should be accepted");
            let mut request = [0_u8; 4096];
            let bytes_read = stream.read(&mut request).expect("request should be readable");
            let request = String::from_utf8_lossy(&request[..bytes_read]).to_string();
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("headers should be written");
            stream.write_all(&body).expect("body should be written");
            request
        });
        Self { addr, handle: Some(handle) }
    }

    fn respond_without_content_length_until_client_closes(
        status: &str,
        body: &str,
        read_timeout: Duration,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let addr = listener.local_addr().expect("test server addr should be available");
        let status = status.to_string();
        let body = body.as_bytes().to_vec();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("test connection should be accepted");
            let mut request = [0_u8; 4096];
            let bytes_read = stream.read(&mut request).expect("request should be readable");
            let request = String::from_utf8_lossy(&request[..bytes_read]).to_string();
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\n\r\n"
            );
            stream.write_all(response.as_bytes()).expect("headers should be written");
            stream.write_all(&body).expect("body should be written");
            stream.flush().expect("body should be flushed");
            stream.set_read_timeout(Some(read_timeout)).expect("read timeout should be set");
            let mut probe = [0_u8; 1];
            let _ = stream.read(&mut probe);
            request
        });
        Self { addr, handle: Some(handle) }
    }

    fn endpoint(&self) -> crate::endpoint::GatewayEndpoint {
        crate::endpoint::GatewayEndpoint::new("127.0.0.1", self.addr.port())
    }

    fn join(&mut self) -> String {
        self.handle.take().expect("server should not be joined").join().unwrap()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for StreamServer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn assert_raw_tenant_headers(request: &str, platform_id: &str, company_id: &str) {
    assert_eq!(raw_header(request, PLATFORM_ID_HEADER).as_deref(), Some(platform_id));
    assert_eq!(raw_header(request, COMPANY_ID_HEADER).as_deref(), Some(company_id));
}

#[cfg(not(target_arch = "wasm32"))]
fn raw_header(request: &str, name: &str) -> Option<String> {
    request.lines().filter_map(|line| line.split_once(':')).find_map(|(header, value)| {
        header.eq_ignore_ascii_case(name).then(|| value.trim().to_string())
    })
}

#[test]
fn usage_serializes_defaults_reasoning_tokens() {
    let usage: GatewayChatUsage =
        serde_json::from_value(json!({"input_tokens": 1, "output_tokens": 2, "cached_tokens": 3}))
            .unwrap();

    assert_eq!(
        usage,
        GatewayChatUsage {
            input_tokens: 1,
            output_tokens: 2,
            cached_tokens: 3,
            reasoning_tokens: 0,
        }
    );
}

#[test]
fn parse_stream_event_maps_known_gateway_events() {
    assert_eq!(
        super::parse_stream_event(json!({"type": "chat.delta", "delta": "hello"})),
        GatewayChatStreamEvent::Delta("hello".to_string())
    );
    assert_eq!(
        super::parse_stream_event(json!({"type": "chat.delta"})),
        GatewayChatStreamEvent::Delta(String::new())
    );
    assert_eq!(
        super::parse_stream_event(json!({
            "type": "chat.done",
            "finish_reason": "stop",
            "usage": {"input_tokens": 1, "output_tokens": 2, "cached_tokens": 3},
            "message_id": "msg-1",
            "parent_message_id": "msg-0"
        })),
        GatewayChatStreamEvent::Done {
            finish_reason: Some("stop".to_string()),
            usage: Some(json!({"input_tokens": 1, "output_tokens": 2, "cached_tokens": 3})),
            message_id: Some("msg-1".to_string()),
            parent_message_id: Some("msg-0".to_string()),
        }
    );
    assert_eq!(
        super::parse_stream_event(json!({"type": "chat.error", "error": "failed"})),
        GatewayChatStreamEvent::Error("failed".to_string())
    );
    assert_eq!(
        super::parse_stream_event(json!({"type": "chat.error"})),
        GatewayChatStreamEvent::Error("gateway stream failed".to_string())
    );
}

#[test]
fn parse_stream_event_preserves_unknown_payload() {
    let payload = json!({"type": "chat.custom", "value": 1});

    assert_eq!(super::parse_stream_event(payload.clone()), GatewayChatStreamEvent::Other(payload));
}

#[test]
fn take_next_sse_event_handles_lf_and_crlf_separators() {
    let mut lf = "data: one\n\ndata: two".to_string();
    assert_eq!(super::take_next_sse_event(&mut lf).as_deref(), Some("data: one"));
    assert_eq!(lf, "data: two");

    let mut crlf = "data: one\r\n\r\ndata: two".to_string();
    assert_eq!(super::take_next_sse_event(&mut crlf).as_deref(), Some("data: one"));
    assert_eq!(crlf, "data: two");

    assert_eq!(super::take_next_sse_event(&mut "data: partial".to_string()), None);
}

#[test]
fn event_name_returns_stable_names() {
    assert_eq!(super::event_name(&GatewayChatStreamEvent::Delta(String::new())), "chat.delta");
    assert_eq!(
        super::event_name(&GatewayChatStreamEvent::Done {
            finish_reason: None,
            usage: None,
            message_id: None,
            parent_message_id: None,
        }),
        "chat.done"
    );
    assert_eq!(super::event_name(&GatewayChatStreamEvent::Error(String::new())), "chat.error");
    assert_eq!(super::event_name(&GatewayChatStreamEvent::Other(json!({}))), "other");
}

#[test]
fn terminal_stream_event_accepts_done_and_error_only() {
    let done = GatewayChatStreamEvent::Done {
        finish_reason: None,
        usage: None,
        message_id: None,
        parent_message_id: None,
    };

    assert!(super::is_terminal_stream_event(&done));
    assert!(super::is_terminal_stream_event(&GatewayChatStreamEvent::Error("failed".to_string())));
    assert!(!super::is_terminal_stream_event(&GatewayChatStreamEvent::Delta(String::new())));
    assert!(!super::is_terminal_stream_event(&GatewayChatStreamEvent::Other(json!({}))));
}

#[test]
fn normalize_chat_stream_event_maps_direct_events() {
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Delta("delta".to_string())),
        GatewayTypedChatStreamEvent::Delta("delta".to_string())
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Error("bad".to_string())),
        GatewayTypedChatStreamEvent::Error("bad".to_string())
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Done {
            finish_reason: Some("stop".to_string()),
            usage: Some(json!({
                "input_tokens": 1,
                "output_tokens": 2,
                "cached_tokens": 3,
                "reasoning_tokens": 4
            })),
            message_id: Some("msg".to_string()),
            parent_message_id: Some("parent".to_string()),
        }),
        GatewayTypedChatStreamEvent::Done {
            finish_reason: Some("stop".to_string()),
            usage: Some(GatewayChatUsage {
                input_tokens: 1,
                output_tokens: 2,
                cached_tokens: 3,
                reasoning_tokens: 4,
            }),
            message_id: Some("msg".to_string()),
            parent_message_id: Some("parent".to_string()),
        }
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Done {
            finish_reason: None,
            usage: Some(json!({"bad": true})),
            message_id: None,
            parent_message_id: None,
        }),
        GatewayTypedChatStreamEvent::Done {
            finish_reason: None,
            usage: None,
            message_id: None,
            parent_message_id: None,
        }
    );
}

#[test]
fn normalize_chat_stream_event_maps_structured_other_events() {
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.step_start",
            "step_index": 2,
            "created_ms": 100,
            "model": "m"
        }))),
        GatewayTypedChatStreamEvent::StepStart(super::GatewayChatStepStartEvent {
            step_index: 2,
            created_ms: 100,
            model: Some("m".to_string()),
        })
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.step_finish",
            "step_index": 3,
            "finished_ms": 200,
            "usage": {"input_tokens": 4, "output_tokens": 5, "cached_tokens": 6},
            "finish_reason": "tool",
            "model": "m2"
        }))),
        GatewayTypedChatStreamEvent::StepFinish(super::GatewayChatStepFinishEvent {
            step_index: 3,
            finished_ms: 200,
            usage: GatewayChatUsage {
                input_tokens: 4,
                output_tokens: 5,
                cached_tokens: 6,
                reasoning_tokens: 0,
            },
            finish_reason: Some("tool".to_string()),
            model: Some("m2".to_string()),
        })
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.post_tool_round",
            "step_index": 4
        }))),
        GatewayTypedChatStreamEvent::PostToolRound(super::GatewayChatPostToolRoundEvent {
            step_index: 4,
        })
    );
}

#[test]
fn normalize_chat_stream_event_maps_session_side_effect_events() {
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.context_loaded",
            "paths": ["/work/AGENTS.md", "  ", 42, "/work/CONTEXT.md"]
        }))),
        GatewayTypedChatStreamEvent::ContextLoaded {
            paths: vec!["/work/AGENTS.md".to_string(), "/work/CONTEXT.md".to_string()],
        }
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.todo_updated",
            "session_id": "s1"
        }))),
        GatewayTypedChatStreamEvent::TodoUpdated { session_id: Some("s1".to_string()) }
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.question_raised",
            "session_id": "s2"
        }))),
        GatewayTypedChatStreamEvent::QuestionRaised { session_id: Some("s2".to_string()) }
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.question_resolved",
            "session_id": "s3"
        }))),
        GatewayTypedChatStreamEvent::QuestionResolved { session_id: Some("s3".to_string()) }
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.usage_updated",
            "session_id": "s4",
            "usage": {"input_tokens": 7, "output_tokens": 8, "cached_tokens": 9}
        }))),
        GatewayTypedChatStreamEvent::UsageUpdated {
            session_id: Some("s4".to_string()),
            usage: GatewayChatUsage {
                input_tokens: 7,
                output_tokens: 8,
                cached_tokens: 9,
                reasoning_tokens: 0,
            },
        }
    );
}

#[test]
fn normalize_chat_stream_event_maps_title_session_and_unknown_events() {
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.title_updated",
            "session_id": "s5",
            "title": "New title"
        }))),
        GatewayTypedChatStreamEvent::TitleUpdated {
            session_id: Some("s5".to_string()),
            title: "New title".to_string(),
        }
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.session_updated",
            "session_id": "s6",
            "session": {"title": "Session title"}
        }))),
        GatewayTypedChatStreamEvent::SessionUpdated {
            session_id: Some("s6".to_string()),
            title: Some("Session title".to_string()),
        }
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.future"
        }))),
        GatewayTypedChatStreamEvent::Unknown { event_type: Some("chat.future".to_string()) }
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({}))),
        GatewayTypedChatStreamEvent::Unknown { event_type: None }
    );
}

#[test]
fn structured_event_parsers_default_invalid_fields() {
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.step_start",
            "step_index": u64::from(u32::MAX) + 1,
            "created_ms": "bad",
            "model": 1
        }))),
        GatewayTypedChatStreamEvent::StepStart(super::GatewayChatStepStartEvent {
            step_index: 0,
            created_ms: 0,
            model: None,
        })
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.step_finish",
            "step_index": u64::from(u32::MAX) + 1,
            "finished_ms": "bad",
            "usage": {"bad": true},
            "finish_reason": 1,
            "model": 2
        }))),
        GatewayTypedChatStreamEvent::StepFinish(super::GatewayChatStepFinishEvent {
            step_index: 0,
            finished_ms: 0,
            usage: GatewayChatUsage::default(),
            finish_reason: None,
            model: None,
        })
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.post_tool_round",
            "step_index": u64::from(u32::MAX) + 1
        }))),
        GatewayTypedChatStreamEvent::PostToolRound(super::GatewayChatPostToolRoundEvent {
            step_index: 0,
        })
    );
}

#[test]
fn normalize_chat_stream_event_defaults_missing_usage_and_optional_titles() {
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.usage_updated"
        }))),
        GatewayTypedChatStreamEvent::UsageUpdated {
            session_id: None,
            usage: GatewayChatUsage::default(),
        }
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.usage_updated",
            "usage": {"bad": true}
        }))),
        GatewayTypedChatStreamEvent::UsageUpdated {
            session_id: None,
            usage: GatewayChatUsage::default(),
        }
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.title_updated"
        }))),
        GatewayTypedChatStreamEvent::TitleUpdated { session_id: None, title: String::new() }
    );
    assert_eq!(
        super::normalize_chat_stream_event(GatewayChatStreamEvent::Other(json!({
            "type": "chat.session_updated",
            "session_id": "s7"
        }))),
        GatewayTypedChatStreamEvent::SessionUpdated {
            session_id: Some("s7".to_string()),
            title: None,
        }
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn stream_chat_reads_sse_events_and_stops_when_callback_returns_false() {
    let mut server = StreamServer::respond(
        "200 OK",
        "data: {\"type\":\"chat.delta\",\"delta\":\"a\"}\n\n\
         data: {\"type\":\"chat.done\",\"finish_reason\":\"stop\"}\n\n",
    );
    let endpoint = server
        .endpoint()
        .with_tenant_scope(TenantScope::new("platform-chat", "company-chat").expect("tenant"));
    let body = GatewayChatStreamRequest::default();
    let mut events = Vec::new();

    super::stream_chat(&endpoint, Some(" /tmp/project "), &body, |event| {
        events.push(event);
        false
    })
    .await
    .unwrap();
    let request = server.join();

    assert_eq!(events, vec![GatewayChatStreamEvent::Delta("a".to_string())]);
    assert!(request.starts_with("POST /v1/chat/stream?directory="));
    assert_raw_tenant_headers(&request, "platform-chat", "company-chat");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn stream_chat_skips_empty_payload_frames_and_collects_multiline_data() {
    let mut server = StreamServer::respond(
        "200 OK",
        "event: ping\n\n\
         data: {\"type\":\"chat.delta\",\n\
         data: \"delta\":\"joined\"}\n\n",
    );
    let endpoint = server.endpoint();
    let body = GatewayChatStreamRequest::default();
    let mut events = Vec::new();

    super::stream_chat(&endpoint, Some(""), &body, |event| {
        events.push(event);
        true
    })
    .await
    .unwrap();
    let request = server.join();

    assert_eq!(events, vec![GatewayChatStreamEvent::Delta("joined".to_string())]);
    assert!(request.starts_with("POST /v1/chat/stream HTTP/1.1"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn stream_chat_reports_http_and_json_errors() {
    let _trace = enable_stream_trace();
    let mut http_server = StreamServer::respond("500 Internal Server Error", " down ");
    let http_error = super::stream_chat(
        &http_server.endpoint(),
        None,
        &GatewayChatStreamRequest::default(),
        |_| true,
    )
    .await
    .unwrap_err();
    let _ = http_server.join();
    assert!(http_error.contains("gateway stream failed: 500 Internal Server Error down"));

    let mut json_server = StreamServer::respond("200 OK", "data: not-json\n\n");
    let json_error = super::stream_chat(
        &json_server.endpoint(),
        None,
        &GatewayChatStreamRequest::default(),
        |_| true,
    )
    .await
    .unwrap_err();
    let _ = json_server.join();
    assert!(json_error.contains("expected ident") || json_error.contains("expected value"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn stream_chat_stops_on_terminal_event_without_waiting_for_socket_close() {
    let _trace = enable_stream_trace();
    let mut server = StreamServer::respond_without_content_length_until_client_closes(
        "200 OK",
        "data: {\"type\":\"chat.delta\",\"delta\":\"a\"}\n\n\
         data: {\"type\":\"chat.done\",\"finish_reason\":\"stop\"}\n\n",
        Duration::from_secs(2),
    );
    let endpoint = server.endpoint();
    let body = GatewayChatStreamRequest::default();
    let mut events = Vec::new();
    let started = Instant::now();

    super::stream_chat(&endpoint, None, &body, |event| {
        events.push(event);
        true
    })
    .await
    .unwrap();
    let elapsed = started.elapsed();
    let request = server.join();

    assert!(elapsed < Duration::from_secs(1), "stream should stop on chat.done: {elapsed:?}");
    assert_eq!(
        events,
        vec![
            GatewayChatStreamEvent::Delta("a".to_string()),
            GatewayChatStreamEvent::Done {
                finish_reason: Some("stop".to_string()),
                usage: None,
                message_id: None,
                parent_message_id: None,
            },
        ]
    );
    assert_raw_tenant_headers(&request, "10000", "100000000");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn stream_chat_completes_when_server_closes_stream() {
    let _trace = enable_stream_trace();
    let mut server =
        StreamServer::respond("200 OK", "data: {\"type\":\"chat.delta\",\"delta\":\"a\"}\n\n");
    let endpoint = server.endpoint();
    let body = GatewayChatStreamRequest::default();
    let mut events = Vec::new();

    super::stream_chat(&endpoint, None, &body, |event| {
        events.push(event);
        true
    })
    .await
    .unwrap();
    let _ = server.join();

    assert_eq!(events, vec![GatewayChatStreamEvent::Delta("a".to_string())]);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn resumable_stream_reports_incomplete_eof_and_sends_resume_headers() {
    let mut server = StreamServer::respond(
        "200 OK",
        "id: 42\ndata: {\"type\":\"chat.delta\",\"delta\":\"a\"}\n\n",
    );
    let endpoint = server.endpoint();
    let resume =
        GatewayStreamResume { request_id: "chat-request-1".to_string(), last_event_id: Some(41) };
    let mut events = Vec::new();

    let result = super::stream_chat_resumable(
        &endpoint,
        None,
        &GatewayChatStreamRequest::default(),
        &resume,
        |event| {
            events.push(event);
            true
        },
    )
    .await
    .expect("resumable stream attempt");
    let request = server.join();

    assert_eq!(result, GatewayStreamAttemptResult::Incomplete);
    assert_eq!(
        events,
        vec![GatewayStreamEvent { id: 42, event: GatewayChatStreamEvent::Delta("a".to_string()) }]
    );
    assert_eq!(raw_header(&request, "x-vw-stream-request-id").as_deref(), Some("chat-request-1"));
    assert_eq!(raw_header(&request, "last-event-id").as_deref(), Some("41"));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn resumable_stream_distinguishes_terminal_event_and_caller_stop() {
    let mut terminal_server = StreamServer::respond(
        "200 OK",
        "id: 7\ndata: {\"type\":\"chat.done\",\"finish_reason\":\"stop\"}\n\n",
    );
    let terminal_resume = GatewayStreamResume {
        request_id: "chat-request-terminal".to_string(),
        last_event_id: None,
    };
    let mut terminal_events = Vec::new();

    let terminal = super::stream_chat_resumable(
        &terminal_server.endpoint(),
        None,
        &GatewayChatStreamRequest::default(),
        &terminal_resume,
        |event| {
            terminal_events.push(event);
            true
        },
    )
    .await
    .expect("terminal resumable stream");
    let terminal_request = terminal_server.join();

    assert_eq!(terminal, GatewayStreamAttemptResult::Terminated);
    assert_eq!(terminal_events.len(), 1);
    assert_eq!(terminal_events[0].id, 7);
    assert_eq!(
        raw_header(&terminal_request, "x-vw-stream-request-id").as_deref(),
        Some("chat-request-terminal")
    );
    assert_eq!(raw_header(&terminal_request, "last-event-id"), None);

    let mut stopped_server = StreamServer::respond(
        "200 OK",
        "id: 8\ndata: {\"type\":\"chat.delta\",\"delta\":\"ignored\"}\n\n",
    );
    let stopped_resume = GatewayStreamResume {
        request_id: "chat-request-stopped".to_string(),
        last_event_id: Some(7),
    };

    let stopped = super::stream_chat_resumable(
        &stopped_server.endpoint(),
        None,
        &GatewayChatStreamRequest::default(),
        &stopped_resume,
        |_| false,
    )
    .await
    .expect("caller-stopped resumable stream");
    let _ = stopped_server.join();

    assert_eq!(stopped, GatewayStreamAttemptResult::CallerStopped);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn stream_chat_with_idle_timeout_delivers_events_within_window() {
    let mut server = StreamServer::respond(
        "200 OK",
        "data: {\"type\":\"chat.delta\",\"delta\":\"a\"}\n\n\
         data: {\"type\":\"chat.done\",\"finish_reason\":\"stop\"}\n\n",
    );
    let endpoint = server.endpoint();
    let body = GatewayChatStreamRequest::default();
    let mut events = Vec::new();

    super::stream_chat_with_idle_timeout(&endpoint, None, &body, Duration::from_secs(5), |event| {
        events.push(event);
        true
    })
    .await
    .unwrap();
    let _ = server.join();

    assert_eq!(
        events,
        vec![
            GatewayChatStreamEvent::Delta("a".to_string()),
            GatewayChatStreamEvent::Done {
                finish_reason: Some("stop".to_string()),
                usage: None,
                message_id: None,
                parent_message_id: None,
            },
        ]
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn stream_chat_blocking_with_idle_timeout_reports_response_gap() {
    let mut server = StreamServer::respond_without_content_length_until_client_closes(
        "200 OK",
        "",
        Duration::from_secs(2),
    );
    let endpoint = server.endpoint();
    let body = GatewayChatStreamRequest::default();
    let started = Instant::now();

    let error = super::stream_chat_blocking_with_idle_timeout(
        &endpoint,
        None,
        &body,
        Duration::from_millis(50),
        |_| true,
    )
    .unwrap_err();
    let elapsed = started.elapsed();
    let request = server.join();

    assert!(elapsed < Duration::from_secs(1), "idle timeout should be prompt: {elapsed:?}");
    assert!(error.contains("idle timeout"));
    assert!(error.contains("without response"));
    assert_raw_tenant_headers(&request, "10000", "100000000");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn stream_chat_with_idle_timeout_reports_response_gap() {
    let mut server = StreamServer::respond_without_content_length_until_client_closes(
        "200 OK",
        "",
        Duration::from_secs(2),
    );
    let endpoint = server.endpoint().with_tenant_scope(
        TenantScope::new("platform-chat-timeout", "company-chat-timeout").expect("tenant"),
    );
    let body = GatewayChatStreamRequest::default();
    let started = Instant::now();

    let error = super::stream_chat_with_idle_timeout(
        &endpoint,
        None,
        &body,
        Duration::from_millis(50),
        |_| true,
    )
    .await
    .unwrap_err();
    let elapsed = started.elapsed();
    let request = server.join();

    assert!(elapsed < Duration::from_secs(1), "idle timeout should be prompt: {elapsed:?}");
    assert!(error.contains("idle timeout"));
    assert!(error.contains("without response"));
    assert!(request.starts_with("POST /v1/chat/stream HTTP/1.1"));
    assert_raw_tenant_headers(&request, "platform-chat-timeout", "company-chat-timeout");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn stream_chat_blocking_reads_events_and_honors_callback_stop() {
    let _trace = enable_stream_trace();
    let mut server =
        StreamServer::respond("200 OK", "data: {\"type\":\"chat.delta\",\"delta\":\"b\"}\r\n\r\n");
    let endpoint = server.endpoint().with_tenant_scope(
        TenantScope::new("platform-chat-blocking", "company-chat-blocking").expect("tenant"),
    );
    let body = GatewayChatStreamRequest::default();
    let mut events = Vec::new();

    super::stream_chat_blocking(&endpoint, Some(" /blocking/project "), &body, |event| {
        events.push(event);
        false
    })
    .unwrap();
    let request = server.join();

    assert_eq!(events, vec![GatewayChatStreamEvent::Delta("b".to_string())]);
    assert!(request.starts_with("POST /v1/chat/stream?directory="));
    assert_raw_tenant_headers(&request, "platform-chat-blocking", "company-chat-blocking");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn stream_chat_blocking_stops_after_done_without_waiting_for_socket_close() {
    let _trace = enable_stream_trace();
    let mut server = StreamServer::respond_without_content_length_until_client_closes(
        "200 OK",
        "data: {\"type\":\"chat.done\",\"finish_reason\":\"stop\"}\n\n",
        Duration::from_secs(2),
    );
    let endpoint = server.endpoint();
    let body = GatewayChatStreamRequest::default();
    let mut events = Vec::new();
    let started = Instant::now();

    super::stream_chat_blocking(&endpoint, None, &body, |event| {
        events.push(event);
        true
    })
    .unwrap();
    let elapsed = started.elapsed();
    let request = server.join();

    assert!(
        elapsed < Duration::from_secs(1),
        "stream should stop on chat.done, elapsed={elapsed:?}"
    );
    assert_eq!(
        events,
        vec![GatewayChatStreamEvent::Done {
            finish_reason: Some("stop".to_string()),
            usage: None,
            message_id: None,
            parent_message_id: None,
        }]
    );
    assert!(request.starts_with("POST /v1/chat/stream HTTP/1.1"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn stream_chat_blocking_completes_when_server_closes_stream() {
    let _trace = enable_stream_trace();
    let mut server = StreamServer::respond(
        "200 OK",
        "event: ping\r\n\r\ndata: {\"type\":\"chat.delta\",\"delta\":\"c\"}\r\n\r\n",
    );
    let endpoint = server.endpoint();
    let body = GatewayChatStreamRequest::default();
    let mut events = Vec::new();

    super::stream_chat_blocking(&endpoint, None, &body, |event| {
        events.push(event);
        true
    })
    .unwrap();
    let _ = server.join();

    assert_eq!(events, vec![GatewayChatStreamEvent::Delta("c".to_string())]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn stream_chat_blocking_reports_http_and_json_errors() {
    let _trace = enable_stream_trace();
    let mut http_server = StreamServer::respond("503 Service Unavailable", "offline");
    let http_error = super::stream_chat_blocking(
        &http_server.endpoint(),
        None,
        &GatewayChatStreamRequest::default(),
        |_| true,
    )
    .unwrap_err();
    let _ = http_server.join();
    assert!(http_error.contains("gateway stream failed: 503 Service Unavailable offline"));

    let mut json_server = StreamServer::respond("200 OK", "data: not-json\n\n");
    let json_error = super::stream_chat_blocking(
        &json_server.endpoint(),
        None,
        &GatewayChatStreamRequest::default(),
        |_| true,
    )
    .unwrap_err();
    let _ = json_server.join();
    assert!(json_error.contains("expected ident") || json_error.contains("expected value"));
}
