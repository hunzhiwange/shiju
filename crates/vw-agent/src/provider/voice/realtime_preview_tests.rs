use super::realtime::RealtimeSession;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;
use vw_config_types::voice::VoiceConfig;

#[tokio::test]
async fn voice_task_13_voice_preview_does_not_capture_microphone() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(socket).await.unwrap();
        ws.send(Message::Text(
            json!({"type":"session.created","session":{"id":"preview"}}).to_string().into(),
        ))
        .await
        .unwrap();
        let Message::Text(text) = ws.next().await.unwrap().unwrap() else {
            panic!("session update");
        };
        let update: Value = serde_json::from_str(&text).unwrap();
        ws.send(Message::Text(json!({"type":"session.updated","session":{"id":"preview", "type":"realtime",
            "model":"gpt-realtime-2.1", "output_modalities":["audio"], "tools":update["session"]["tools"],
            "tool_choice":update["session"]["tool_choice"],"audio": update["session"]["audio"]}}).to_string().into())).await.unwrap();
        let command = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next()).await;
        (
            update,
            command
                .ok()
                .flatten()
                .and_then(Result::ok)
                .and_then(|m| m.into_text().ok())
                .and_then(|s| serde_json::from_str::<Value>(&s).ok()),
        )
    });
    let config = VoiceConfig {
        realtime_enabled: true,
        prompt: "private user context".into(),
        dictionary: vec!["private".into()],
        ..Default::default()
    };
    let mut preview = RealtimeSession::connect_preview(
        &base,
        "fixture",
        reqwest::Client::builder().no_proxy(),
        &config,
        "openai".to_owned().try_into().unwrap(),
        "preview".to_owned().try_into().unwrap(),
    )
    .await
    .unwrap();
    let (update, command) = server.await.unwrap();
    assert_eq!(update["session"]["tools"], json!([]));
    assert_eq!(update["session"]["tool_choice"], "none");
    assert_eq!(update["session"]["audio"]["input"]["turn_detection"], Value::Null);
    assert!(update["session"]["audio"]["input"]["transcription"].get("prompt").is_none());
    assert!(update["session"]["audio"]["input"]["transcription"].get("keywords").is_none());
    let command = command.expect("preview must generate without microphone input");
    assert_eq!(command["type"], "response.create");
    assert_eq!(command["response"]["conversation"], "none");
    assert_eq!(command["response"]["input"], json!([]));
    assert_eq!(command["response"]["tools"], json!([]));
    preview.stop().await;
}
