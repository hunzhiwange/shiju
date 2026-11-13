use super::{generation::GenerationProtocol, stream_observer::StreamObserver};

#[test]
fn cc_route_033_sse_framing_expected_behavior() {
    let wire = "\u{feff}data: {\"model\":\"fixture\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"中文😀\"},\"finish_reason\":null}]}\r\n\r\ndata: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
    for split in 0..=wire.len() {
        let mut observer = StreamObserver::new(GenerationProtocol::Chat);
        let first = observer.push(&wire.as_bytes()[..split]).unwrap();
        let second = observer.push(&wire.as_bytes()[split..]).unwrap();
        assert_eq!(first.text_events + second.text_events, 1, "split {split}");
        assert!(observer.finish().is_ok(), "split {split}");
    }
}

#[test]
fn cc_route_033_pending_and_invalid_utf8_are_rejected() {
    let mut lines = super::sse_framing::Lines::default();
    assert!(lines.push(b"1234", 4).unwrap().is_empty());
    assert!(lines.push(b"5", 4).is_err());
    let mut lines = super::sse_framing::Lines::default();
    assert!(lines.push(b"data: \xff\n", 100).is_err());
    let mut lines = super::sse_framing::Lines::default();
    assert!(lines.push(b"data: a\rb\n", 100).is_err());
}

#[tokio::test]
async fn cc_route_033_resident_tail_requires_terminal_and_preserves_json() {
    use axum::body::{Body, to_bytes};
    use tokio_util::sync::CancellationToken;
    for (format, _, valid) in super::stream_observer::tests::samples() {
        let protocol = match format {
            "openai_chat" => GenerationProtocol::Chat,
            "anthropic" => GenerationProtocol::Messages,
            "openai_responses" => GenerationProtocol::Responses,
            _ => GenerationProtocol::Gemini,
        };
        let wire = format!("\u{feff}{}", valid.trim_end_matches('\n'));
        let body = super::sse_framing::resident_body(Body::from(wire));
        let body = super::stream_observer::validated_body(body, protocol, CancellationToken::new());
        assert!(to_bytes(body, usize::MAX).await.is_ok(), "{format}");
        let truncated = valid.rsplit_once("data:").unwrap().0.to_owned();
        let body = super::sse_framing::resident_body(Body::from(truncated));
        let body = super::stream_observer::validated_body(body, protocol, CancellationToken::new());
        assert!(to_bytes(body, usize::MAX).await.is_err(), "{format}");
    }
    let data = "data: {\"arguments\":\"{\\\"中文\\\": [1, true, null]}\"}\r\n\r\n";
    let chunks = data
        .as_bytes()
        .iter()
        .map(|byte| Ok::<_, std::io::Error>(axum::body::Bytes::copy_from_slice(&[*byte])))
        .collect::<Vec<_>>();
    let body =
        super::sse_framing::resident_body(Body::from_stream(futures_util::stream::iter(chunks)));
    let bytes = to_bytes(body, usize::MAX).await.unwrap();
    assert_eq!(std::str::from_utf8(&bytes).unwrap().trim_end(), data.trim_end());
}

#[tokio::test]
async fn cc_route_033_loopback_http_tail_and_truncation() {
    use axum::body::{Body, to_bytes};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_util::sync::CancellationToken;
    let (_, _, valid) = super::stream_observer::tests::samples().remove(0);
    for wire in
        [valid.trim_end_matches('\n').to_owned(), valid.rsplit_once("data:").unwrap().0.to_owned()]
    {
        let complete = wire.contains("[DONE]");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let upstream = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                socket.read_exact(&mut byte).await.unwrap();
                request.push(byte[0]);
                assert!(request.len() < 8192);
            }
            let wire = format!("\u{feff}{wire}");
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", wire.len()).as_bytes()).await.unwrap();
            for byte in wire.as_bytes() {
                socket.write_all(&[*byte]).await.unwrap();
                tokio::task::yield_now().await;
            }
        });
        let response = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap()
            .get(format!("http://{address}/events"))
            .send()
            .await
            .unwrap();
        let body = super::sse_framing::resident_body(Body::from_stream(
            futures_util::stream::unfold(response, |mut response| async move {
                match response.chunk().await {
                    Ok(Some(bytes)) => Some((Ok::<_, std::io::Error>(bytes), response)),
                    Ok(None) => None,
                    Err(_) => Some((Err(std::io::Error::other("fixture read failed")), response)),
                }
            }),
        ));
        let body = super::stream_observer::validated_body(
            body,
            GenerationProtocol::Chat,
            CancellationToken::new(),
        );
        let result =
            tokio::time::timeout(std::time::Duration::from_secs(5), to_bytes(body, usize::MAX))
                .await
                .unwrap();
        assert_eq!(result.is_ok(), complete);
        upstream.await.unwrap();
    }
}
