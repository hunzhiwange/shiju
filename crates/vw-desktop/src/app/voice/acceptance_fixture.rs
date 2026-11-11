//! Synthetic gateway boundary; exercises the real HTTP/WS client, no devices or credentials.
use iced::futures::{SinkExt, StreamExt};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use vw_gateway_client::{GatewayEndpoint, VoiceStream, vw_api_types::voice::*};

pub(crate) async fn roundtrip() -> Result<Vec<(TranscriptRole, String)>, Box<dyn std::error::Error>>
{
    tokio::time::timeout(Duration::from_secs(8), exchange()).await?
}
async fn exchange() -> Result<Vec<(TranscriptRole, String)>, Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr()?.port());
    let start = VoiceSessionStart {
        request_id: id("request"),
        chat_session_id: id("acceptance"),
        provider_id: id("fixture"),
        generation: VoiceCounter::FIRST,
        turn_mode: TurnMode::Manual,
    };
    let server = tokio::spawn(async move {
        let (mut http, _) = listener.accept().await.unwrap();
        let mut header = vec![];
        let mut byte = [0];
        while !header.ends_with(b"\r\n\r\n") {
            http.read_exact(&mut byte).await.unwrap();
            header.push(byte[0]);
            assert!(header.len() < 8192);
        }
        let header = String::from_utf8(header).unwrap();
        assert!(header.starts_with("POST /api/voice/sessions HTTP/1.1"));
        let length: usize = header
            .lines()
            .find_map(|line| line.strip_prefix("content-length: "))
            .unwrap()
            .parse()
            .unwrap();
        assert!(length < 8192);
        let mut body = vec![0; length];
        http.read_exact(&mut body).await.unwrap();
        let request: VoiceSessionStart = serde_json::from_slice(&body).unwrap();
        assert_eq!(request.chat_session_id, id("acceptance"));
        let response = VoiceSessionStarted {
            request_id: request.request_id,
            chat_session_id: request.chat_session_id,
            generation: VoiceCounter::FIRST,
            voice_session_id: id("fixture-session"),
            stream_path: "/api/voice/sessions/fixture-session/stream".into(),
            format: AudioFormat::default(),
            voice: VoiceName::Marin,
            expires_in_ms: MAX_SESSION_MS,
        };
        let body = serde_json::to_string(&response).unwrap();
        http.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        drop(http);
        let (tcp, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(tcp).await.unwrap();
        for (index, event) in [
            VoiceEvent::SessionReady { format: AudioFormat::default(), voice: VoiceName::Marin },
            VoiceEvent::TranscriptFinal {
                role: TranscriptRole::User,
                item_id: id("zh"),
                content_index: 0,
                text: "中文动态字幕 🌱".into(),
            },
            VoiceEvent::TranscriptFinal {
                role: TranscriptRole::Assistant,
                item_id: id("en"),
                content_index: 0,
                text: "English caption".into(),
            },
        ]
        .into_iter()
        .enumerate()
        {
            let frame = VoiceFrame {
                voice_session_id: id("fixture-session"),
                generation: VoiceCounter::FIRST,
                seq: ((index + 1) as u64).try_into().unwrap(),
                event,
            };
            ws.send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::to_string(&frame).unwrap().into(),
            ))
            .await
            .unwrap();
        }
        // Client teardown must release the upgraded socket, not just the local state.
        let began = Instant::now();
        while let Some(message) = ws.next().await {
            match message {
                Ok(tokio_tungstenite::tungstenite::Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
        began.elapsed()
    });
    let mut stream = VoiceStream::connect(&endpoint, &start).await?;
    let mut captions = vec![];
    while captions.len() < 2 {
        if let VoiceEvent::TranscriptFinal { role, text, .. } = stream.next().await?.event {
            captions.push((role, text));
        }
    }
    drop(stream);
    let released = server.await?;
    assert!(released < Duration::from_secs(2), "WS release exceeded deadline");
    eprintln!("VOICE_METRICS captions=2 websocket_release_ms={}", released.as_millis());
    Ok(captions)
}
fn id(value: &str) -> VoiceId {
    value.to_owned().try_into().unwrap()
}
