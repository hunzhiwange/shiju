use super::*;
use std::time::Duration;
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

        if buf.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    String::from_utf8_lossy(&buf).into_owned()
}

async fn typing_server(
    status: u16,
) -> (String, mpsc::Receiver<String>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel(1);

    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_http_request(&mut stream).await;
        let reason = match status {
            204 => "No Content",
            500 => "Internal Server Error",
            _ => "OK",
        };
        let response =
            format!("HTTP/1.1 {status} {reason}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
        stream.write_all(response.as_bytes()).await.unwrap();
        let _ = tx.send(request).await;
    });

    (format!("http://{addr}"), rx, handle)
}

async fn recv_request(requests: &mut mpsc::Receiver<String>) -> String {
    tokio::time::timeout(Duration::from_secs(2), requests.recv())
        .await
        .expect("typing request should arrive")
        .expect("typing request channel should stay open")
}

#[test]
fn new_typing_handles_starts_empty() {
    let handles = new_typing_handles();
    assert!(handles.lock().is_empty());
}

#[tokio::test]
async fn stop_typing_without_existing_handle_is_noop() {
    let handles = new_typing_handles();

    stop_typing(&handles, "channel-1").await.expect("stop should be idempotent");

    assert!(handles.lock().is_empty());
}

#[tokio::test]
async fn start_typing_registers_handle_and_stop_removes_it() {
    let handles = new_typing_handles();

    start_typing(
        &handles,
        reqwest::Client::new(),
        "token".to_string(),
        "https://discord.com/api/v10".to_string(),
        "channel-1".to_string(),
    )
    .await
    .expect("typing task should start");

    assert!(handles.lock().contains_key("channel-1"));

    stop_typing(&handles, "channel-1").await.expect("typing task should stop");

    assert!(!handles.lock().contains_key("channel-1"));
}

#[tokio::test]
async fn start_typing_replaces_existing_handle_for_same_recipient() {
    let handles = new_typing_handles();

    start_typing(
        &handles,
        reqwest::Client::new(),
        "token-a".to_string(),
        "https://discord.com/api/v10".to_string(),
        "channel-1".to_string(),
    )
    .await
    .expect("first typing task should start");
    let first_handle = {
        let mut guard = handles.lock();
        guard.remove("channel-1").expect("first handle should be stored")
    };
    let first_id = first_handle.id();
    handles.lock().insert("channel-1".to_string(), first_handle);

    start_typing(
        &handles,
        reqwest::Client::new(),
        "token-b".to_string(),
        "https://discord.com/api/v10".to_string(),
        "channel-1".to_string(),
    )
    .await
    .expect("second typing task should replace first");

    let second_id = handles.lock().get("channel-1").expect("second handle").id();
    assert_ne!(first_id, second_id);

    stop_typing(&handles, "channel-1").await.expect("cleanup should stop replacement");
}

#[tokio::test]
async fn start_typing_posts_to_trimmed_api_base_with_authorization() {
    let (base_url, mut requests, server) = typing_server(204).await;
    let handles = new_typing_handles();

    start_typing(
        &handles,
        reqwest::Client::new(),
        "token-secret".to_string(),
        format!("{base_url}///"),
        "channel-1".to_string(),
    )
    .await
    .expect("typing task should start");

    let request = recv_request(&mut requests).await;
    assert!(request.starts_with("POST /channels/channel-1/typing "));
    assert!(request.to_ascii_lowercase().contains("authorization: bot token-secret"));

    server.await.expect("typing test server should finish");
    tokio::task::yield_now().await;
    stop_typing(&handles, "channel-1").await.expect("typing task should stop");
}

#[tokio::test]
async fn start_typing_keeps_handle_after_http_error_response() {
    let (base_url, mut requests, server) = typing_server(500).await;
    let handles = new_typing_handles();

    start_typing(
        &handles,
        reqwest::Client::new(),
        "token-secret".to_string(),
        base_url,
        "channel-1".to_string(),
    )
    .await
    .expect("typing task should start");

    let request = recv_request(&mut requests).await;
    assert!(request.starts_with("POST /channels/channel-1/typing "));
    assert!(handles.lock().contains_key("channel-1"));

    server.await.expect("typing test server should finish");
    stop_typing(&handles, "channel-1").await.expect("typing task should stop");
}
// 本文件验证 Discord 输入状态任务的启停、替换与请求发送行为。
