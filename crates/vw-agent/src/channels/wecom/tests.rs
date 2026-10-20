use super::*;
use crate::app::agent::channels::traits::{Channel, SendMessage};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::{accept_async, tungstenite::Message};

async fn start_mock_gateway() -> (String, tokio::task::JoinHandle<Vec<Value>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("ws://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        let subscribe: Value = serde_json::from_str(
            socket.next().await.unwrap().unwrap().into_text().unwrap().as_ref(),
        )
        .unwrap();
        let request_id = subscribe["headers"]["req_id"].as_str().unwrap();
        socket
            .send(Message::Text(
                json!({"errcode": 0, "headers": {"req_id": request_id}}).to_string().into(),
            ))
            .await
            .unwrap();
        socket
            .send(Message::Text(
                json!({
                    "cmd": "aibot_msg_callback",
                    "headers": {"req_id": "callback-1"},
                    "body": {
                        "msgtype": "text",
                        "msgid": "message-1",
                        "chatid": "chat-1",
                        "chattype": "single",
                        "send_time": 42,
                        "from": {"userid": "user-1"},
                        "text": {"content": "hello"}
                    }
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
        let reply: Value = serde_json::from_str(
            socket.next().await.unwrap().unwrap().into_text().unwrap().as_ref(),
        )
        .unwrap();
        vec![subscribe, reply]
    });
    (endpoint, server)
}

#[tokio::test]
async fn wecom_channel_receives_text_and_replies_in_same_chat() {
    let (endpoint, server) = start_mock_gateway().await;
    let channel = WeComChannel::new_for_test("bot-id".into(), "secret".into(), endpoint);
    let listener_channel = channel.clone();
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let listen = tokio::spawn(async move { listener_channel.listen(tx).await });

    let received = rx.recv().await.expect("text callback should reach channel runtime");
    assert_eq!(received.content, "hello");
    assert_eq!(received.reply_target, "chat-1");

    channel
        .send(&SendMessage {
            recipient: received.reply_target,
            content: "| 名称 | 状态 |\n| --- | --- |\n| A | 已完成 |".into(),
            subject: None,
            thread_ts: None,
        })
        .await
        .unwrap();

    let frames = server.await.unwrap();
    assert_eq!(frames[0]["cmd"], "aibot_subscribe");
    assert_eq!(frames[0]["body"]["bot_id"], "bot-id");
    assert_eq!(frames[0]["body"]["secret"], "secret");
    assert_eq!(frames[1]["cmd"], "aibot_respond_msg");
    assert_eq!(frames[1]["headers"]["req_id"], "callback-1");
    assert_eq!(
        frames[1]["body"]["stream"]["content"],
        "| 名称 | 状态  |\n| --- | --- |\n| A  | 已完成 |"
    );
    assert_eq!(frames[1]["body"]["stream"]["finish"], true);

    listen.abort();
}

#[tokio::test]
async fn wecom_channel_rejects_missing_credentials() {
    let channel = WeComChannel::new(String::new(), "secret".into());
    let (tx, _) = tokio::sync::mpsc::channel(1);
    let error = channel.listen(tx).await.unwrap_err().to_string();
    assert!(error.contains("bot_id"));
}

#[tokio::test]
async fn wecom_channel_rejects_failed_subscription() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("ws://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        let subscribe: Value = serde_json::from_str(
            socket.next().await.unwrap().unwrap().into_text().unwrap().as_ref(),
        )
        .unwrap();
        socket
            .send(Message::Text(
                json!({
                    "errcode": 40001,
                    "errmsg": "invalid secret",
                    "headers": {"req_id": subscribe["headers"]["req_id"]}
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
    });

    let channel = WeComChannel::new_for_test("bot-id".into(), "secret".into(), endpoint);
    let (tx, _) = tokio::sync::mpsc::channel(1);
    let error = channel.listen(tx).await.unwrap_err().to_string();

    assert!(error.contains("订阅"));
    assert!(error.contains("40001"));
    assert!(!error.contains("bot-id"));
    assert!(!error.contains("secret"));
    server.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn wecom_channel_sends_protocol_heartbeat() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("ws://{}", listener.local_addr().unwrap());
    let (subscribed_tx, subscribed_rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();
        let subscribe: Value = serde_json::from_str(
            socket.next().await.unwrap().unwrap().into_text().unwrap().as_ref(),
        )
        .unwrap();
        socket
            .send(Message::Text(
                json!({
                    "errcode": 0,
                    "headers": {"req_id": subscribe["headers"]["req_id"]}
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
        subscribed_tx.send(()).unwrap();
        let heartbeat = timeout(Duration::from_secs(31), socket.next())
            .await
            .expect("channel should send protocol heartbeat")
            .unwrap()
            .unwrap();
        serde_json::from_str::<Value>(heartbeat.into_text().unwrap().as_ref()).unwrap()
    });

    let channel = WeComChannel::new_for_test("bot-id".into(), "secret".into(), endpoint);
    let listener_channel = channel.clone();
    let (tx, _) = tokio::sync::mpsc::channel(1);
    let listen = tokio::spawn(async move { listener_channel.listen(tx).await });
    subscribed_rx.await.unwrap();
    while channel.outbound.read().await.is_none() {
        tokio::task::yield_now().await;
    }
    tokio::time::advance(Duration::from_secs(30)).await;
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    tokio::time::advance(Duration::from_secs(1)).await;

    let heartbeat = server.await.unwrap();
    assert_eq!(heartbeat["cmd"], "ping");
    assert!(heartbeat["headers"]["req_id"].as_str().is_some());
    listen.abort();
}

#[test]
fn wecom_channel_rejects_unsupported_media() {
    let error = WeComChannel::callback_message(&json!({
        "cmd": "aibot_msg_callback",
        "headers": {"req_id": "callback-2"},
        "body": {"msgtype": "image"}
    }))
    .unwrap_err()
    .to_string();
    assert!(error.contains("image"));
    assert!(error.contains("仅支持 text"));
}
