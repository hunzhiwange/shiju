use super::*;
use crate::app::agent::channels::traits::{Channel, SendMessage};
use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn mock_server(
    responses: Vec<(&'static str, u16, &'static str)>,
) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let mut responses: VecDeque<_> = responses
        .into_iter()
        .map(|(path, status, body)| (path.to_string(), status, body.to_string()))
        .collect();

    let handle = tokio::spawn(async move {
        let mut seen = Vec::new();
        while let Some((expected_path, status, body)) = responses.pop_front() {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("")
                .to_string();
            seen.push(path.clone());
            assert_eq!(path, expected_path);

            let status_text = match status {
                200 => "OK",
                404 => "Not Found",
                _ => "Error",
            };
            let response = format!(
                "HTTP/1.1 {status} {status_text}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        }
        seen
    });

    (format!("http://{addr}"), handle)
}

fn channel(room_id: &str) -> MatrixChannel {
    MatrixChannel::new(
        "http://127.0.0.1:9".to_string(),
        "token".to_string(),
        room_id.to_string(),
        Vec::new(),
    )
}

fn channel_with_homeserver(homeserver: String, room_id: &str) -> MatrixChannel {
    MatrixChannel::new(homeserver, "token".to_string(), room_id.to_string(), Vec::new())
}

#[test]
fn channel_name_is_matrix() {
    assert_eq!(channel("!room:server").name(), "matrix");
}

#[tokio::test]
async fn send_returns_otk_recovery_error_when_conflict_is_set() {
    let ch = channel("!room:server");
    ch.otk_conflict_detected.store(true, Ordering::Relaxed);
    let message = SendMessage::new("hello", "!room:server");

    let err = ch.send(&message).await.expect_err("OTK conflict should stop send");

    assert!(err.to_string().contains("one-time key upload conflict"));
}

#[tokio::test]
async fn send_propagates_matrix_client_initialization_error() {
    let (homeserver, handle) =
        mock_server(vec![("/_matrix/client/v3/account/whoami", 500, r#"{"error":"failed"}"#)])
            .await;
    let ch = channel_with_homeserver(homeserver, "!room:server");
    let message = SendMessage::new("hello", "!room:server");

    let err = ch.send(&message).await.expect_err("whoami failure should stop send");

    assert!(err.to_string().contains("Matrix whoami failed"));
    assert_eq!(handle.await.unwrap(), vec!["/_matrix/client/v3/account/whoami".to_string()]);
}

#[tokio::test]
async fn health_check_fails_when_otk_conflict_was_detected() {
    let ch = channel("!room:server");
    ch.otk_conflict_detected.store(true, Ordering::Relaxed);

    assert!(!ch.health_check().await);
}

#[tokio::test]
async fn health_check_fails_for_invalid_room_reference() {
    let ch = channel("not-a-room-reference");

    assert!(!ch.health_check().await);
}

#[tokio::test]
async fn health_check_fails_when_room_support_check_errors() {
    let (homeserver, _handle) = mock_server(vec![(
        "/_matrix/client/v3/rooms/%21room%3Aserver/joined_members",
        403,
        r#"{"error":"forbidden"}"#,
    )])
    .await;
    let ch = channel_with_homeserver(homeserver, "!room:server");

    assert!(!ch.health_check().await);
}

#[tokio::test]
async fn health_check_fails_when_client_initialization_errors() {
    let (homeserver, _handle) = mock_server(vec![
        ("/_matrix/client/v3/rooms/%21room%3Aserver/joined_members", 200, r#"{"joined":{}}"#),
        (
            "/_matrix/client/v3/rooms/%21room%3Aserver/state/m.room.encryption",
            404,
            r#"{"errcode":"M_NOT_FOUND"}"#,
        ),
        ("/_matrix/client/v3/account/whoami", 500, r#"{"error":"failed"}"#),
    ])
    .await;
    let ch = channel_with_homeserver(homeserver, "!room:server");

    assert!(!ch.health_check().await);
}

#[tokio::test]
async fn health_check_succeeds_when_room_and_client_are_available() {
    let (homeserver, handle) = mock_server(vec![
        ("/_matrix/client/v3/rooms/%21room%3Aserver/joined_members", 200, r#"{"joined":{}}"#),
        (
            "/_matrix/client/v3/rooms/%21room%3Aserver/state/m.room.encryption",
            404,
            r#"{"errcode":"M_NOT_FOUND"}"#,
        ),
        (
            "/_matrix/client/v3/account/whoami",
            200,
            r#"{"user_id":"@bot:server","device_id":"DEVICE"}"#,
        ),
    ])
    .await;
    let ch = channel_with_homeserver(homeserver, "!room:server");

    assert!(ch.health_check().await);
    assert_eq!(handle.await.unwrap().len(), 3);
}

#[tokio::test]
async fn listen_returns_otk_recovery_error_when_conflict_is_set() {
    let ch = channel("!room:server");
    ch.otk_conflict_detected.store(true, Ordering::Relaxed);
    let (tx, _rx) = tokio::sync::mpsc::channel(1);

    let err = ch.listen(tx).await.expect_err("OTK conflict should stop listener");

    assert!(err.to_string().contains("one-time key upload conflict"));
}

#[tokio::test]
async fn listen_propagates_invalid_room_reference_error() {
    let ch = channel("not-a-room-reference");
    let (tx, _rx) = tokio::sync::mpsc::channel(1);

    let err = ch.listen(tx).await.expect_err("invalid room reference should stop listener");

    assert!(err.to_string().contains("must start with '!' (room ID) or '#' (room alias)"));
}
