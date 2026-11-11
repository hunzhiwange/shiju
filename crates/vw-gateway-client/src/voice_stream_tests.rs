use super::*;
#[tokio::test]
async fn voice_task_10_remote_cleartext_voice_is_rejected() {
    let start = VoiceSessionStart {
        request_id: "request".to_owned().try_into().unwrap(),
        chat_session_id: "chat".to_owned().try_into().unwrap(),
        generation: VoiceCounter::FIRST,
        provider_id: "openai".to_owned().try_into().unwrap(),
        turn_mode: TurnMode::Manual,
    };
    assert!(matches!(
        VoiceStream::connect(&GatewayEndpoint::new("example.invalid", 80), &start).await,
        Err(VoiceApiError::Service(VoiceErrorCode::UnsupportedCapability))
    ));
}

fn ident(s: &str) -> VoiceId {
    s.to_owned().try_into().unwrap()
}
fn start() -> VoiceSessionStart {
    VoiceSessionStart {
        request_id: ident("request"),
        chat_session_id: ident("chat"),
        generation: VoiceCounter::FIRST,
        provider_id: ident("openai"),
        turn_mode: TurnMode::Manual,
    }
}
// tungstenite's server callback fixes the large HTTP response error type.
#[allow(clippy::result_large_err)]
async fn loopback(
    bad_path: bool,
    foreign_frame: bool,
) -> (GatewayEndpoint, tokio::task::JoinHandle<()>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
        .with_auth(crate::GatewayAuth { skey: Some("synthetic-gateway-key".into()) });
    let task = tokio::spawn(async move {
        let (mut http, _) = listener.accept().await.unwrap();
        let mut bytes = vec![];
        let mut byte = [0; 1];
        while !bytes.ends_with(b"\r\n\r\n") {
            http.read_exact(&mut byte).await.unwrap();
            bytes.push(byte[0]);
            assert!(bytes.len() < 8192);
        }
        let headers = String::from_utf8(bytes).unwrap();
        assert!(headers.starts_with("POST /api/voice/sessions HTTP/1.1"));
        assert!(headers.contains("authorization: Bearer synthetic-gateway-key"));
        assert!(headers.contains("x-platform-id:"));
        let connection = headers
            .lines()
            .find_map(|h| h.strip_prefix("x-shiju-voice-connection-id: "))
            .unwrap()
            .to_owned();
        let len: usize = headers
            .lines()
            .find_map(|h| h.strip_prefix("content-length: "))
            .unwrap()
            .parse()
            .unwrap();
        let mut body = vec![0; len];
        http.read_exact(&mut body).await.unwrap();
        assert_eq!(serde_json::from_slice::<VoiceSessionStart>(&body).unwrap(), start());
        let session = VoiceSessionStarted {
            request_id: ident("request"),
            chat_session_id: ident("chat"),
            generation: VoiceCounter::FIRST,
            voice_session_id: ident("session-1"),
            stream_path: if bad_path {
                "http://other.invalid/steal".into()
            } else {
                "/api/voice/sessions/session-1/stream".into()
            },
            format: AudioFormat::default(),
            voice: VoiceName::Marin,
            expires_in_ms: MAX_SESSION_MS,
        };
        let body = serde_json::to_string(&session).unwrap();
        http.write_all(format!("HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
        drop(http);
        if bad_path {
            assert!(
                tokio::time::timeout(Duration::from_millis(100), listener.accept()).await.is_err(),
                "client followed an untrusted stream URL"
            );
            return;
        }
        let (socket, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_hdr_async(
            socket,
            move |request: &tokio_tungstenite::tungstenite::handshake::server::Request,
                  response| {
                assert_eq!(request.uri().path(), "/api/voice/sessions/session-1/stream");
                assert!(request.uri().query().is_none());
                assert_eq!(request.headers()["authorization"], "Bearer synthetic-gateway-key");
                assert_eq!(request.headers()["x-shiju-voice-connection-id"], connection);
                Ok(response)
            },
        )
        .await
        .unwrap();
        let ready = VoiceFrame {
            voice_session_id: ident("session-1"),
            generation: VoiceCounter::FIRST,
            seq: VoiceCounter::FIRST,
            event: VoiceEvent::SessionReady {
                format: AudioFormat::default(),
                voice: VoiceName::Marin,
            },
        };
        socket.send(Message::Text(serde_json::to_string(&ready).unwrap().into())).await.unwrap();
        let sent = socket.next().await.unwrap().unwrap();
        let frame = VoiceFrame::parse(sent.into_data().as_ref()).unwrap();
        assert_eq!(frame.voice_session_id, ident("session-1"));
        assert_eq!(frame.seq, VoiceCounter::FIRST);
        assert!(matches!(frame.event, VoiceEvent::ResponseInterrupt { .. }));
        let ack = VoiceFrame {
            voice_session_id: ident(if foreign_frame { "foreign" } else { "session-1" }),
            generation: 2.try_into().unwrap(),
            seq: 2.try_into().unwrap(),
            event: VoiceEvent::GenerationChanged {
                reason: GenerationReason::Interrupt,
                response_id: Some(ident("r1")),
                item_id: None,
                content_index: None,
            },
        };
        socket.send(Message::Text(serde_json::to_string(&ack).unwrap().into())).await.unwrap();
        let after_ack = socket.next().await;
        assert!(
            !matches!(after_ack, Some(Ok(Message::Text(_)))),
            "retired capture audio must not reach the gateway socket"
        );
    });
    (endpoint, task)
}
fn interrupt() -> VoiceEvent {
    VoiceEvent::ResponseInterrupt {
        response_id: ident("r1"),
        item_id: None,
        content_index: None,
        audio_end_ms: None,
    }
}
#[tokio::test]
async fn voice_task_10_loopback_authenticated_duplex_and_generation_ack() {
    let (endpoint, task) = loopback(false, false).await;
    let mut stream = VoiceStream::connect(&endpoint, &start()).await.unwrap();
    stream.send(VoiceCounter::FIRST, interrupt()).await.unwrap();
    assert!(matches!(stream.next().await.unwrap().event, VoiceEvent::GenerationChanged { .. }));
    assert_eq!(stream.generation().get(), 2);
    stream.close().await;
    task.await.unwrap();
}
#[tokio::test]
async fn voice_task_10_foreign_output_is_rejected() {
    let (endpoint, task) = loopback(false, true).await;
    let mut stream = VoiceStream::connect(&endpoint, &start()).await.unwrap();
    stream.send(VoiceCounter::FIRST, interrupt()).await.unwrap();
    assert_eq!(stream.next().await.unwrap_err(), VoiceApiError::InvalidResponse);
    assert_eq!(stream.next().await.unwrap_err(), VoiceApiError::Service(VoiceErrorCode::Cancelled));
    stream.close().await;
    task.await.unwrap();
}
#[tokio::test]
async fn voice_task_10_gateway_supplied_foreign_url_is_rejected() {
    let (endpoint, task) = loopback(true, false).await;
    assert!(matches!(
        VoiceStream::connect(&endpoint, &start()).await,
        Err(VoiceApiError::InvalidResponse)
    ));
    task.await.unwrap();
}

#[tokio::test]
async fn voice_task_10_stale_capture_generation_is_not_relabelled() {
    let (endpoint, task) = loopback(false, false).await;
    let mut stream = VoiceStream::connect(&endpoint, &start()).await.unwrap();
    stream.send(VoiceCounter::FIRST, interrupt()).await.unwrap();
    stream.next().await.unwrap();
    assert_eq!(stream.generation().get(), 2);
    assert_eq!(
        stream
            .send(
                VoiceCounter::FIRST,
                VoiceEvent::AudioAppend(AudioPayload {
                    audio: "AAA=".into(),
                    samples_per_channel: 1
                })
            )
            .await,
        Err(VoiceApiError::Service(VoiceErrorCode::Cancelled)),
        "retired microphone samples must not be relabelled as the current generation"
    );
    stream.close().await;
    task.await.unwrap();
}
