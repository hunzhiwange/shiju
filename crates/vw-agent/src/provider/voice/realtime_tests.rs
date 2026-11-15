use super::realtime::*;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;
use vw_api_types::voice::*;
use vw_config_types::voice::VoiceConfig;

fn ident(s: &str) -> VoiceId {
    s.to_owned().try_into().unwrap()
}
async fn fixture(events: Vec<Value>) -> (RealtimeSession, tokio::task::JoinHandle<Vec<Value>>) {
    fixture_mode(events, TurnMode::Manual).await
}
// tungstenite fixes the callback error type to an HTTP response.
#[allow(clippy::result_large_err)]
async fn fixture_mode(
    events: Vec<Value>,
    mode: TurnMode,
) -> (RealtimeSession, tokio::task::JoinHandle<Vec<Value>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_hdr_async(
            socket,
            |request: &tokio_tungstenite::tungstenite::handshake::server::Request, response| {
                assert_eq!(request.uri().path(), "/v1/realtime");
                assert_eq!(request.uri().query(), Some("model=gpt-realtime-2.1"));
                assert_eq!(request.headers()["authorization"], "Bearer synthetic-key");
                assert!(!request.headers().contains_key("openai-beta"));
                Ok(response)
            },
        )
        .await
        .unwrap();
        ws.send(Message::Text(
            json!({"type":"session.created","session":{"id":"upstream"}}).to_string().into(),
        ))
        .await
        .unwrap();
        let update: Value =
            serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
        let mut session = update["session"].clone();
        session["id"] = json!("upstream");
        ws.send(Message::Text(
            json!({"type":"session.updated","session":session}).to_string().into(),
        ))
        .await
        .unwrap();
        for event in events {
            ws.send(Message::Text(event.to_string().into())).await.unwrap();
        }
        let mut commands = vec![update];
        while let Some(Ok(Message::Text(text))) = ws.next().await {
            commands.push(serde_json::from_str(&text).unwrap());
        }
        commands
    });
    let config = VoiceConfig { realtime_enabled: true, ..Default::default() };
    let session = RealtimeSession::connect(
        &base,
        "synthetic-key",
        reqwest::Client::builder().no_proxy(),
        &config,
        ident("openai"),
        ident("local"),
        mode,
    )
    .await
    .unwrap();
    (session, server)
}
fn started() -> Vec<Value> {
    vec![
        json!({"type":"response.created","response":{"id":"r1"}}),
        json!({"type":"response.output_item.added","response_id":"r1","item":{"id":"a1","type":"message"}}),
    ]
}
#[tokio::test]
async fn voice_task_09_realtime_cancel_stops_generation() {
    let mut events = started();
    events.push(json!({"type":"response.output_audio.delta","response_id":"r1","item_id":"a1","content_index":0,"delta":"AAAAAA=="}));
    let (mut session, server) = fixture(events).await;
    assert_eq!(session.next().await.unwrap().len(), 1);
    session.next().await.unwrap();
    session
        .input(
            VoiceCounter::FIRST,
            VoiceEvent::ResponseInterrupt {
                response_id: ident("r1"),
                item_id: None,
                content_index: None,
                audio_end_ms: None,
            },
        )
        .await
        .unwrap();
    assert!(
        session.next().await.unwrap().is_empty(),
        "cancelled response must not emit queued late audio"
    );
    session.stop().await;
    let commands = server.await.unwrap();
    assert!(commands.iter().any(|v| v["type"] == "response.cancel"));
}
#[tokio::test]
async fn voice_task_09_transcript_final_is_idempotent() {
    let mut events = started();
    let final_event = json!({"type":"response.output_audio_transcript.done","response_id":"r1","item_id":"a1","content_index":0,"transcript":"你好。"});
    events.push(json!({"type":"response.output_audio_transcript.delta","response_id":"r1","item_id":"a1","content_index":0,"delta":"你好"}));
    events.push(final_event.clone());
    events.push(final_event);
    let (mut session, server) = fixture(events).await;
    session.next().await.unwrap();
    session.next().await.unwrap();
    assert!(matches!(session.next().await.unwrap()[0].event, VoiceEvent::TranscriptDelta { .. }));
    assert!(
        matches!(&session.next().await.unwrap()[0].event,VoiceEvent::TranscriptFinal{text,..} if text=="你好。")
    );
    assert!(
        session.next().await.unwrap().is_empty(),
        "duplicate final must not emit a second completion"
    );
    session.stop().await;
    server.await.unwrap();
}

#[tokio::test]
async fn voice_task_09_manual_commit_and_mute_only_stop_input() {
    use base64::Engine as _;
    let (mut session, server) = fixture(started()).await;
    session.next().await.unwrap();
    session.next().await.unwrap();
    session.input(VoiceCounter::FIRST, VoiceEvent::InputMute { muted: true }).await.unwrap();
    let audio = AudioPayload {
        audio: base64::engine::general_purpose::STANDARD.encode(vec![0; 4800]),
        samples_per_channel: 2400,
    };
    session.input(VoiceCounter::FIRST, VoiceEvent::AudioAppend(audio.clone())).await.unwrap();
    session.input(VoiceCounter::FIRST, VoiceEvent::InputMute { muted: false }).await.unwrap();
    session.input(VoiceCounter::FIRST, VoiceEvent::AudioAppend(audio)).await.unwrap();
    session.input(VoiceCounter::FIRST, VoiceEvent::InputCommit {}).await.unwrap();
    session.stop().await;
    let commands = server.await.unwrap();
    for kind in ["input_audio_buffer.append", "input_audio_buffer.commit", "response.create"] {
        assert_eq!(commands.iter().filter(|v| v["type"] == kind).count(), 1);
    }
    assert!(!commands.iter().any(|v| v["type"] == "response.cancel"));
}

#[tokio::test]
async fn voice_task_09_audio_chunks_and_truncate_use_played_position() {
    use base64::Engine as _;
    let mut events = started();
    events.push(json!({"type":"response.output_audio.delta","response_id":"r1","item_id":"a1","content_index":0,"delta":base64::engine::general_purpose::STANDARD.encode(vec![0;9600])}));
    let (mut session, server) = fixture(events).await;
    session.next().await.unwrap();
    session.next().await.unwrap();
    let frames = session.next().await.unwrap();
    assert_eq!(frames.len(), 2);
    for frame in frames {
        assert!(matches!(frame.event, VoiceEvent::AudioDelta { samples_per_channel: 2400, .. }));
    }
    session
        .input(
            VoiceCounter::FIRST,
            VoiceEvent::PlaybackProgress {
                response_id: ident("r1"),
                item_id: ident("a1"),
                content_index: 0,
                audio_end_ms: 100,
            },
        )
        .await
        .unwrap();
    session
        .input(
            VoiceCounter::FIRST,
            VoiceEvent::ResponseInterrupt {
                response_id: ident("r1"),
                item_id: Some(ident("a1")),
                content_index: Some(0),
                audio_end_ms: Some(100),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        session.input(VoiceCounter::FIRST, VoiceEvent::InputCommit {}).await,
        Err(VoiceErrorCode::Cancelled)
    );
    session.stop().await;
    let commands = server.await.unwrap();
    assert!(commands.iter().any(|v|v==&json!({"type":"conversation.item.truncate","item_id":"a1","content_index":0,"audio_end_ms":100})));
}

#[tokio::test]
async fn voice_task_09_disconnect_ends_without_reconnect() {
    let (mut session, server) = fixture(vec![]).await;
    server.abort();
    let _ = server.await;
    assert!(session.next().await.is_err());
    assert_eq!(
        session.input(VoiceCounter::FIRST, VoiceEvent::InputCommit {}).await,
        Err(VoiceErrorCode::Cancelled)
    );
}

#[tokio::test]
async fn voice_task_09_vad_interrupt_retires_without_duplicate_cancel() {
    let mut events = started();
    events.push(
        json!({"type":"input_audio_buffer.speech_started","item_id":"u2","audio_start_ms":100}),
    );
    events.push(json!({"type":"response.output_audio.delta","response_id":"r1","item_id":"a1","content_index":0,"delta":"AAAAAA=="}));
    let (mut session, server) = fixture_mode(events, TurnMode::ServerVad).await;
    session.next().await.unwrap();
    session.next().await.unwrap();
    assert!(matches!(
        session.next().await.unwrap()[0].event,
        VoiceEvent::GenerationChanged { reason: GenerationReason::Vad, .. }
    ));
    assert_eq!(session.generation().get(), 2);
    assert!(session.next().await.unwrap().is_empty());
    session.stop().await;
    assert!(!server.await.unwrap().iter().any(|v| v["type"] == "response.cancel"));
}

#[tokio::test]
async fn voice_task_09_invalid_audio_and_error_close_without_payload_leak() {
    let mut events = started();
    events.push(json!({"type":"response.output_audio.delta","response_id":"r1","item_id":"a1","content_index":0,"delta":"AA=="}));
    let (mut session, server) = fixture(events).await;
    session.next().await.unwrap();
    session.next().await.unwrap();
    assert_eq!(session.next().await.unwrap_err(), VoiceErrorCode::ProtocolError);
    assert_eq!(
        session.input(VoiceCounter::FIRST, VoiceEvent::InputCommit {}).await,
        Err(VoiceErrorCode::Cancelled)
    );
    server.await.unwrap();
    let (mut session,server)=fixture(vec![json!({"type":"error","error":{"message":"sensitive upstream text","code":"invalid_api_key"}})]).await;
    assert_eq!(session.next().await.unwrap_err(), VoiceErrorCode::ProviderUnavailable);
    server.await.unwrap();
}

#[tokio::test]
async fn voice_task_09_completed_response_can_truncate_unplayed_audio() {
    let mut events = started();
    events.push(json!({"type":"response.output_audio.delta","response_id":"r1","item_id":"a1","content_index":0,"delta":"AAAAAA=="}));
    events.push(
        json!({"type":"response.done","response":{"id":"r1","status":"completed","output":[]}}),
    );
    events.push(json!({"type":"conversation.item.truncated","item_id":"a1","content_index":0,"audio_end_ms":0}));
    let (mut session, server) = fixture(events).await;
    for _ in 0..4 {
        session.next().await.unwrap();
    }
    let ack = session
        .input(
            VoiceCounter::FIRST,
            VoiceEvent::ResponseInterrupt {
                response_id: ident("r1"),
                item_id: Some(ident("a1")),
                content_index: Some(0),
                audio_end_ms: Some(0),
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ack.generation.get(), 2);
    assert!(session.next().await.unwrap().is_empty());
    session.stop().await;
    let commands = server.await.unwrap();
    assert!(commands.iter().any(|v| v["type"] == "conversation.item.truncate"));
    assert!(!commands.iter().any(|v| v["type"] == "response.cancel"));
}

#[tokio::test]
async fn voice_task_09_negotiation_rejects_unsupported_voice() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
        ws.send(Message::Text(
            json!({"type":"session.created","session":{"id":"upstream"}}).to_string().into(),
        ))
        .await
        .unwrap();
        let update: Value =
            serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
        let mut session = update["session"].clone();
        session["id"] = json!("upstream");
        session["audio"]["output"]["voice"] = json!("unsupported");
        ws.send(Message::Text(
            json!({"type":"session.updated","session":session}).to_string().into(),
        ))
        .await
        .unwrap();
    });
    let config = VoiceConfig { realtime_enabled: true, ..Default::default() };
    assert!(matches!(
        RealtimeSession::connect(
            &base,
            "synthetic-key",
            reqwest::Client::builder().no_proxy(),
            &config,
            ident("openai"),
            ident("local"),
            TurnMode::Manual
        )
        .await,
        Err(VoiceErrorCode::UnsupportedCapability)
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn voice_task_09_user_transcript_merges_and_finalizes_once() {
    let (mut session,server)=fixture(vec![
        json!({"type":"input_audio_buffer.committed","item_id":"u1"}),
        json!({"type":"conversation.item.input_audio_transcription.delta","item_id":"u1","content_index":0,"delta":"你"}),
        json!({"type":"conversation.item.input_audio_transcription.completed","item_id":"u1","content_index":0,"transcript":"你好"}),
        json!({"type":"conversation.item.input_audio_transcription.completed","item_id":"u1","content_index":0,"transcript":"你好"}),
    ]).await;
    assert_eq!(session.provider_id(), &ident("openai"));
    assert_eq!(session.voice_session_id(), &ident("local"));
    assert!(session.next().await.unwrap().is_empty());
    let delta = session.next().await.unwrap().remove(0);
    assert_eq!(delta.seq.get(), 1);
    assert_eq!(delta.voice_session_id, ident("local"));
    let final_frame = session.next().await.unwrap().remove(0);
    assert_eq!(final_frame.seq.get(), 2);
    assert!(
        matches!(final_frame.event,VoiceEvent::TranscriptFinal{role:TranscriptRole::User,text,..} if text=="你好")
    );
    assert!(session.next().await.unwrap().is_empty());
    session.stop().await;
    server.await.unwrap();
}

fn screen_fixture() -> ScreenPayload {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let image = image::DynamicImage::new_rgb8(8, 8);
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    ScreenPayload { png: STANDARD.encode(bytes.into_inner()) }
}
#[tokio::test]
async fn voice_task_17_realtime_screen_uses_user_image_without_tools_or_auto_reply() {
    let (mut session, server) = fixture(vec![]).await;
    session.input(VoiceCounter::FIRST, VoiceEvent::ScreenShare(screen_fixture())).await.unwrap();
    session.stop().await;
    let commands = server.await.unwrap();
    assert_eq!(commands.len(), 2);
    let item = &commands[1]["item"];
    assert_eq!(commands[1]["type"], "conversation.item.create");
    assert_eq!(item["role"], "user");
    assert_eq!(item["content"][1]["type"], "input_image");
    assert!(
        item["content"][1]["image_url"].as_str().unwrap().starts_with("data:image/png;base64,")
    );
    assert!(item["content"][0]["text"].as_str().unwrap().contains("不可信"));
    assert!(commands[0]["session"]["instructions"].as_str().unwrap().contains("不可信"));
    assert_eq!(commands[0]["session"]["tools"].as_array().unwrap().len(), 1);
}
#[tokio::test]
async fn voice_task_17_realtime_screen_invalid_image_and_stale_generation_are_not_sent() {
    let (mut session, server) = fixture(vec![]).await;
    assert_eq!(
        session
            .input(
                VoiceCounter::FIRST.checked_next().unwrap(),
                VoiceEvent::ScreenShare(screen_fixture())
            )
            .await
            .unwrap_err(),
        VoiceErrorCode::Cancelled
    );
    let mut payload = screen_fixture();
    // A valid bounded PNG header with truncated image data must fail full decoding.
    payload.png.truncate(48);
    assert!(session.input(VoiceCounter::FIRST, VoiceEvent::ScreenShare(payload)).await.is_err());
    session.stop().await;
    assert_eq!(server.await.unwrap().len(), 1);
}
#[tokio::test]
async fn voice_task_17_realtime_screen_rate_is_bounded() {
    let (mut session, server) = fixture(vec![]).await;
    session.input(VoiceCounter::FIRST, VoiceEvent::ScreenShare(screen_fixture())).await.unwrap();
    assert_eq!(
        session
            .input(VoiceCounter::FIRST, VoiceEvent::ScreenShare(screen_fixture()))
            .await
            .unwrap_err(),
        VoiceErrorCode::RateLimited
    );
    assert_eq!(server.await.unwrap().len(), 2);
}
