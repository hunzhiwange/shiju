use super::*;
use crate::app::agent::channels::outbound_delivery::{ChannelDeliveryError, DeliveryFailureKind};
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
            let request = read_ilink_request(&mut stream).await;
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

    let received = tokio::time::timeout(std::time::Duration::from_secs(3), rx.recv())
        .await
        .unwrap()
        .expect("text update should reach channel runtime");
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

    let requests =
        tokio::time::timeout(std::time::Duration::from_secs(3), server).await.unwrap().unwrap();
    assert_eq!(requests[0]["get_updates_buf"], "");
    let sent = requests.last().unwrap();
    assert_eq!(sent["msg"]["to_user_id"], "user-1@im.wechat");
    assert_eq!(sent["msg"]["context_token"], "context-1");
    assert_eq!(
        sent["msg"]["item_list"][0]["text_item"]["text"],
        "## 回复\n\n**重点**与[文档](https://example.com)"
    );

    listen.abort();
    let _ = listen.await;
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

async fn read_ilink_request(stream: &mut tokio::net::TcpStream) -> String {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let mut bytes = Vec::new();
        loop {
            let mut buffer = [0_u8; 4096];
            let read = stream.read(&mut buffer).await.unwrap();
            assert_ne!(read, 0, "complete HTTP request required");
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..end]);
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse().unwrap())
                    })
                    .unwrap();
                if bytes.len() >= end + 4 + length {
                    return String::from_utf8(bytes).unwrap();
                }
            }
        }
    })
    .await
    .unwrap()
}

async fn markdown_delivery_wechat_fixture(
    responses: Vec<Option<String>>,
) -> (WeChatChannel, std::sync::Arc<std::sync::Mutex<Vec<Value>>>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let channel = WeChatChannel::new_for_test(
        "fixture-token".into(),
        format!("http://{}", listener.local_addr().unwrap()),
    )
    .unwrap();
    channel.context_tokens.write().await.insert("fixture-user".into(), "fixture-context".into());
    let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = requests.clone();
    let contexts = channel.context_tokens.clone();
    let server = tokio::spawn(async move {
        let mut responses = responses.into_iter();
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_ilink_request(&mut stream).await;
            assert!(request.starts_with("POST /ilink/bot/sendmessage "));
            let (_, body) = request.split_once("\r\n\r\n").unwrap();
            captured.lock().unwrap().push(serde_json::from_str(body).unwrap());
            // An inbound update during delivery must not change this reply's context.
            contexts.write().await.insert("fixture-user".into(), "updated-context".into());
            if let Some(body) = responses.next().unwrap_or(Some("{\"ret\":0}".into())) {
                if body == "__stall__" {
                    std::future::pending::<()>().await;
                }
                let status =
                    if body == "__server_error__" { "500 Internal Server Error" } else { "200 OK" };
                let reply = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream.write_all(reply.as_bytes()).await.unwrap();
            }
        }
    });
    (channel, requests, server)
}

fn markdown_delivery_wechat_message() -> SendMessage {
    SendMessage {
        recipient: "fixture-user".into(),
        content: ["甲😀".repeat(300), "乙😀".repeat(300), "丙😀".repeat(300)].join("\n\n"),
        subject: None,
        thread_ts: None,
    }
}

#[tokio::test]
async fn markdown_delivery_wechat_chunks_preserve_source_and_order() {
    let (channel, captured, server) = markdown_delivery_wechat_fixture(vec![]).await;
    let message = markdown_delivery_wechat_message();
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), channel.send(&message))
        .await
        .unwrap();
    server.abort();
    let _ = server.await;
    result.unwrap();
    let requests = captured.lock().unwrap();
    assert_eq!(requests.len(), 3);
    let mut reconstructed = String::new();
    let mut ids = std::collections::HashSet::new();
    for request in requests.iter() {
        let text = request["msg"]["item_list"][0]["text_item"]["text"].as_str().unwrap();
        assert!(text.len() <= 3000);
        assert!(serde_json::to_vec(request).unwrap().len() <= 16 * 1024);
        assert_eq!(request["msg"]["context_token"], "fixture-context");
        assert_eq!(request["msg"]["to_user_id"], message.recipient);
        assert!(ids.insert(request["msg"]["client_id"].as_str().unwrap()));
        reconstructed.push_str(text);
    }
    assert_eq!(reconstructed, message.content);
}

#[tokio::test]
async fn markdown_delivery_wechat_errcode_stops_remaining_chunks() {
    let (channel, captured, server) = markdown_delivery_wechat_fixture(vec![
        Some(json!({"ret":0}).to_string()),
        Some(json!({"ret":0,"errcode":-14}).to_string()),
    ])
    .await;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        channel.send(&markdown_delivery_wechat_message()),
    )
    .await
    .unwrap();
    server.abort();
    let _ = server.await;
    assert!(result.is_err(), "nonzero errcode must reject the current chunk");
    assert_eq!(captured.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn markdown_delivery_wechat_unknown_result_does_not_regenerate_ids() {
    let (channel, captured, server) = markdown_delivery_wechat_fixture(vec![
        Some(json!({"ret":0}).to_string()),
        Some("{}".into()),
    ])
    .await;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        channel.send(&markdown_delivery_wechat_message()),
    )
    .await
    .unwrap();
    server.abort();
    let _ = server.await;
    assert!(result.is_err(), "missing success evidence must stop delivery");
    let requests = captured.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_ne!(requests[0]["msg"]["client_id"], requests[1]["msg"]["client_id"]);
}

fn markdown_delivery_wechat_assert_failure(
    error: &anyhow::Error,
    confirmed: usize,
    total: usize,
    kind: DeliveryFailureKind,
) {
    let progress = error.downcast_ref::<ChannelDeliveryError>().expect("typed delivery progress");
    assert_eq!(progress.confirmed_parts, confirmed);
    assert_eq!(progress.total_parts, total);
    assert_eq!(progress.failed_part_index, confirmed);
    assert_eq!(progress.kind, kind);
    let display = format!("{error:?}");
    for secret in ["fixture-token", "fixture-context", "fixture-user", "127.0.0.1", "甲"] {
        assert!(!display.contains(secret));
    }
}

#[tokio::test]
async fn markdown_delivery_wechat_failures_preserve_confirmed_prefix() {
    for (reply, kind) in [
        (Some(json!({"ret":-2}).to_string()), DeliveryFailureKind::Rejected),
        (Some(json!({"ret":0,"errcode":-9}).to_string()), DeliveryFailureKind::Rejected),
        (Some(json!({"errcode":-9}).to_string()), DeliveryFailureKind::Rejected),
        (Some(json!({"ret":"0"}).to_string()), DeliveryFailureKind::Unknown),
        (Some(json!({"ret":0,"errcode":null}).to_string()), DeliveryFailureKind::Unknown),
        (Some("{}".into()), DeliveryFailureKind::Unknown),
        (Some("not json".into()), DeliveryFailureKind::Unknown),
        (Some("__server_error__".into()), DeliveryFailureKind::Unknown),
        (None, DeliveryFailureKind::Unknown),
    ] {
        let (channel, captured, server) =
            markdown_delivery_wechat_fixture(vec![Some("{\"ret\":0}".into()), reply]).await;
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            channel.send(&markdown_delivery_wechat_message()),
        )
        .await
        .unwrap();
        server.abort();
        let _ = server.await;
        markdown_delivery_wechat_assert_failure(&result.unwrap_err(), 1, 3, kind);
        let requests = captured.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(
            requests[0]["msg"]["item_list"][0]["text_item"]["text"]
                .as_str()
                .unwrap()
                .starts_with('甲')
        );
        assert!(
            requests[1]["msg"]["item_list"][0]["text_item"]["text"]
                .as_str()
                .unwrap()
                .starts_with('乙')
        );
    }
    let (channel, captured, server) =
        markdown_delivery_wechat_fixture(vec![Some("{\"ret\":-2}".into())]).await;
    let error = channel.send(&markdown_delivery_wechat_message()).await.unwrap_err();
    server.abort();
    let _ = server.await;
    markdown_delivery_wechat_assert_failure(&error, 0, 3, DeliveryFailureKind::Rejected);
    assert_eq!(captured.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn markdown_delivery_wechat_preflight_rejects_unsendable_reply_without_requests() {
    for content in [
        format!("valid prefix\n\n```rs\n{}\n```", "x".repeat(3001)),
        format!("valid prefix\n\n| h |\n| --- |\n| {} |", "x".repeat(3001)),
        format!("{}\n\n{}", "a".repeat(2100), "\u{1}".repeat(2800)),
    ] {
        let (channel, captured, server) = markdown_delivery_wechat_fixture(vec![]).await;
        let mut message = markdown_delivery_wechat_message();
        message.content = content;
        let error = channel.send(&message).await.unwrap_err();
        server.abort();
        let _ = server.await;
        let progress = error.downcast_ref::<ChannelDeliveryError>().unwrap();
        assert_eq!(progress.confirmed_parts, 0);
        assert_eq!(progress.failed_part_index, 0);
        assert_eq!(progress.kind, DeliveryFailureKind::NotSent);
        assert!(captured.lock().unwrap().is_empty());
    }
    let (channel, captured, server) = markdown_delivery_wechat_fixture(vec![]).await;
    let mut message = markdown_delivery_wechat_message();
    message.recipient = "r".repeat(16 * 1024);
    channel
        .context_tokens
        .write()
        .await
        .insert(message.recipient.clone(), "fixture-context".into());
    let error = channel.send(&message).await.unwrap_err();
    markdown_delivery_wechat_assert_failure(&error, 0, 3, DeliveryFailureKind::NotSent);
    message.recipient = "missing-context".into();
    let error = channel.send(&message).await.unwrap_err();
    markdown_delivery_wechat_assert_failure(&error, 0, 3, DeliveryFailureKind::NotSent);
    message.content.clear();
    channel.send(&message).await.unwrap();
    server.abort();
    let _ = server.await;
    assert!(captured.lock().unwrap().is_empty());
}

#[tokio::test]
async fn markdown_delivery_wechat_text_items_preserve_fences_and_table_rows() {
    let (channel, captured, server) = markdown_delivery_wechat_fixture(vec![]).await;
    let mut message = markdown_delivery_wechat_message();
    let code: String = (0..150).map(|i| format!("let value_{i} = \"中文😀\";\n")).collect();
    let rows: Vec<_> = (0..150).map(|i| format!("| row-{i} | 中文😀-{i} |\n")).collect();
    message.content =
        format!("````rust\n{code}````\n\n| key | value |\n| --- | --- |\n{}", rows.concat());
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), channel.send(&message))
        .await
        .unwrap();
    server.abort();
    let _ = server.await;
    result.unwrap();
    let requests = captured.lock().unwrap();
    assert!(requests.len() >= 3);
    let mut reconstructed_code = String::new();
    let mut reconstructed_rows = Vec::new();
    for request in requests.iter() {
        let text = request["msg"]["item_list"][0]["text_item"]["text"].as_str().unwrap();
        assert!(text.len() <= 3000);
        for (event, _) in
            pulldown_cmark::Parser::new_ext(text, pulldown_cmark::Options::ENABLE_TABLES)
                .into_offset_iter()
        {
            if let pulldown_cmark::Event::Text(value) = event {
                if value.starts_with("let value_") {
                    reconstructed_code.push_str(&value);
                }
            }
        }
        reconstructed_rows.extend(
            text.lines().filter(|line| line.starts_with("| row-")).map(|line| format!("{line}\n")),
        );
        if text.contains("| row-") {
            assert!(text.contains("| key | value |\n| --- | --- |\n"));
        }
    }
    assert_eq!(reconstructed_code, code);
    assert_eq!(reconstructed_rows, rows);
}

#[tokio::test]
async fn markdown_delivery_wechat_timeout_stops_without_post_retry() {
    let (mut channel, captured, server) = markdown_delivery_wechat_fixture(vec![
        Some("{\"ret\":0}".into()),
        Some("__stall__".into()),
    ])
    .await;
    channel.client = Client::builder()
        .retry(reqwest::retry::never())
        .redirect(reqwest::redirect::Policy::none())
        .read_timeout(std::time::Duration::from_millis(50))
        .build()
        .unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        channel.send(&markdown_delivery_wechat_message()),
    )
    .await
    .unwrap();
    server.abort();
    let _ = server.await;
    markdown_delivery_wechat_assert_failure(
        &result.unwrap_err(),
        1,
        3,
        DeliveryFailureKind::Unknown,
    );
    assert_eq!(captured.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn markdown_delivery_wechat_complete_large_json_and_errcode_only_success() {
    let (channel, captured, server) =
        markdown_delivery_wechat_fixture(vec![Some("{\"errcode\":0}".into())]).await;
    let mut message = markdown_delivery_wechat_message();
    message.recipient = "r".repeat(9000);
    message.content = "## short 中文 reply".into();
    channel
        .context_tokens
        .write()
        .await
        .insert(message.recipient.clone(), "fixture-context".into());
    channel.send(&message).await.unwrap();
    server.abort();
    let _ = server.await;
    let requests = captured.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(serde_json::to_vec(&requests[0]).unwrap().len() > 8192);
    assert_eq!(requests[0]["msg"]["item_list"][0]["text_item"]["text"], message.content);
}
