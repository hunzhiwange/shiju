use super::*;

#[test]
fn media_adapter_capabilities_are_specific_to_the_wire_protocol() {
    assert_eq!(MediaProtocol::from_endpoint_type("openai-video"), Some(MediaProtocol::Video));
    assert_eq!(MediaProtocol::from_endpoint_type("video-generation"), None);
    assert_eq!(
        MediaProtocol::from_endpoint_type("audio-transcription"),
        Some(MediaProtocol::Transcription)
    );
    assert_eq!(MediaProtocol::from_endpoint_type("realtime-asr"), None);
    assert_eq!(
        MediaProtocol::from_endpoint_type("gemini-chat-image"),
        Some(MediaProtocol::GeminiChatImage)
    );
}

#[test]
fn media_adapter_rejects_ambiguous_gateway_base_urls() {
    for url in [
        "https://key@example.test/v1",
        "https://example.test/v1?token=key",
        "http://example.test/v1",
        "file:///tmp/provider",
    ] {
        assert!(deployment_url(url).is_none());
    }
    assert!(deployment_url("http://127.0.0.1:8181/v1").is_some());
    assert!(deployment_url("https://example.test/v1").is_some());
}

fn endpoint(protocol: MediaProtocol, base: &str) -> MediaEndpoint {
    MediaEndpoint {
        protocol,
        provider: "fixture".into(),
        model: "fixture-media".into(),
        base: deployment_url(base).unwrap(),
        path: "chat/completions".into(),
        bearer: "fixture-private-key".into(),
        video: OpenAIVideoCapabilities::default(),
        voice: None,
        result_origin: None,
    }
}

#[tokio::test]
async fn media_adapter_vision_rejects_native_video_without_protocol_support() {
    let endpoint = endpoint(MediaProtocol::ChatVision, "http://127.0.0.1:1/v1");
    let file = MediaFile { mime: "video/mp4".into(), bytes: vec![1] };
    let result = endpoint
        .execute(Some("Describe it".into()), vec![file], &CancellationToken::new(), |_| {})
        .await;
    assert!(matches!(result, Err(MediaFailure::Unsupported)));
}

#[test]
fn media_adapter_result_origin_never_accepts_a_provider_credential_url() {
    let mut endpoint = endpoint(MediaProtocol::Speech, "https://provider.example/v1");
    endpoint.result_origin = deployment_url("https://media.example");
    for blocked in [
        "https://provider.example.evil.test/result",
        "https://key@media.example/result",
        "https://media.example/result?key=fixture%2Dprivate%2Dkey",
        "http://media.example/result",
        "https://127.0.0.1/result",
    ] {
        assert!(!endpoint.allows_result_url(&Url::parse(blocked).unwrap()));
    }
    for allowed in ["https://provider.example/file", "https://media.example/file?sig=temporary"] {
        assert!(endpoint.allows_result_url(&Url::parse(allowed).unwrap()));
    }
}

#[tokio::test]
async fn media_adapter_speech_uses_selected_model_and_deployment() {
    use axum::{
        Json, Router,
        body::Bytes,
        http::{HeaderMap, header},
        routing::post,
    };
    use serde_json::Value;

    let app = Router::new().route(
        "/v1/audio/speech",
        post(|headers: HeaderMap, Json(body): Json<Value>| async move {
            assert_eq!(headers[header::AUTHORIZATION], "Bearer fixture-private-key");
            assert_eq!(body["model"], "fixture-media");
            assert_eq!(body["input"], "read this");
            ([(header::CONTENT_TYPE, "audio/mpeg")], Bytes::from_static(b"ID3fixture"))
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let endpoint = endpoint(MediaProtocol::Speech, &base);
    let result = endpoint
        .execute(Some("read this".into()), Vec::new(), &CancellationToken::new(), |_| {})
        .await
        .unwrap();
    server.abort();
    assert!(result.text.is_none());
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].mime, "audio/mpeg");
    assert_eq!(result.files[0].bytes, b"ID3fixture");
}
