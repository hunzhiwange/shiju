use super::Decoder;
use axum::http::StatusCode;
use serde_json::json;

#[test]
fn chat_stream_accepts_empty_reasoning_and_completes_tools() {
    for empty in [serde_json::Value::Null, json!("")] {
        let mut decoder = Decoder { model: "selected".into(), ..Default::default() };
        decoder
            .frame(
                &json!({"id":"reply","choices":[{"index":0,"delta":{
                "role":"assistant","reasoning_content":empty,"tool_calls":null}}]})
                .to_string(),
            )
            .unwrap();
        decoder
            .frame(
                &json!({"id":"reply","choices":[{"index":0,"delta":{
            "tool_calls":[{"index":0,"id":"read-1","type":"function",
                "function":{"name":"Read","arguments":"{}"}}]},
            "finish_reason":"tool_calls"}],"usage":{"prompt_tokens":1,"completion_tokens":1}})
                .to_string(),
            )
            .unwrap();
        let result = decoder.frame("[DONE]").unwrap();
        assert!(result.contains("read-1"));
        assert!(result.contains("message_stop"));
        assert!(decoder.done);
    }
}

#[test]
fn chat_stream_rejects_nonempty_reasoning_in_nonthinking_mode() {
    let mut decoder = Decoder::default();
    assert_eq!(
        decoder.frame(
            &json!({"id":"reply","choices":[{"index":0,
            "delta":{"reasoning_content":"requires replay"}}]})
            .to_string()
        ),
        Err(StatusCode::UNPROCESSABLE_ENTITY)
    );
}

#[tokio::test]
async fn model_protocol_016_http_sse_thinking_receipt_replays() {
    let _lock = crate::test_support::global_async_env_lock().lock().await;
    super::super::messages_thinking_http_tests::thinking_replays_through_bound_http().await;
}

fn chat_wire(done: bool) -> String {
    let chunks = [
        json!({"id":"reply","choices":[{"index":0,"delta":{"role":"assistant","content":"你好"}}]}),
        json!({"id":"reply","choices":[{"index":0,"delta":{"tool_calls":[
            {"index":1,"id":"b","type":"function","function":{"name":"second","arguments":"{"}},
            {"index":0,"id":"a","type":"function","function":{"name":"first","arguments":"{\"城市\":"}}]}}]}),
        json!({"id":"reply","choices":[{"index":0,"delta":{"content":"","reasoning_content":null,"tool_calls":[
            {"index":0,"function":{"arguments":"\"杭州\"}"}},
            {"index":1,"function":{"arguments":"}"}}]},"finish_reason":"tool_calls"}]}),
        json!({"id":"reply","choices":[],"usage":{"prompt_tokens":7,"completion_tokens":9}}),
    ];
    let mut wire = chunks.iter().map(|v| format!("data: {v}\r\n\r\n")).collect::<String>();
    if done {
        wire.push_str("data: [DONE]\r\n\r\n");
    }
    wire
}

async fn converted(wire: String, cancel: tokio_util::sync::CancellationToken) -> String {
    use axum::{
        body::{Body, Bytes},
        response::Response,
    };
    let bytes =
        wire.into_bytes().into_iter().map(|byte| Ok::<_, std::io::Error>(Bytes::from(vec![byte])));
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from_stream(futures_util::stream::iter(bytes)))
        .unwrap();
    let result =
        super::response_body(response, "selected".into(), true, false, cancel).await.unwrap();
    String::from_utf8(
        axum::body::to_bytes(result.into_body(), 8 * 1024 * 1024).await.unwrap().to_vec(),
    )
    .unwrap()
}

#[tokio::test]
async fn model_protocol_016_utf8_parallel_tools_and_final_usage() {
    let text = converted(chat_wire(true), Default::default()).await;
    let events: Vec<serde_json::Value> = text
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(events.last().unwrap()["type"], "message_stop");
    assert_eq!(events.iter().filter(|v| v["type"] == "message_stop").count(), 1);
    let starts: Vec<_> = events.iter().filter(|v| v["type"] == "content_block_start").collect();
    assert_eq!(starts.len(), 3);
    assert_eq!(starts[1]["index"], 1);
    assert_eq!(starts[1]["content_block"]["id"], "a");
    assert_eq!(starts[2]["index"], 2);
    assert_eq!(starts[2]["content_block"]["id"], "b");
    assert!(text.contains("你好") && text.contains("杭州"));
    let delta = events.iter().find(|v| v["type"] == "message_delta").unwrap();
    assert_eq!(delta["usage"], json!({"input_tokens":7,"output_tokens":9}));
    assert_eq!(delta["delta"]["stop_reason"], "tool_use");
}

#[tokio::test]
async fn model_protocol_016_missing_done_and_invalid_arguments_never_succeed() {
    for wire in [chat_wire(false), chat_wire(true).replace("杭州\\\"}", "杭州\\\"]")] {
        let text = converted(wire, Default::default()).await;
        assert!(text.contains("event: error"), "{text}");
        assert!(!text.contains("message_stop"));
    }
}

#[tokio::test]
async fn model_protocol_016_cancellation_never_reports_success() {
    let cancel = tokio_util::sync::CancellationToken::new();
    cancel.cancel();
    assert!(converted(chat_wire(true), cancel).await.is_empty());
}

#[test]
fn model_protocol_016_input_limit_is_bounded() {
    let mut decoder = Decoder::default();
    assert_eq!(decoder.push(&vec![b'x'; super::LIMIT + 1]), Err(StatusCode::BAD_GATEWAY));
}

#[tokio::test]
async fn model_protocol_016_thinking_http_errors_never_issue_success() {
    use super::super::{RelayIdentity, messages_thinking::Context};
    use crate::app_routing::reference_scope::References;
    use axum::{Router, routing::get};
    use std::sync::Arc;
    let context = Context::new(
        &RelayIdentity {
            tenant: vw_api_types::id::TenantScope::new("stream", "fixture").unwrap(),
            project: "project".into(),
            session: "session".into(),
            provider: "provider".into(),
            model: "selected".into(),
            generation: 1,
        },
        "deployment".into(),
        Arc::new(References::default()),
    );
    for (wire, cancelled) in [
        (chat_wire(false), false),
        ("data: {\"error\":{\"message\":\"private-payload\"}}\n\n".into(), false),
        (chat_wire(true), true),
    ] {
        let router = Router::new().route(
            "/",
            get(move || {
                let wire = wire.clone();
                async move { ([("content-type", "text/event-stream")], wire) }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let response = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .get(format!("http://{address}/"))
            .send()
            .await
            .unwrap();
        let cancel = tokio_util::sync::CancellationToken::new();
        if cancelled {
            cancel.cancel();
        }
        let response = super::super::messages_thinking::response(
            response,
            context.clone(),
            "selected",
            true,
            cancel,
        )
        .await
        .unwrap();
        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let text = std::str::from_utf8(&body).unwrap();
        if cancelled {
            assert!(text.is_empty());
        } else {
            assert!(text.contains("event: error"));
        }
        assert!(!text.contains("message_stop") && !text.contains("signature_delta"));
        assert!(!text.contains("private-payload"));
        server.abort();
        let _ = server.await;
    }
}
