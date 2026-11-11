use super::{StreamObserver, validated_body};
use crate::model_relay::generation::GenerationProtocol;
use axum::body::{Body, to_bytes};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

pub(crate) fn samples() -> Vec<(&'static str, &'static str, String)> {
    let data = |value: Value| format!("data: {value}\n\n");
    vec![
        ("openai_chat", "/v1/chat/completions", [
            data(json!({"object":"chat.completion.chunk","model":"fixture-v1","choices":[{"index":0,"delta":{"role":"assistant","content":"OK"},"finish_reason":null}]})),
            data(json!({"object":"chat.completion.chunk","model":"fixture-v1","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]})),
            "data: [DONE]\n\n".into(),
        ].concat()),
        ("anthropic", "/v1/messages", [
            data(json!({"type":"message_start","message":{"id":"m","role":"assistant","model":"fixture-v1","type":"message","content":[]}})),
            data(json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}})),
            data(json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"OK"}})),
            data(json!({"type":"content_block_stop","index":0})),
            data(json!({"type":"message_delta","delta":{"stop_reason":"end_turn"}})),
            data(json!({"type":"message_stop"})),
        ].concat()),
        ("openai_responses", "/v1/responses", [
            data(json!({"type":"response.created","response":{"id":"r","object":"response","model":"fixture-v1","status":"in_progress"}})),
            data(json!({"type":"response.output_text.delta","delta":"OK","item_id":"m","output_index":0,"content_index":0})),
            data(json!({"type":"response.completed","response":{"id":"r","object":"response","model":"fixture-v1","status":"completed"}})),
        ].concat()),
        ("gemini_native", "/v1/models/fixture:streamGenerateContent?alt=sse", [
            data(json!({"modelVersion":"fixture-v1","candidates":[{"index":0,"content":{"role":"model","parts":[{"text":"OK"}]}}]})),
            data(json!({"modelVersion":"fixture-v1","candidates":[{"index":0,"finishReason":"STOP"}]})),
        ].concat()),
    ]
}

fn protocol(format: &str) -> GenerationProtocol {
    match format {
        "openai_chat" => GenerationProtocol::Chat,
        "anthropic" => GenerationProtocol::Messages,
        "openai_responses" => GenerationProtocol::Responses,
        "gemini_native" => GenerationProtocol::Gemini,
        _ => unreachable!(),
    }
}

#[test]
fn shared_stream_observer_validates_four_protocols_and_fragmentation() {
    for (format, _, valid) in samples() {
        let protocol = protocol(format);
        for size in [1, 7, valid.len()] {
            let mut observer = StreamObserver::new(protocol);
            let mut text_events = 0;
            let valid = valid.replace("OK", "你好");
            for chunk in valid.as_bytes().chunks(size) {
                text_events += observer.push(chunk).expect(format).text_events;
            }
            assert_eq!(text_events, 1, "{format}");
            assert!(observer.finish().is_ok(), "{format}");
            assert_eq!(observer.actual_model(), Some("fixture-v1"));
        }
        for invalid in [
            String::new(),
            ": heartbeat\n\n".into(),
            "data: {broken}\n\n".into(),
            valid.rsplit_once("data:").unwrap().0.into(),
            "data: [DONE]\n\n".into(),
        ] {
            let mut observer = StreamObserver::new(protocol);
            assert!(
                observer.push(invalid.as_bytes()).is_err() || observer.finish().is_err(),
                "{format} must reject {invalid}"
            );
        }
    }
}

#[tokio::test]
async fn shared_stream_observer_production_body_preserves_bytes_and_cancels() {
    for (format, _, valid) in samples() {
        let body =
            validated_body(Body::from(valid.clone()), protocol(format), CancellationToken::new());
        assert_eq!(to_bytes(body, usize::MAX).await.unwrap().as_ref(), valid.as_bytes());
        let open = Body::from_stream(futures_util::StreamExt::chain(
            futures_util::stream::once(std::future::ready(Ok::<_, std::io::Error>(
                axum::body::Bytes::from(valid.clone()),
            ))),
            futures_util::stream::pending(),
        ));
        let body = validated_body(open, protocol(format), CancellationToken::new());
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(1), to_bytes(body, usize::MAX))
                .await
                .unwrap()
                .is_ok(),
            "legal ending must release {format} upstream without waiting for EOF"
        );
        let truncated = valid.rsplit_once("data:").unwrap().0.to_owned();
        let body =
            validated_body(Body::from(truncated), protocol(format), CancellationToken::new());
        assert!(to_bytes(body, usize::MAX).await.is_err(), "{format} must surface incomplete EOF");
        let cancel = CancellationToken::new();
        let body = Body::from_stream(futures_util::stream::pending::<
            Result<axum::body::Bytes, std::io::Error>,
        >());
        let body = validated_body(body, protocol(format), cancel.clone());
        cancel.cancel();
        assert!(to_bytes(body, usize::MAX).await.unwrap().is_empty());
    }
}

#[test]
fn shared_stream_observer_rejects_illegal_end_and_model_changes() {
    for (format, _, valid) in samples() {
        let mut observer = StreamObserver::new(protocol(format));
        let terminal = format!("data:{}", valid.rsplit_once("data:").unwrap().1);
        if format == "gemini_native" {
            assert_eq!(observer.push(terminal.as_bytes()).unwrap().text_events, 0);
        } else {
            assert!(
                observer.push(terminal.as_bytes()).is_err() || observer.finish().is_err(),
                "{format} terminal without prior state"
            );
        }
        let mut observer = StreamObserver::new(protocol(format));
        let (head, tail) = valid.rsplit_once("data:").unwrap();
        observer.push(head.as_bytes()).unwrap();
        let tail = format!("data:{}", tail.replace("fixture-v1", "other-model"));
        if format != "openai_chat" && format != "anthropic" {
            assert!(
                observer.push(tail.as_bytes()).is_err(),
                "{format} must preserve actual upstream model"
            );
        }
        let mut observer = StreamObserver::new(protocol(format));
        assert!(observer.push(b"data: {\"error\":{\"message\":\"upstream\"}}\n\n").is_err());
        assert!(observer.push(b"data: \xff\n\n").is_err());
    }
}

pub(crate) async fn source(wire: String) -> reqwest::Response {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut headers = Vec::new();
        while !headers.ends_with(b"\r\n\r\n") {
            headers.push(socket.read_u8().await.unwrap());
            assert!(headers.len() < 8192);
        }
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{wire}", wire.len()).as_bytes()).await.unwrap();
    });
    let response =
        reqwest::Client::builder().no_proxy().build().unwrap().get(url).send().await.unwrap();
    server.await.unwrap();
    response
}

#[tokio::test]
async fn shared_stream_observer_native_converters_preserve_client_model_mapping() {
    use crate::model_relay::{RelayIdentity, gemini_stream, messages_stream, responses_stream};
    for (format, _, wire) in samples().into_iter().filter(|(format, _, _)| *format != "openai_chat")
    {
        let response = source(wire.clone()).await;
        let output = match format {
            "anthropic" => messages_stream::response(
                response,
                "client-model".into(),
                true,
                true,
                CancellationToken::new(),
            )
            .await
            .unwrap(),
            "gemini_native" => gemini_stream::response(
                response,
                "client-model".into(),
                true,
                true,
                CancellationToken::new(),
            )
            .await
            .unwrap(),
            "openai_responses" => responses_stream::response(
                response,
                responses_stream::Output {
                    identity: RelayIdentity {
                        tenant: vw_api_types::id::TenantScope::default(),
                        project: "project".into(),
                        session: "session".into(),
                        provider: "provider".into(),
                        model: "client-model".into(),
                        generation: 1,
                    },
                    native: true,
                    codex_chat: false,
                    tool_names: Default::default(),
                    streaming: true,
                    store: false,
                    references: Default::default(),
                },
                CancellationToken::new(),
            )
            .await
            .unwrap(),
            _ => unreachable!(),
        };
        let output = to_bytes(output.into_body(), usize::MAX).await.unwrap();
        let output = std::str::from_utf8(&output).unwrap();
        assert!(output.contains("client-model"), "{format}: {output}");
        assert!(!output.contains("fixture-v1"), "{format}: model mapping changed");
        assert!(
            !output.contains("Invalid") && !output.contains("response.failed"),
            "{format}: {output}"
        );

        let malformed = "data: {broken}\n\n".to_owned();
        let response = source(malformed).await;
        let output = match format {
            "anthropic" => messages_stream::response(
                response,
                "client-model".into(),
                true,
                true,
                CancellationToken::new(),
            )
            .await
            .unwrap(),
            "gemini_native" => gemini_stream::response(
                response,
                "client-model".into(),
                true,
                true,
                CancellationToken::new(),
            )
            .await
            .unwrap(),
            _ => continue,
        };
        let output = to_bytes(output.into_body(), usize::MAX).await.unwrap();
        assert!(std::str::from_utf8(&output).unwrap().contains("error"));
    }
}
