use axum::{
    body::{Body, to_bytes},
    response::Response,
};
use serde_json::{Value, json};

async fn convert(wire: String) -> Vec<Value> {
    use axum::{Router, routing::get};
    let router = Router::new().route(
        "/chat/completions",
        get(move || {
            let wire = wire.clone();
            async move {
                let chunks = wire
                    .into_bytes()
                    .into_iter()
                    .map(|b| Ok::<_, std::io::Error>(axum::body::Bytes::from(vec![b])));
                Response::builder()
                    .header("content-type", "text/event-stream")
                    .body(Body::from_stream(futures_util::stream::iter(chunks)))
                    .unwrap()
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let upstream = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .get(format!("http://{address}/chat/completions"))
        .send()
        .await
        .unwrap();
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from_stream(upstream.bytes_stream()))
        .unwrap();
    let response = super::messages_stream::resident_response_body(
        response,
        "client".into(),
        Default::default(),
    )
    .await
    .unwrap();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    server.abort();
    let _ = server.await;
    std::str::from_utf8(&bytes)
        .unwrap()
        .lines()
        .filter_map(|s| s.strip_prefix("data: "))
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn wire() -> String {
    let frames = [
        json!({"id":"r","choices":[{"index":0,"delta":{"content":"你好"}}]}),
        json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":1,"id":"b","type":"function","function":{"name":"second","arguments":"{"}},{"index":0,"id":"a","type":"function","function":{"name":"first","arguments":"{\"x\":"}}]}}]}),
        json!({"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"1}"}},{"index":1,"function":{"arguments":"}"}}]},"finish_reason":"tool_calls"}]}),
        json!({"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":2,"prompt_tokens_details":{"cached_tokens":3,"cache_write_tokens":1}}}),
    ];
    let mut wire = frames.iter().map(|v| format!("data: {v}\n\n")).collect::<String>();
    wire.push_str("data: [DONE]\n\n");
    wire
}
#[tokio::test]
async fn cc_route_040_messages_chat_stream_expected_behavior() {
    let events = convert(wire()).await;
    assert_eq!(events.iter().filter(|v| v["type"] == "message_stop").count(), 1);
    let usage = &events.iter().find(|v| v["type"] == "message_delta").unwrap()["usage"];
    assert_eq!(
        usage,
        &json!({"input_tokens":6,"output_tokens":2,"cache_read_input_tokens":3,"cache_creation_input_tokens":1})
    );
    let starts: Vec<_> = events.iter().filter(|v| v["type"] == "content_block_start").collect();
    assert_eq!(starts[1]["content_block"]["id"], "a");
    assert_eq!(starts[2]["index"], 2);
    let parts: Vec<_> =
        events.iter().filter(|v| v["delta"]["type"] == "input_json_delta").collect();
    assert_eq!(parts[0]["delta"]["partial_json"], "{\"x\":1}");
    assert_eq!(parts[1]["delta"]["partial_json"], "{}");
}

#[tokio::test]
async fn cc_route_040_invalid_or_incomplete_stream_fails_once() {
    for wire in [
        wire().replace("data: [DONE]\n\n", ""),
        wire().replace("1}", "1]"),
        "data: {\"error\":{\"message\":\"private upstream text\"}}\n\ndata: [DONE]\n\n".into(),
    ] {
        let events = convert(wire).await;
        assert_eq!(events.iter().filter(|v| v["type"] == "error").count(), 1);
        assert!(!events.iter().any(|v| v["type"] == "message_stop"));
        assert!(!events.iter().any(|v| v.to_string().contains("private upstream text")));
    }
}

#[tokio::test]
async fn cc_route_040_duplicate_terminal_emits_one_stop() {
    let mut input = wire();
    input.push_str("data: [DONE]\n\ndata: {invalid}\n\n");
    let events = convert(input).await;
    assert_eq!(events.iter().filter(|v| v["type"] == "message_stop").count(), 1);
    assert!(!events.iter().any(|v| v["type"] == "error"));
}

#[tokio::test]
async fn cc_route_040_session_usage_remains_strict() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(wire()))
        .unwrap();
    let response = super::messages_stream::response_body(
        response,
        "client".into(),
        true,
        false,
        Default::default(),
    )
    .await
    .unwrap();
    let bytes = to_bytes(response.into_body(), 65536).await.unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    let delta: Value = text
        .lines()
        .filter_map(|s| s.strip_prefix("data: "))
        .map(|s| serde_json::from_str::<Value>(s).unwrap())
        .find(|v| v["type"] == "message_delta")
        .unwrap();
    assert_eq!(delta["usage"], json!({"input_tokens":10,"output_tokens":2}));
}
