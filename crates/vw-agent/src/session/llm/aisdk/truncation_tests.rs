//! Verify length termination through the real OpenAI-compatible HTTP parser.
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use aisdk::core::{DynamicModel, Message};
use aisdk::providers::OpenAICompatible;
use serde_json::{Value, json};

use crate::app::agent::session::llm::types::{Error, StreamEvent};
use crate::app::agent::session::message::AssistantError;

use super::do_stream_request_aisdk_with_model;

async fn run_termination_response(
    reason: &'static str,
) -> (Result<(), Error>, Vec<StreamEvent>, Value) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local protocol fixture");
    let address = listener.local_addr().expect("fixture address");
    let request = Arc::new(Mutex::new(None::<Value>));
    let captured = Arc::clone(&request);
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().expect("accept fixture request");
        socket.set_read_timeout(Some(std::time::Duration::from_secs(10))).unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut buffer = [0; 4096];
            let read = socket.read(&mut buffer).expect("read fixture request");
            assert!(read > 0);
            bytes.extend_from_slice(&buffer[..read]);
            let Some(end) = bytes.windows(4).position(|v| v == b"\r\n\r\n") else {
                continue;
            };
            let headers = std::str::from_utf8(&bytes[..end]).unwrap();
            let length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .expect("request body length");
            if bytes.len() >= end + 4 + length {
                *captured.lock().unwrap() =
                    Some(serde_json::from_slice(&bytes[end + 4..end + 4 + length]).unwrap());
                break;
            }
        }
        let chunks = [
            json!({"id":"fixture","object":"chat.completion.chunk","created":0,
                   "model":"deepseek-v4-flash-0731","choices":[{"index":0,
                   "delta":{"content":"literal max_tokens text"},"finish_reason":null}]}),
            json!({"id":"fixture","object":"chat.completion.chunk","created":0,
                   "model":"deepseek-v4-flash-0731","choices":[{"index":0,
                   "delta":{},"finish_reason":reason}]}),
        ];
        let body = format!("data: {}\n\ndata: {}\n\ndata: [DONE]\n\n", chunks[0], chunks[1]);
        write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
    });
    let model = OpenAICompatible::<DynamicModel>::builder()
        .model_name("deepseek-v4-flash-0731")
        .api_key("fixture-token")
        .base_url(format!("http://{address}/v1"))
        .build()
        .expect("fixture model");
    let mut events = Vec::new();
    let result = do_stream_request_aisdk_with_model(
        "token-plan".into(),
        model,
        format!("http://{address}/v1/chat/completions"),
        false,
        vec![Message::User("protocol fixture".into())],
        &HashMap::new(),
        None,
        None,
        Some(32_000),
        &json!({}),
        0,
        None,
        &mut |event| events.push(event),
    )
    .await;
    server.join().expect("fixture server exited");
    let body = request.lock().unwrap().take().expect("captured request");
    assert_eq!(body["model"], "deepseek-v4-flash-0731");
    assert_eq!(body["max_completion_tokens"], 32_000);
    assert_eq!(body.pointer("/stream_options/include_usage"), Some(&json!(true)));
    (result, events, body)
}

#[tokio::test]
async fn literal_max_tokens_text_with_stop_is_not_truncation() {
    let (result, events, _) = run_termination_response("stop").await;
    assert!(result.is_ok());
    assert!(
        events.iter().any(|e| matches!(e, StreamEvent::Delta(s) if s == "literal max_tokens text"))
    );
    assert!(events.iter().any(|e| matches!(e, StreamEvent::Done { finish_reason, .. } if finish_reason.as_deref() == Some("stop"))));
}

#[tokio::test]
async fn provider_length_is_failed_without_executing_tools_or_emitting_done() {
    let (result, events, _) = run_termination_response("length").await;
    assert!(
        matches!(result, Err(Error::Api(AssistantError::APIError { message, metadata: Some(metadata), is_retryable: false, .. }))
        if message == "max_tokens" && metadata.get("stream_failure_kind").map(String::as_str) == Some("incomplete"))
    );
    assert!(
        events.iter().any(|e| matches!(e, StreamEvent::Delta(s) if s == "literal max_tokens text"))
    );
    assert!(
        !events.iter().any(|e| matches!(e, StreamEvent::Done { .. } | StreamEvent::ToolCalls(_)))
    );
}
