use super::*;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message as Wire;
fn id(s: &str) -> VoiceId {
    s.to_owned().try_into().unwrap()
}
type Peer =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
struct Fixture {
    peer: Peer,
    commands: tokio::task::JoinHandle<Vec<Value>>,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
        self.commands.abort();
    }
}
async fn fixture() -> Fixture {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let commands = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
        ws.send(Wire::Text(
            json!({"type":"session.created","session":{"id":"service"}}).to_string().into(),
        ))
        .await
        .unwrap();
        let update: Value =
            serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
        let mut session = update["session"].clone();
        session["id"] = json!("service");
        ws.send(Wire::Text(json!({"type":"session.updated","session":session}).to_string().into()))
            .await
            .unwrap();
        let mut commands = vec![];
        while let Some(Ok(Wire::Text(text))) = ws.next().await {
            let command: Value = serde_json::from_str(&text).unwrap();
            if command["type"] == "response.create" {
                ws.send(Wire::Text(
                    json!({"type":"response.created","response":{"id":"r1"}}).to_string().into(),
                ))
                .await
                .unwrap();
            }
            if command["type"] == "response.create" {
                ws.send(Wire::Text(json!({"type":"response.output_item.added","response_id":"r1","item":{"id":"item1","type":"message"}}).to_string().into())).await.unwrap();
                ws.send(Wire::Text(json!({"type":"response.output_audio.delta","response_id":"r1","item_id":"item1","content_index":0,"delta":"AAAAAA=="}).to_string().into())).await.unwrap();
            }
            commands.push(command);
        }
        commands
    });
    let config = VoiceConfig { realtime_enabled: true, ..Default::default() };
    let upstream = RealtimeSession::connect(
        &base,
        "synthetic",
        reqwest::Client::builder().no_proxy(),
        &config,
        id("openai"),
        id("local"),
        TurnMode::Manual,
    )
    .await
    .unwrap();
    let upstream = Arc::new(tokio::sync::Mutex::new(Some(upstream)));
    let app = Router::new().route(
        "/stream",
        get(move |ws: WebSocketUpgrade| {
            let upstream = upstream.clone();
            async move {
                ws.on_upgrade(move |socket| async move {
                    bridge(
                        socket,
                        upstream.lock().await.take().unwrap(),
                        id("local"),
                        VoiceName::Marin,
                    )
                    .await;
                })
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}/stream", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let (mut peer, _) = tokio_tungstenite::connect_async(url).await.unwrap();
    let ready =
        VoiceFrame::parse(peer.next().await.unwrap().unwrap().into_data().as_ref()).unwrap();
    assert!(matches!(ready.event, VoiceEvent::SessionReady { .. }));
    Fixture { peer, commands, server }
}
async fn send(peer: &mut Peer, session: &str, seq: u64, event: VoiceEvent) {
    let frame = VoiceFrame {
        voice_session_id: id(session),
        generation: VoiceCounter::FIRST,
        seq: seq.try_into().unwrap(),
        event,
    };
    peer.send(Wire::Text(serde_json::to_string(&frame).unwrap().into())).await.unwrap();
}
#[tokio::test]
async fn voice_task_10_voice_stream_rejects_cross_session_cancel() {
    let mut source = fixture().await;
    let mut target = fixture().await;
    send(&mut source.peer, "foreign", 1, VoiceEvent::SessionStop {}).await;
    // A foreign target must be rejected at the reader, rather than reaching its
    // priority channel. Assert the real channel closes without delivering it.
    let (media, mut media_rx) = mpsc::channel(QUEUE);
    let (control, mut control_rx) = mpsc::channel(QUEUE);
    let closed = CancellationToken::new();
    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let app = Router::new().route(
        "/read",
        get(move |ws: WebSocketUpgrade| {
            let media = media.clone();
            let control = control.clone();
            let closed = closed.clone();
            let tx = tx.clone();
            async move {
                ws.on_upgrade(move |socket| async move {
                    let (_, reader) = socket.split();
                    read_peer(reader, id("local"), media, control, closed).await;
                    let _ = tx.lock().unwrap().take().unwrap().send(());
                })
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}/read", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let (mut peer, _) = tokio_tungstenite::connect_async(url).await.unwrap();
    send(&mut peer, "foreign", 1, VoiceEvent::SessionStop {}).await;
    peer.close(None).await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), rx).await.unwrap().unwrap();
    assert!(
        control_rx.try_recv().is_err(),
        "cross-session cancel reached the upstream control queue"
    );
    assert!(media_rx.try_recv().is_err());
    server.abort();
    send(
        &mut target.peer,
        "local",
        1,
        VoiceEvent::AudioAppend(AudioPayload {
            audio: "A".repeat(6400),
            samples_per_channel: 2400,
        }),
    )
    .await;
    send(&mut target.peer, "local", 2, VoiceEvent::InputCommit {}).await;
    let response = tokio::time::timeout(Duration::from_secs(3), target.peer.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(
        matches!(
            VoiceFrame::parse(response.into_data().as_ref()).unwrap().event,
            VoiceEvent::ResponseStarted { .. }
        ),
        "other connection must remain usable"
    );
    let _ = tokio::time::timeout(Duration::from_secs(3), target.peer.next()).await.unwrap();
    target.peer.close(None).await.unwrap();
    let commands =
        tokio::time::timeout(Duration::from_secs(3), &mut target.commands).await.unwrap().unwrap();
    assert!(commands.iter().any(|c| c["type"] == "response.create"));
}
#[tokio::test]
async fn voice_task_10_voice_stream_disconnect_closes_upstream() {
    let mut fixture = fixture().await;
    fixture.peer.close(None).await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(3), &mut fixture.commands).await.is_ok(),
        "desktop disconnect must close the service socket"
    );
}
#[tokio::test]
async fn voice_task_10_voice_stream_bidirectional_audio_and_response() {
    let mut fixture = fixture().await;
    use base64::Engine as _;
    let audio = base64::engine::general_purpose::STANDARD.encode(vec![0u8; 4800]);
    send(
        &mut fixture.peer,
        "local",
        1,
        VoiceEvent::AudioAppend(AudioPayload { audio, samples_per_channel: 2400 }),
    )
    .await;
    send(&mut fixture.peer, "local", 2, VoiceEvent::InputCommit {}).await;
    let frame = tokio::time::timeout(Duration::from_secs(3), fixture.peer.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let frame = VoiceFrame::parse(frame.into_data().as_ref()).unwrap();
    assert_eq!(frame.seq.get(), 2);
    assert!(matches!(frame.event, VoiceEvent::ResponseStarted { .. }));
    let audio = tokio::time::timeout(Duration::from_secs(3), fixture.peer.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(matches!(
        VoiceFrame::parse(audio.into_data().as_ref()).unwrap().event,
        VoiceEvent::AudioDelta { samples_per_channel: 2, .. }
    ));
    fixture.peer.close(None).await.unwrap();
    let commands =
        tokio::time::timeout(Duration::from_secs(3), &mut fixture.commands).await.unwrap().unwrap();
    assert!(commands.iter().any(|v| v["type"] == "input_audio_buffer.append"));
    assert!(commands.iter().any(|v| v["type"] == "input_audio_buffer.commit"));
    assert!(commands.iter().any(|v| v["type"] == "response.create"));
}

#[test]
fn voice_task_10_reservation_rejects_foreign_tenant_principal_and_desktop() {
    let channels = Channels::default();
    let tenant = TenantScope::new("voice10", "owner").unwrap();
    let reservation = Reservation {
        tenant: tenant.clone(),
        principal: [1; 32],
        connection: id("desktop-a"),
        start: VoiceSessionStart {
            request_id: id("req"),
            chat_session_id: id("chat"),
            generation: VoiceCounter::FIRST,
            provider_id: id("openai"),
            turn_mode: TurnMode::Manual,
        },
        config: VoiceConfig::default(),
        created: Instant::now(),
        _permit: channels.capacity.clone().try_acquire_owned().unwrap(),
    };
    channels.pending.lock().unwrap().insert("local".into(), reservation);
    assert!(matches!(
        channels.take(
            &id("local"),
            &TenantScope::new("voice10", "other").unwrap(),
            [1; 32],
            &id("desktop-a")
        ),
        Err(VoiceErrorCode::Forbidden)
    ));
    assert!(matches!(
        channels.take(&id("local"), &tenant, [2; 32], &id("desktop-a")),
        Err(VoiceErrorCode::Forbidden)
    ));
    assert!(matches!(
        channels.take(&id("local"), &tenant, [1; 32], &id("desktop-b")),
        Err(VoiceErrorCode::Forbidden)
    ));
    assert!(channels.take(&id("local"), &tenant, [1; 32], &id("desktop-a")).is_ok());
    assert!(
        matches!(
            channels.take(&id("local"), &tenant, [1; 32], &id("desktop-a")),
            Err(VoiceErrorCode::Forbidden)
        ),
        "upgrade must be one-use"
    );
    assert_eq!(channels.capacity.available_permits(), 4);
}

#[tokio::test]
async fn voice_task_10_bounded_media_preserves_control_priority() {
    let (media, mut media_rx) = mpsc::channel(QUEUE);
    let (control, mut control_rx) = mpsc::channel(QUEUE);
    let closed = CancellationToken::new();
    let (done, wait) = tokio::sync::oneshot::channel();
    let done = Arc::new(Mutex::new(Some(done)));
    let app = Router::new().route(
        "/read",
        get(move |ws: WebSocketUpgrade| {
            let media = media.clone();
            let control = control.clone();
            let closed = closed.clone();
            let done = done.clone();
            async move {
                ws.on_upgrade(move |socket| async move {
                    let (_, reader) = socket.split();
                    read_peer(reader, id("local"), media, control, closed).await;
                    let _ = done.lock().unwrap().take().unwrap().send(());
                })
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}/read", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let (mut peer, _) = tokio_tungstenite::connect_async(url).await.unwrap();
    for seq in 1..=QUEUE {
        send(
            &mut peer,
            "local",
            seq as u64,
            VoiceEvent::AudioAppend(AudioPayload { audio: "AAA=".into(), samples_per_channel: 1 }),
        )
        .await;
    }
    send(&mut peer, "local", QUEUE as u64 + 1, VoiceEvent::SessionStop {}).await;
    tokio::time::timeout(Duration::from_secs(3), wait).await.unwrap().unwrap();
    assert!(
        matches!(control_rx.try_recv().unwrap().event, VoiceEvent::SessionStop {}),
        "stop must bypass full media queue"
    );
    assert_eq!(media_rx.len(), QUEUE);
    for _ in 0..QUEUE {
        assert!(media_rx.try_recv().is_ok());
    }
    task.abort();
}

#[tokio::test]
async fn voice_task_10_upstream_failure_closes_desktop() {
    let mut fixture = fixture().await;
    fixture.commands.abort();
    let frame = tokio::time::timeout(Duration::from_secs(3), fixture.peer.next()).await.unwrap();
    assert!(!matches!(frame, Some(Ok(Wire::Text(_)))), "upstream failure must end desktop socket");
}

#[test]
fn voice_task_10_slow_consumer_has_one_second_shared_pcm_budget() {
    let frames = Arc::new(Semaphore::new(50));
    let samples = Arc::new(Semaphore::new(24_000));
    let audio = || VoiceFrame {
        voice_session_id: id("local"),
        generation: VoiceCounter::FIRST,
        seq: VoiceCounter::FIRST,
        event: VoiceEvent::AudioDelta {
            response_id: id("r"),
            item_id: id("i"),
            content_index: 0,
            audio: "A".repeat(6400),
            samples_per_channel: 2400,
        },
    };
    let mut held = Vec::new();
    for _ in 0..10 {
        held.push(bounded_frame(audio(), &frames, &samples).unwrap());
    }
    assert!(
        matches!(bounded_frame(audio(), &frames, &samples), Err(VoiceErrorCode::Backpressure)),
        "writer/queue/batch may not jointly buffer more than one second"
    );
    held.pop();
    assert!(bounded_frame(audio(), &frames, &samples).is_ok());
    drop(held);
    assert_eq!(samples.available_permits(), 24_000);
    assert_eq!(frames.available_permits(), 50);
}
