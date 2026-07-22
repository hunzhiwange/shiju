use std::time::Duration;

use super::client::{
    discord_messages_url, send_discord_message_json, send_discord_message_json_to_url,
    send_discord_message_with_files, send_discord_message_with_files_to_url,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

async fn read_http_request(stream: &mut tokio::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0_u8; 1024];

    loop {
        let n = stream.read(&mut tmp).await.unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);

        let Some(header_end) = buf.windows(4).position(|window| window == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&buf[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);

        if buf.len() >= header_end + 4 + content_length {
            break;
        }
    }

    String::from_utf8_lossy(&buf).into_owned()
}

async fn discord_server(
    responses: Vec<(u16, &'static str)>,
) -> (String, mpsc::Receiver<String>, tokio::task::JoinHandle<()>) {
    let responses = responses
        .into_iter()
        .map(|(status, body)| {
            let reason = match status {
                200 => "OK",
                400 => "Bad Request",
                401 => "Unauthorized",
                500 => "Internal Server Error",
                _ => "Error",
            };
            format!(
                "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            )
            .into_bytes()
        })
        .collect();

    discord_raw_server(responses).await
}

async fn discord_raw_server(
    responses: Vec<Vec<u8>>,
) -> (String, mpsc::Receiver<String>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel(responses.len().max(1));

    let handle = tokio::spawn(async move {
        for response in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_http_request(&mut stream).await;
            let _ = tx.send(request).await;
            stream.write_all(&response).await.unwrap();
        }
    });

    (format!("http://{addr}"), rx, handle)
}

fn request_body(request: &str) -> &str {
    request.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or_default()
}

fn truncated_error_response() -> Vec<u8> {
    b"HTTP/1.1 500 Internal Server Error\r\ncontent-type: text/plain\r\ncontent-length: 8\r\nconnection: close\r\n\r\nabc"
        .to_vec()
}

#[test]
fn discord_messages_url_trims_api_base_trailing_slashes() {
    assert_eq!(
        discord_messages_url("http://127.0.0.1:9///", "channel-1"),
        "http://127.0.0.1:9/channels/channel-1/messages"
    );
}

#[tokio::test]
async fn send_message_json_posts_content_and_authorization() {
    let (base_url, mut requests, server) = discord_server(vec![(200, "{}")]).await;
    let client = reqwest::Client::new();
    let url = discord_messages_url(&base_url, "channel-1");

    send_discord_message_json_to_url(&client, "token-secret", &url, "hello").await.unwrap();

    let request = requests.recv().await.unwrap();
    assert!(request.starts_with("POST /channels/channel-1/messages "));
    assert!(request.to_ascii_lowercase().contains("authorization: bot token-secret"));
    let body = serde_json::from_str::<serde_json::Value>(request_body(&request)).unwrap();
    assert_eq!(body["content"], "hello");
    server.await.unwrap();
}

#[tokio::test]
async fn send_message_json_reports_sanitized_error_response() {
    let (base_url, _requests, server) =
        discord_server(vec![(401, r#"{"message":"bad token=12345678TOKEN"}"#)]).await;
    let client = reqwest::Client::new();
    let url = discord_messages_url(&base_url, "channel-1");

    let error = send_discord_message_json_to_url(&client, "token-secret", &url, "hello")
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("Discord send message failed (401 Unauthorized)"));
    assert!(error.contains("[REDACTED]"));
    assert!(!error.contains("12345678TOKEN"));
    server.await.unwrap();
}

#[tokio::test]
async fn send_message_json_reports_response_body_read_failure() {
    let (base_url, _requests, server) = discord_raw_server(vec![truncated_error_response()]).await;
    let client = reqwest::Client::new();
    let url = discord_messages_url(&base_url, "channel-1");

    let error = send_discord_message_json_to_url(&client, "token-secret", &url, "hello")
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("<failed to read response body:"));
    server.await.unwrap();
}

#[tokio::test]
async fn send_message_json_uses_default_discord_endpoint_before_transport_error() {
    let proxy = reqwest::Proxy::all("http://127.0.0.1:9").unwrap();
    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_millis(100))
        .build()
        .unwrap();

    assert!(send_discord_message_json(&client, "token", "channel", "content").await.is_err());
}

#[tokio::test]
async fn send_message_with_files_posts_payload_and_file_part() {
    let tempdir = tempfile::tempdir().unwrap();
    let file = tempdir.path().join("hello.txt");
    std::fs::write(&file, "file-body").unwrap();
    let (base_url, mut requests, server) = discord_server(vec![(200, "{}")]).await;
    let client = reqwest::Client::new();
    let url = discord_messages_url(&base_url, "channel-1");

    send_discord_message_with_files_to_url(&client, "token-secret", &url, "with file", &[file])
        .await
        .unwrap();

    let request = requests.recv().await.unwrap();
    let body = request_body(&request);
    assert!(request.starts_with("POST /channels/channel-1/messages "));
    assert!(request.to_ascii_lowercase().contains("authorization: bot token-secret"));
    assert!(request.to_ascii_lowercase().contains("content-type: multipart/form-data"));
    assert!(body.contains(r#"name="payload_json""#));
    assert!(body.contains(r#""content""#));
    assert!(body.contains("with file"));
    assert!(body.contains(r#"name="files[0]""#));
    assert!(body.contains(r#"filename="hello.txt""#));
    assert!(body.contains("file-body"));
    server.await.unwrap();
}

#[tokio::test]
async fn send_message_with_files_reports_sanitized_error_response() {
    let tempdir = tempfile::tempdir().unwrap();
    let file = tempdir.path().join("hello.txt");
    std::fs::write(&file, "file-body").unwrap();
    let (base_url, _requests, server) =
        discord_server(vec![(500, r#"{"message":"bad token=12345678TOKEN"}"#)]).await;
    let client = reqwest::Client::new();
    let url = discord_messages_url(&base_url, "channel-1");

    let error =
        send_discord_message_with_files_to_url(&client, "token-secret", &url, "content", &[file])
            .await
            .unwrap_err()
            .to_string();

    assert!(error.contains("Discord send message with files failed (500 Internal Server Error)"));
    assert!(error.contains("[REDACTED]"));
    assert!(!error.contains("12345678TOKEN"));
    server.await.unwrap();
}

#[tokio::test]
async fn send_message_with_files_reports_response_body_read_failure() {
    let tempdir = tempfile::tempdir().unwrap();
    let file = tempdir.path().join("hello.txt");
    std::fs::write(&file, "file-body").unwrap();
    let (base_url, _requests, server) = discord_raw_server(vec![truncated_error_response()]).await;
    let client = reqwest::Client::new();
    let url = discord_messages_url(&base_url, "channel-1");

    let error =
        send_discord_message_with_files_to_url(&client, "token-secret", &url, "content", &[file])
            .await
            .unwrap_err()
            .to_string();

    assert!(error.contains("<failed to read response body:"));
    server.await.unwrap();
}

#[tokio::test]
async fn send_message_with_files_reports_missing_local_file_before_network() {
    let client = reqwest::Client::new();
    let tempdir = tempfile::tempdir().unwrap();
    let missing = tempdir.path().join("missing-attachment.txt");

    let error = send_discord_message_with_files(&client, "token", "channel", "content", &[missing])
        .await
        .unwrap_err()
        .to_string();

    assert!(error.contains("Discord attachment read failed"));
}
