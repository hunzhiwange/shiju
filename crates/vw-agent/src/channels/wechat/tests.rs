use super::*;
use crate::app::agent::channels::traits::{Channel, SendMessage};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn start_mock_ilink() -> (String, tokio::task::JoinHandle<Vec<Value>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        let mut delivered_update = false;
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buffer = vec![0_u8; 8192];
            let read = stream.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..read]);
            let is_send = request.starts_with("POST /ilink/bot/sendmessage ");
            let (_, body) = request.split_once("\r\n\r\n").unwrap();
            requests.push(serde_json::from_str(body).unwrap());

            let response = if is_send {
                json!({"ret": 0})
            } else if !delivered_update {
                delivered_update = true;
                json!({
                "ret": 0,
                "get_updates_buf": "cursor-2",
                "msgs": [{
                    "from_user_id": "user-1@im.wechat",
                    "message_type": 1,
                    "context_token": "context-1",
                    "item_list": [{"type": 1, "text_item": {"text": "hello"}}]
                }]
                })
            } else {
                json!({"ret": 0, "get_updates_buf": "cursor-2"})
            };

            let body = response.to_string();
            let reply = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(reply.as_bytes()).await.unwrap();
            if is_send {
                break;
            }
        }
        requests
    });
    (endpoint, server)
}

#[tokio::test]
async fn wechat_channel_receives_text_and_sends_reply() {
    let (endpoint, server) = start_mock_ilink().await;
    let channel = WeChatChannel::new_for_test("bot-token".into(), endpoint).unwrap();
    let listener_channel = channel.clone();
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let listen = tokio::spawn(async move { listener_channel.listen(tx).await });

    let received = rx.recv().await.expect("text update should reach channel runtime");
    assert_eq!(received.content, "hello");
    assert_eq!(received.reply_target, "user-1@im.wechat");

    channel
        .send(&SendMessage {
            recipient: received.reply_target,
            content: "## 回复\n\n**重点**与[文档](https://example.com)".into(),
            subject: None,
            thread_ts: None,
        })
        .await
        .unwrap();

    let requests = server.await.unwrap();
    assert_eq!(requests[0]["get_updates_buf"], "");
    let sent = requests.last().unwrap();
    assert_eq!(sent["msg"]["to_user_id"], "user-1@im.wechat");
    assert_eq!(sent["msg"]["context_token"], "context-1");
    assert_eq!(
        sent["msg"]["item_list"][0]["text_item"]["text"],
        "## 回复\n\n**重点**与[文档](https://example.com)"
    );

    listen.abort();
}

#[tokio::test]
async fn wechat_channel_rejects_missing_token() {
    let channel = WeChatChannel::new_for_test(String::new(), "http://127.0.0.1:1".into()).unwrap();
    let (tx, _) = tokio::sync::mpsc::channel(1);
    assert!(channel.listen(tx).await.unwrap_err().to_string().contains("bot_token"));
}

#[test]
fn wechat_channel_rejects_untrusted_base_url_and_non_text_messages() {
    assert!(WeChatChannel::new("token".into(), "https://attacker.example".into()).is_err());
    let error = WeChatChannel::parse_message(&json!({
        "from_user_id": "user-1@im.wechat",
        "message_type": 1,
        "context_token": "context-1",
        "item_list": [{"type": 2}]
    }))
    .unwrap_err()
    .to_string();
    assert!(error.contains("仅支持 text"));
}
