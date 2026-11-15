use super::messages_gemini_stream::{Decoder, response_body};
use axum::body::{Body, Bytes, to_bytes};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

fn wire(frames: &[Value]) -> String {
    frames.iter().map(|frame| format!("data: {frame}\n\n")).collect()
}
fn events(output: &str) -> Vec<Value> {
    output
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(|data| serde_json::from_str(data).unwrap())
        .collect()
}
async fn convert(input: String) -> Vec<Value> {
    let chunks =
        input.into_bytes().into_iter().map(|byte| Ok::<_, std::io::Error>(Bytes::from(vec![byte])));
    let body = response_body(
        Body::from_stream(futures_util::stream::iter(chunks)),
        "client".into(),
        CancellationToken::new(),
    );
    let output = to_bytes(body, 65536).await.unwrap();
    events(std::str::from_utf8(&output).unwrap())
}
fn finished() -> Value {
    json!({"candidates":[{"index":0,"finishReason":"STOP"}]})
}

#[tokio::test]
async fn cc_route_049_messages_gemini_stream_expected_behavior() {
    // Real finite HTTP upstream; byte chunks split UTF-8 and SSE boundaries.
    use axum::{Router, response::Response, routing::get};
    let input = wire(&[
        json!({"responseId":"reply","modelVersion":"gemini","candidates":[{"index":0,"content":{"parts":[
            {"text":"private thought","thought":true},{"text":"你"},
            {"functionCall":{"name":"lookup","args":{"city":"杭州"}},"thoughtSignature":"opaque"},
            {"functionCall":{"id":"","name":"lookup","args":{"city":"上海"}}}]}}]}),
        json!({"candidates":[{"index":0,"content":{"parts":[{"text":"你好"},
            {"functionCall":{"id":"real-id","name":"lookup","args":{"city":"杭州","units":"c"}}},
            {"functionCall":{"name":"lookup","args":{"city":"上海"}}}]},"finishReason":"STOP"}]}),
        json!({"usageMetadata":{"promptTokenCount":20,"cachedContentTokenCount":8,"totalTokenCount":25}}),
    ]);
    let router = Router::new().route(
        "/stream",
        get(move || {
            let input = input.clone();
            async move {
                let chunks = input
                    .into_bytes()
                    .into_iter()
                    .map(|byte| Ok::<_, std::io::Error>(Bytes::from(vec![byte])));
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
        .get(format!("http://{address}/stream"))
        .send()
        .await
        .unwrap();
    let body = response_body(
        Body::from_stream(upstream.bytes_stream()),
        "client".into(),
        CancellationToken::new(),
    );
    let output = to_bytes(body, 65536).await.unwrap();
    server.abort();
    let _ = server.await;
    let output = std::str::from_utf8(&output).unwrap();
    let events = events(output);
    assert!(!output.contains("private thought"));
    assert!(!output.contains("opaque"));
    assert_eq!(events[0]["message"]["id"], "reply");
    assert_eq!(events[0]["message"]["model"], "client");
    let text = events.iter().filter_map(|v| v["delta"]["text"].as_str()).collect::<String>();
    assert_eq!(text, "你好");
    let tools: Vec<_> =
        events.iter().filter(|v| v["content_block"]["type"] == "tool_use").collect();
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0]["content_block"]["id"], "real-id");
    assert!(tools[1]["content_block"]["id"].as_str().unwrap().starts_with("toolu_"));
    assert_ne!(tools[0]["content_block"]["id"], tools[1]["content_block"]["id"]);
    let inputs: Vec<Value> = events
        .iter()
        .filter_map(|v| v["delta"]["partial_json"].as_str())
        .map(|input| serde_json::from_str(input).unwrap())
        .collect();
    assert_eq!(inputs, vec![json!({"city":"杭州","units":"c"}), json!({"city":"上海"})]);
    let terminal = events.iter().find(|v| v["type"] == "message_delta").unwrap();
    assert_eq!(terminal["delta"]["stop_reason"], "tool_use");
    assert_eq!(
        terminal["usage"],
        json!({"input_tokens":12,"output_tokens":5,"cache_read_input_tokens":8})
    );
    assert_eq!(events.iter().filter(|v| v["type"] == "message_stop").count(), 1);
}

#[tokio::test]
async fn cc_route_049_invalid_stream_emits_one_error_without_success() {
    for input in [
        String::new(),
        wire(&[json!({"candidates":[{"content":{"parts":[{"text":"partial"}]}}]})]),
        wire(&[json!({"error":{"message":"secret upstream details"}})]),
        format!("{}data: {{invalid}}\n\n", wire(&[finished()])),
        wire(&[finished(), finished()]),
        wire(&[json!({"candidates":[{"index":1,"finishReason":"STOP"}]})]),
        wire(&[json!({"candidates":[{"finishReason":"UNKNOWN"}]})]),
        wire(&[
            json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"inlineData":{}}]}}]}),
        ]),
        wire(&[
            json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"functionCall":{"name":"f","args":"{"}}]}}]}),
        ]),
        wire(&[finished(), json!({"usageMetadata":{"promptTokenCount":1,"totalTokenCount":0}})]),
        "event: error\ndata: {\"candidates\":[]}\n\n".into(),
    ] {
        let output = convert(input).await;
        assert_eq!(output.iter().filter(|v| v["type"] == "error").count(), 1);
        assert!(!output.iter().any(|v| v["type"] == "message_stop"));
        assert!(!output.iter().any(|v| v.to_string().contains("secret upstream details")));
    }
}

#[tokio::test]
async fn cc_route_049_refusal_and_token_limit_preserve_terminal_policy() {
    for (frame, reason) in [
        (json!({"promptFeedback":{"blockReason":"SAFETY"}}), "refusal"),
        (
            json!({"candidates":[{"finishReason":"SAFETY","content":{"parts":[{"functionCall":{"name":"danger","args":{}}}]}}]}),
            "refusal",
        ),
        (json!({"candidates":[{"finishReason":"MAX_TOKENS"}]}), "max_tokens"),
    ] {
        let output = convert(wire(&[frame])).await;
        let terminal = output.iter().find(|v| v["type"] == "message_delta").unwrap();
        assert_eq!(terminal["delta"]["stop_reason"], reason);
        assert!(!output.iter().any(|v| v["content_block"]["type"] == "tool_use"));
        assert_eq!(output.last().unwrap()["type"], "message_stop");
    }
}

#[test]
fn cc_route_049_success_waits_for_complete_eof_and_respects_bounds() {
    let mut decoder = Decoder::new("client".into());
    let output = decoder.push(wire(&[finished()]).as_bytes()).unwrap();
    assert!(!output.contains("message_stop"));
    assert!(decoder.finish().unwrap().contains("message_stop"));
    assert!(decoder.finish().is_err());
    let mut decoder = Decoder::new("client".into());
    assert!(decoder.push(b"data: {\"candidates\":[{\"finishReason\":\"STOP\"}]}\n").is_ok());
    assert!(decoder.finish().is_err());
    assert!(Decoder::new("client".into()).push(&vec![b'x'; 4 * 1024 * 1024 + 1]).is_err());
    assert!(Decoder::new("client".into()).push(b"data: \xff\n\n").is_err());
}

#[tokio::test]
async fn cc_route_049_cancellation_does_not_finish_or_wait_for_upstream() {
    let cancel = CancellationToken::new();
    let body = Body::from_stream(futures_util::stream::pending::<Result<Bytes, std::io::Error>>());
    let body = response_body(body, "client".into(), cancel.clone());
    cancel.cancel();
    let bytes = tokio::time::timeout(std::time::Duration::from_secs(1), to_bytes(body, 4096))
        .await
        .unwrap()
        .unwrap();
    assert!(bytes.is_empty());
}

#[test]
fn cc_route_049_stream_action_preserves_authorized_resource() {
    for action in ["generateContent", "streamGenerateContent?alt=sse"] {
        let target = reqwest::Url::parse(&format!(
            "https://fixture.invalid/prefix/v1beta/models/selected:{action}"
        ))
        .unwrap();
        let url = super::messages_gemini_stream::stream_url(&target).unwrap();
        assert_eq!(
            url.as_str(),
            "https://fixture.invalid/prefix/v1beta/models/selected:streamGenerateContent?alt=sse"
        );
    }
    let target = reqwest::Url::parse(
        "https://fixture.invalid/v1beta/models/selected:generateContent?secret=value",
    )
    .unwrap();
    assert!(super::messages_gemini_stream::stream_url(&target).is_err());
}
