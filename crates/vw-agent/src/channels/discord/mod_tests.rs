// 本文件覆盖 Discord 渠道的收发主流程、网关交互与错误路径。

use super::*;
use crate::app::agent::channels::traits::Channel;
use crate::app::agent::channels::traits::SendMessage;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

struct TestHttpServer {
    base_url: String,
    requests: mpsc::Receiver<String>,
    handle: tokio::task::JoinHandle<()>,
}

impl TestHttpServer {
    async fn spawn(responses: Vec<(u16, String)>) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, requests) = mpsc::channel(responses.len().max(1));

        let handle = tokio::spawn(async move {
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let request = read_http_request(&mut stream).await;
                let _ = tx.send(request).await;
                let response = http_response(status, &body);
                stream.write_all(response.as_bytes()).await.unwrap();
            }
        });

        Self { base_url: format!("http://{addr}"), requests, handle }
    }

    async fn spawn_raw(responses: Vec<Vec<u8>>) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, requests) = mpsc::channel(responses.len().max(1));

        let handle = tokio::spawn(async move {
            for response in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let request = read_http_request(&mut stream).await;
                let _ = tx.send(request).await;
                stream.write_all(&response).await.unwrap();
            }
        });

        Self { base_url: format!("http://{addr}"), requests, handle }
    }

    async fn next_request(&mut self) -> String {
        timeout(Duration::from_secs(2), self.requests.recv())
            .await
            .expect("request should arrive")
            .expect("request channel should stay open")
    }
}

impl Drop for TestHttpServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

fn response(status: u16, body: impl Into<String>) -> (u16, String) {
    (status, body.into())
}

fn http_response(status: u16, body: &str) -> String {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        500 => "Internal Server Error",
        _ => "Error",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
}

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

fn request_body(request: &str) -> &str {
    request.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or_default()
}

fn request_json_body(request: &str) -> serde_json::Value {
    serde_json::from_str(request_body(request)).unwrap()
}

// 渠道夹具注入本地 API 与网关地址，使完整收发流程离线可控。
fn test_channel(base_url: &str) -> DiscordChannel {
    DiscordChannel::new("token".to_string(), None, vec!["*".to_string()], false, false)
        .with_api_base(base_url.to_string())
}

async fn spawn_gateway(
    events: Vec<serde_json::Value>,
) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        ws.send(Message::Text(
            json!({"op":10,"d":{"heartbeat_interval":60_000}}).to_string().into(),
        ))
        .await
        .unwrap();

        let mut incoming = Vec::new();
        if let Some(Ok(message)) = ws.next().await {
            incoming.push(message.to_string());
        }

        for event in events {
            ws.send(Message::Text(event.to_string().into())).await.unwrap();
        }
        let _ = ws.close(None).await;
        incoming
    });

    (format!("ws://{addr}"), handle)
}

async fn spawn_gateway_for_interval_heartbeat() -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        ws.send(Message::Text(json!({"op":10,"d":{"heartbeat_interval":1}}).to_string().into()))
            .await
            .unwrap();

        let mut incoming = Vec::new();
        for _ in 0..2 {
            if let Some(Ok(message)) = ws.next().await {
                incoming.push(message.to_string());
            }
        }
        let _ = ws.close(None).await;
        incoming
    });

    (format!("ws://{addr}"), handle)
}

async fn spawn_gateway_for_server_heartbeat_request()
-> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        ws.send(Message::Text(
            json!({"op":10,"d":{"heartbeat_interval":60_000}}).to_string().into(),
        ))
        .await
        .unwrap();

        let mut incoming = Vec::new();
        if let Some(Ok(message)) = ws.next().await {
            incoming.push(message.to_string());
        }

        ws.send(Message::Text(json!({"op":1,"s":42}).to_string().into())).await.unwrap();

        if let Some(Ok(message)) = ws.next().await {
            incoming.push(message.to_string());
        }

        let _ = ws.close(None).await;
        incoming
    });

    (format!("ws://{addr}"), handle)
}

async fn listen_with_gateway_events(events: Vec<serde_json::Value>) {
    let (gateway_url, _gateway_handle) = spawn_gateway(events).await;
    let mut server =
        TestHttpServer::spawn(vec![response(200, json!({"url": gateway_url}).to_string())]).await;
    let channel = test_channel(&server.base_url);
    let (tx, _rx) = mpsc::channel(1);

    channel.listen(tx).await.unwrap();

    let gateway_request = server.next_request().await;
    assert!(gateway_request.starts_with("GET /gateway/bot "));
}

#[test]
fn constructor_and_builders_preserve_state() {
    let channel = DiscordChannel::new(
        "token".to_string(),
        Some("guild-1".to_string()),
        vec!["user-1".to_string()],
        true,
        true,
    )
    .with_group_reply_allowed_senders(vec![
        " user-2 ".to_string(),
        "user-1".to_string(),
        "user-1".to_string(),
    ])
    .with_workspace_dir(PathBuf::from("/tmp/discord-workspace"))
    .with_api_base("http://127.0.0.1:1///".to_string());

    assert_eq!(channel.name(), "discord");
    assert_eq!(channel.bot_token, "token");
    assert_eq!(channel.guild_id.as_deref(), Some("guild-1"));
    assert_eq!(channel.allowed_users, vec!["user-1"]);
    assert!(channel.listen_to_bots);
    assert!(channel.mention_only);
    assert_eq!(channel.group_reply_allowed_sender_ids, vec!["user-1", "user-2"]);
    assert_eq!(channel.workspace_dir.as_deref(), Some(Path::new("/tmp/discord-workspace")));
    assert_eq!(channel.discord_api_base_url(), "http://127.0.0.1:1");
    assert!(channel.typing_handles.lock().is_empty());
}

#[tokio::test]
async fn send_posts_text_to_configured_api_base() {
    let mut server = TestHttpServer::spawn(vec![response(200, "{}")]).await;
    let channel = test_channel(&server.base_url);

    channel.send(&SendMessage::new("hello", "channel-1")).await.unwrap();

    let request = server.next_request().await;
    assert!(request.starts_with("POST /channels/channel-1/messages "));
    assert!(request.to_ascii_lowercase().contains("authorization: bot token"));
    assert_eq!(request_json_body(&request)["content"], "hello");
}

#[tokio::test]
async fn send_reports_api_rejection() {
    let mut server = TestHttpServer::spawn(vec![response(500, r#"{"message":"rejected"}"#)]).await;
    let channel = test_channel(&server.base_url);

    let error =
        channel.send(&SendMessage::new("hello", "channel-1")).await.unwrap_err().to_string();

    let request = server.next_request().await;
    assert!(request.starts_with("POST /channels/channel-1/messages "));
    assert!(error.contains("Discord send message failed (500 Internal Server Error)"));
    assert!(error.contains("rejected"));
}

#[tokio::test]
async fn send_inlines_remote_urls_and_unresolved_local_markers() {
    let mut server = TestHttpServer::spawn(vec![response(200, "{}")]).await;
    let channel = test_channel(&server.base_url);

    channel
        .send(&SendMessage::new(
            "body [IMAGE:https://cdn.test/a.png] [FILE:missing.txt]",
            "channel-1",
        ))
        .await
        .unwrap();

    let request = server.next_request().await;
    assert_eq!(
        request_json_body(&request)["content"],
        "body\nhttps://cdn.test/a.png\n[DOCUMENT:missing.txt]"
    );
}

#[tokio::test]
async fn send_uploads_first_ten_local_files() {
    let tempdir = tempfile::tempdir().unwrap();
    let mut markers = Vec::new();
    for idx in 0..11 {
        let filename = format!("file-{idx}.txt");
        std::fs::write(tempdir.path().join(&filename), format!("body-{idx}")).unwrap();
        markers.push(format!("[FILE:{filename}]"));
    }

    let mut server = TestHttpServer::spawn(vec![response(200, "{}")]).await;
    let channel = test_channel(&server.base_url).with_workspace_dir(tempdir.path().to_path_buf());

    channel.send(&SendMessage::new(markers.join(" "), "channel-1")).await.unwrap();

    let request = server.next_request().await;
    let body = request_body(&request);
    assert!(request.to_ascii_lowercase().contains("content-type: multipart/form-data"));
    assert!(body.contains(r#"name="files[0]""#));
    assert!(body.contains(r#"filename="file-0.txt""#));
    assert!(body.contains(r#"name="files[9]""#));
    assert!(!body.contains(r#"name="files[10]""#));
}

#[tokio::test]
async fn send_splits_long_messages() {
    let mut server = TestHttpServer::spawn(vec![response(200, "{}"), response(200, "{}")]).await;
    let channel = test_channel(&server.base_url);
    let content = "a".repeat(message_split::DISCORD_MAX_MESSAGE_LENGTH + 1);

    channel.send(&SendMessage::new(content, "channel-1")).await.unwrap();

    let first = server.next_request().await;
    let second = server.next_request().await;
    assert_eq!(
        request_json_body(&first)["content"].as_str().unwrap().chars().count(),
        message_split::DISCORD_MAX_MESSAGE_LENGTH
    );
    assert_eq!(request_json_body(&second)["content"], "a");
}

#[tokio::test]
async fn health_check_uses_configured_api_base() {
    let mut server = TestHttpServer::spawn(vec![response(200, "{}"), response(500, "{}")]).await;
    let channel = test_channel(&server.base_url);

    assert!(channel.health_check().await);
    assert!(!channel.health_check().await);

    let ok_request = server.next_request().await;
    let fail_request = server.next_request().await;
    assert!(ok_request.starts_with("GET /users/@me "));
    assert!(fail_request.starts_with("GET /users/@me "));
}

#[tokio::test]
async fn health_check_returns_false_on_transport_error() {
    let channel = test_channel("http://127.0.0.1:9");

    assert!(!channel.health_check().await);
}

#[tokio::test]
async fn reaction_methods_use_configured_api_base() {
    let mut server = TestHttpServer::spawn(vec![
        response(204, ""),
        response(500, r#"{"message":"bad token=12345678TOKEN"}"#),
    ])
    .await;
    let channel = test_channel(&server.base_url);

    channel.add_reaction("channel-1", "discord_message-1", "👌").await.unwrap();
    let error =
        channel.remove_reaction("channel-1", "message-1", "👌").await.unwrap_err().to_string();

    let add_request = server.next_request().await;
    let remove_request = server.next_request().await;
    assert!(
        add_request
            .starts_with("PUT /channels/channel-1/messages/message-1/reactions/%F0%9F%91%8C/@me ")
    );
    assert!(add_request.to_ascii_lowercase().contains("content-length: 0"));
    assert!(
        remove_request.starts_with(
            "DELETE /channels/channel-1/messages/message-1/reactions/%F0%9F%91%8C/@me "
        )
    );
    assert!(error.contains("Discord remove reaction failed (500 Internal Server Error)"));
    assert!(error.contains("[REDACTED]"));
    assert!(!error.contains("12345678TOKEN"));
}

#[tokio::test]
async fn reaction_methods_report_response_body_read_failure() {
    let truncated =
        b"HTTP/1.1 500 Internal Server Error\r\ncontent-type: text/plain\r\ncontent-length: 8\r\nconnection: close\r\n\r\nabc"
            .to_vec();
    let mut server = TestHttpServer::spawn_raw(vec![truncated.clone(), truncated]).await;
    let channel = test_channel(&server.base_url);

    let add_error =
        channel.add_reaction("channel-1", "message-1", "👌").await.unwrap_err().to_string();
    let remove_error =
        channel.remove_reaction("channel-1", "message-1", "👌").await.unwrap_err().to_string();

    let add_request = server.next_request().await;
    let remove_request = server.next_request().await;
    assert!(
        add_request
            .starts_with("PUT /channels/channel-1/messages/message-1/reactions/%F0%9F%91%8C/@me ")
    );
    assert!(
        remove_request.starts_with(
            "DELETE /channels/channel-1/messages/message-1/reactions/%F0%9F%91%8C/@me "
        )
    );
    assert!(add_error.contains("<failed to read response body:"));
    assert!(remove_error.contains("<failed to read response body:"));
}

#[tokio::test]
async fn start_and_stop_typing_use_configured_api_base() {
    let mut server = TestHttpServer::spawn(vec![response(204, "")]).await;
    let channel = test_channel(&server.base_url);

    channel.start_typing("channel-1").await.unwrap();
    let request = server.next_request().await;
    channel.stop_typing("channel-1").await.unwrap();

    assert!(request.starts_with("POST /channels/channel-1/typing "));
    assert!(!channel.typing_handles.lock().contains_key("channel-1"));
}

#[tokio::test]
async fn listen_filters_events_and_dispatches_valid_message() {
    let valid_event = json!({
        "op": 0,
        "s": 7,
        "t": "MESSAGE_CREATE",
        "d": {
            "id": "message-1",
            "channel_id": "channel-1",
            "guild_id": "guild-1",
            "content": "<@123> run status",
            "author": {"id": "user-1"},
            "attachments": []
        }
    });
    let events = vec![
        json!({"op":0,"s":1,"t":"READY","d":{}}),
        json!({"op":0,"s":2,"t":"MESSAGE_CREATE"}),
        json!({"op":0,"s":3,"t":"MESSAGE_CREATE","d":{"author":{"id":"123"}}}),
        json!({"op":0,"s":4,"t":"MESSAGE_CREATE","d":{"author":{"id":"bot-1","bot":true}}}),
        json!({"op":0,"s":5,"t":"MESSAGE_CREATE","d":{"author":{"id":"blocked"}}}),
        json!({"op":0,"s":6,"t":"MESSAGE_CREATE","d":{"guild_id":"other","author":{"id":"user-1"}}}),
        json!({"op":0,"s":6,"t":"MESSAGE_CREATE","d":{"guild_id":"guild-1","content":"missing mention","author":{"id":"user-1"}}}),
        valid_event,
    ];
    let (gateway_url, gateway_handle) = spawn_gateway(events).await;
    let mut server = TestHttpServer::spawn(vec![
        response(200, json!({"url": gateway_url}).to_string()),
        response(204, ""),
    ])
    .await;
    let channel = DiscordChannel::new(
        "MTIz.fake.signature".to_string(),
        Some("guild-1".to_string()),
        vec!["user-1".to_string()],
        false,
        true,
    )
    .with_api_base(server.base_url.clone());
    let (tx, mut rx) = mpsc::channel(2);

    let listen_handle = tokio::spawn(async move { channel.listen(tx).await });
    let message = timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
    listen_handle.await.unwrap().unwrap();

    let gateway_request = server.next_request().await;
    let reaction_request = server.next_request().await;
    let gateway_incoming = gateway_handle.await.unwrap();

    assert!(gateway_request.starts_with("GET /gateway/bot "));
    assert!(gateway_incoming[0].contains(r#""op":2"#));
    assert!(gateway_incoming[0].contains("MTIz.fake.signature"));
    assert!(reaction_request.starts_with("PUT /channels/channel-1/messages/message-1/reactions/"));
    assert_eq!(message.id, "discord_message-1");
    assert_eq!(message.sender, "user-1");
    assert_eq!(message.reply_target, "channel-1");
    assert_eq!(message.channel, "discord");
    assert_eq!(message.content, "run status");
}

#[tokio::test]
async fn listen_dispatches_dm_with_attachment_and_generated_id() {
    let events = vec![json!({
        "op": 0,
        "s": 1,
        "t": "MESSAGE_CREATE",
        "d": {
            "id": "",
            "channel_id": "",
            "content": "see image",
            "author": {"id": "user-1"},
            "attachments": [{
                "content_type": "image/png",
                "filename": "image.png",
                "url": "https://cdn.test/image.png"
            }]
        }
    })];
    let (gateway_url, _gateway_handle) = spawn_gateway(events).await;
    let mut server =
        TestHttpServer::spawn(vec![response(200, json!({"url": gateway_url}).to_string())]).await;
    let channel = DiscordChannel::new(
        "MTIz.fake.signature".to_string(),
        None,
        vec!["user-1".to_string()],
        true,
        false,
    )
    .with_api_base(server.base_url.clone());
    let (tx, mut rx) = mpsc::channel(1);

    let listen_handle = tokio::spawn(async move { channel.listen(tx).await });
    let message = timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
    listen_handle.await.unwrap().unwrap();

    let gateway_request = server.next_request().await;
    assert!(gateway_request.starts_with("GET /gateway/bot "));
    assert_eq!(message.sender, "user-1");
    assert_eq!(message.reply_target, "user-1");
    assert!(!message.id.is_empty());
    assert!(!message.id.starts_with("discord_"));
    assert_eq!(message.content, "see image\n\n[Attachments]\n[IMAGE:https://cdn.test/image.png]");
}

#[tokio::test]
async fn listen_allows_group_sender_override_without_mention() {
    let events = vec![json!({
        "op": 0,
        "s": 1,
        "t": "MESSAGE_CREATE",
        "d": {
            "id": "message-2",
            "channel_id": "channel-1",
            "guild_id": "guild-1",
            "content": "run without mention",
            "author": {"id": "user-1"},
            "attachments": []
        }
    })];
    let (gateway_url, gateway_handle) = spawn_gateway(events).await;
    let mut server = TestHttpServer::spawn(vec![
        response(200, json!({"url": gateway_url}).to_string()),
        response(204, ""),
    ])
    .await;
    let channel = DiscordChannel::new(
        "MTIz.fake.signature".to_string(),
        Some("guild-1".to_string()),
        vec!["user-1".to_string()],
        false,
        true,
    )
    .with_group_reply_allowed_senders(vec!["user-1".to_string()])
    .with_api_base(server.base_url.clone());
    let (tx, mut rx) = mpsc::channel(1);

    let listen_handle = tokio::spawn(async move { channel.listen(tx).await });
    let message = timeout(Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
    listen_handle.await.unwrap().unwrap();

    let gateway_request = server.next_request().await;
    let reaction_request = server.next_request().await;
    gateway_handle.await.unwrap();
    assert!(gateway_request.starts_with("GET /gateway/bot "));
    assert!(reaction_request.starts_with("PUT /channels/channel-1/messages/message-2/reactions/"));
    assert_eq!(message.content, "run without mention");
}

#[tokio::test]
async fn listen_sends_interval_heartbeat() {
    let (gateway_url, gateway_handle) = spawn_gateway_for_interval_heartbeat().await;
    let mut server =
        TestHttpServer::spawn(vec![response(200, json!({"url": gateway_url}).to_string())]).await;
    let channel = test_channel(&server.base_url);
    let (tx, _rx) = mpsc::channel(1);

    channel.listen(tx).await.unwrap();

    let gateway_request = server.next_request().await;
    let incoming = gateway_handle.await.unwrap();
    assert!(gateway_request.starts_with("GET /gateway/bot "));
    assert!(incoming.iter().any(|message| message.contains(r#""op":1"#)));
}

#[tokio::test]
async fn listen_responds_to_server_heartbeat_requests() {
    let (gateway_url, gateway_handle) = spawn_gateway_for_server_heartbeat_request().await;
    let mut server =
        TestHttpServer::spawn(vec![response(200, json!({"url": gateway_url}).to_string())]).await;
    let channel = test_channel(&server.base_url);
    let (tx, _rx) = mpsc::channel(1);

    channel.listen(tx).await.unwrap();

    let gateway_request = server.next_request().await;
    let incoming = gateway_handle.await.unwrap();
    assert!(gateway_request.starts_with("GET /gateway/bot "));
    assert!(incoming[0].contains(r#""op":2"#));
    assert!(incoming[1].contains(r#""op":1"#));
    assert!(incoming[1].contains(r#""d":42"#));
}

#[tokio::test]
async fn listen_exits_on_gateway_control_opcodes() {
    listen_with_gateway_events(vec![json!({"op":7,"s":1})]).await;
    listen_with_gateway_events(vec![json!({"op":9,"s":1})]).await;
}
