use std::future::Future;
use std::net::TcpListener as StdTcpListener;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

use super::*;

async fn spawn_gateway_server<F, Fut>(handler: F) -> (String, JoinHandle<()>)
where
    F: FnOnce(WebSocketStream<tokio::net::TcpStream>) -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind local websocket server");
    let addr = listener.local_addr().expect("read local websocket addr");
    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept websocket client");
        let websocket = tokio_tungstenite::accept_async(stream).await.expect("accept websocket");
        handler(websocket).await;
    });

    (format!("ws://{addr}"), handle)
}

async fn read_http_request(stream: &mut tokio::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0_u8; 1024];

    loop {
        let n = stream.read(&mut tmp).await.expect("read http request");
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

async fn spawn_gateway_http_server(
    body: &'static str,
) -> (String, mpsc::Receiver<String>, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind local http server");
    let addr = listener.local_addr().expect("read local http addr");
    let (tx, rx) = mpsc::channel(1);

    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept http client");
        let request = read_http_request(&mut stream).await;
        tx.send(request).await.expect("send captured request");
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.expect("write http response");
    });

    (format!("http://{addr}"), rx, handle)
}

fn unused_local_addr() -> std::net::SocketAddr {
    let listener = StdTcpListener::bind("127.0.0.1:0").expect("bind unused local port");
    listener.local_addr().expect("read unused local addr")
}

#[tokio::test]
async fn fetch_gateway_url_returns_error_when_api_cannot_be_reached() {
    let proxy_url = format!("http://{}", unused_local_addr());
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::all(proxy_url).expect("build proxy"))
        .timeout(Duration::from_millis(500))
        .build()
        .expect("build client");

    let error = fetch_gateway_url(&client, "bot-token", "http://127.0.0.1").await.unwrap_err();

    assert!(!error.to_string().is_empty());
}

#[tokio::test]
async fn fetch_gateway_url_reads_gateway_endpoint_from_api_response() {
    let (base_url, mut requests, handle) =
        spawn_gateway_http_server(r#"{"url":"wss://gateway.example.test"}"#).await;
    let client = reqwest::Client::new();

    let url = fetch_gateway_url(&client, "bot-token", &format!("{base_url}/")).await.unwrap();

    assert_eq!(url, "wss://gateway.example.test");
    let request = requests.recv().await.expect("captured request");
    assert!(request.starts_with("GET /gateway/bot "));
    assert!(request.to_ascii_lowercase().contains("authorization: bot bot-token"));
    handle.await.expect("http server task finished");
}

#[tokio::test]
async fn fetch_gateway_url_defaults_when_api_response_omits_url() {
    let (base_url, _requests, handle) =
        spawn_gateway_http_server(r#"{"session_start_limit":{}}"#).await;
    let client = reqwest::Client::new();

    let url = fetch_gateway_url(&client, "bot-token", &base_url).await.unwrap();

    assert_eq!(url, "wss://gateway.discord.gg");
    handle.await.expect("http server task finished");
}

#[test]
fn gateway_url_from_response_returns_api_url() {
    let response = serde_json::json!({"url": "wss://gateway.example.test"});

    assert_eq!(gateway_url_from_response(&response), "wss://gateway.example.test");
}

#[test]
fn gateway_url_from_response_defaults_when_url_is_missing() {
    let response = serde_json::json!({"session_start_limit": {}});

    assert_eq!(gateway_url_from_response(&response), "wss://gateway.discord.gg");
}

#[tokio::test]
async fn connect_gateway_splits_local_websocket_stream() {
    let (url, handle) = spawn_gateway_server(|websocket| async move {
        let (mut server_write, mut server_read) = websocket.split();
        server_write.send(Message::Text("gateway-ready".into())).await.expect("send server frame");
        let message = server_read.next().await.expect("read client frame").expect("client frame");
        assert_eq!(message.to_text().expect("client text"), "client-ready");
    })
    .await;

    let (mut write, mut read) = connect_gateway(&url).await.expect("connect local gateway");
    write.send(Message::Text("client-ready".into())).await.expect("send client frame");
    let message = read.next().await.expect("read server frame").expect("server frame");

    assert_eq!(message.to_text().expect("server text"), "gateway-ready");
    handle.await.expect("server task finished");
}

#[tokio::test]
async fn connect_gateway_rejects_invalid_url() {
    let error = connect_gateway("not-a-websocket-url").await.unwrap_err();

    assert!(!error.to_string().is_empty());
}

#[tokio::test]
async fn read_gateway_hello_returns_configured_heartbeat_interval() {
    let (url, handle) = spawn_gateway_server(|mut websocket| async move {
        websocket
            .send(Message::Text(r#"{"op":10,"d":{"heartbeat_interval":1234}}"#.into()))
            .await
            .expect("send hello");
    })
    .await;

    let (_write, mut read) = connect_gateway(&url).await.expect("connect local gateway");
    let interval = read_gateway_hello(&mut read).await.expect("read hello");

    assert_eq!(interval, 1234);
    handle.await.expect("server task finished");
}

#[tokio::test]
async fn read_gateway_hello_uses_default_interval_when_missing() {
    let (url, handle) = spawn_gateway_server(|mut websocket| async move {
        websocket.send(Message::Text(r#"{"op":10,"d":{}}"#.into())).await.expect("send hello");
    })
    .await;

    let (_write, mut read) = connect_gateway(&url).await.expect("connect local gateway");
    let interval = read_gateway_hello(&mut read).await.expect("read hello");

    assert_eq!(interval, 41250);
    handle.await.expect("server task finished");
}

#[tokio::test]
async fn read_gateway_hello_rejects_invalid_json() {
    let (url, handle) = spawn_gateway_server(|mut websocket| async move {
        websocket.send(Message::Text("not-json".into())).await.expect("send invalid hello");
    })
    .await;

    let (_write, mut read) = connect_gateway(&url).await.expect("connect local gateway");
    let error = read_gateway_hello(&mut read).await.unwrap_err();

    assert!(!error.to_string().is_empty());
    handle.await.expect("server task finished");
}

#[tokio::test]
async fn read_gateway_hello_returns_error_when_connection_closes_immediately() {
    let (url, handle) = spawn_gateway_server(|mut websocket| async move {
        websocket.close(None).await.expect("close websocket");
    })
    .await;

    let (_write, mut read) = connect_gateway(&url).await.expect("connect local gateway");
    let error = read_gateway_hello(&mut read).await.unwrap_err();

    assert_eq!(error.to_string(), "No hello");
    handle.await.expect("server task finished");
}

#[tokio::test]
async fn send_identify_writes_discord_identify_payload() {
    let (url, handle) = spawn_gateway_server(|mut websocket| async move {
        let message = websocket.next().await.expect("read identify frame").expect("identify frame");
        let payload: Value = serde_json::from_str(message.to_text().expect("identify text"))
            .expect("parse identify payload");

        assert_eq!(payload["op"], 2);
        assert_eq!(payload["d"]["token"], "bot-token");
        assert_eq!(payload["d"]["intents"], 37377);
        assert_eq!(payload["d"]["properties"]["os"], "linux");
        assert_eq!(payload["d"]["properties"]["browser"], "vibewindow");
        assert_eq!(payload["d"]["properties"]["device"], "vibewindow");
    })
    .await;

    let (mut write, _read) = connect_gateway(&url).await.expect("connect local gateway");

    send_identify(&mut write, "bot-token").await.expect("send identify");
    handle.await.expect("server task finished");
}

#[test]
fn gateway_connection_type_remains_available_to_gateway_module() {
    assert!(std::mem::size_of::<GatewayConnection>() > 0);
}
// 本文件验证 Discord Gateway 地址发现、连接拆分与握手心跳处理。
