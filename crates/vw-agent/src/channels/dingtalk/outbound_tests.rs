use super::*;
use crate::channels::outbound_delivery::{ChannelDeliveryError, DeliveryFailureKind};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

struct Webhook {
    url: String,
    stop: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<Vec<serde_json::Value>>,
    observed: tokio::sync::mpsc::UnboundedReceiver<usize>,
}

impl Webhook {
    async fn start(responses: Vec<(&'static str, &'static str)>) -> Self {
        Self::start_with_cache(responses, None).await
    }

    async fn start_with_cache(
        responses: Vec<(&'static str, &'static str)>,
        cache: Option<Arc<RwLock<HashMap<String, String>>>>,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url =
            format!("http://{}/reply?access_token=fixture-secret", listener.local_addr().unwrap());
        let (stop, mut stopped) = oneshot::channel();
        let (observations, observed) = tokio::sync::mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            let mut requests = Vec::new();
            loop {
                let accepted = tokio::select! {
                    _ = &mut stopped => break,
                    accepted = listener.accept() => accepted.unwrap(),
                };
                let (mut stream, _) = accepted;
                let body = tokio::time::timeout(Duration::from_secs(3), async {
                    let mut bytes = Vec::new();
                    loop {
                        let mut buffer = [0; 2048];
                        let count = stream.read(&mut buffer).await.unwrap();
                        assert_ne!(count, 0);
                        bytes.extend_from_slice(&buffer[..count]);
                        let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") else {
                            continue;
                        };
                        let headers = String::from_utf8_lossy(&bytes[..end]);
                        let length: usize = headers
                            .lines()
                            .find_map(|line| {
                                let (key, value) = line.split_once(':')?;
                                key.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse().unwrap())
                            })
                            .unwrap();
                        if bytes.len() >= end + 4 + length {
                            break serde_json::from_slice::<serde_json::Value>(
                                &bytes[end + 4..end + 4 + length],
                            )
                            .unwrap();
                        }
                    }
                })
                .await
                .unwrap();
                let (status, response) = responses
                    .get(requests.len())
                    .copied()
                    .unwrap_or(("200 OK", r#"{"errcode":0}"#));
                requests.push(body);
                let _ = observations.send(requests.len());
                if requests.len() == 1 {
                    if let Some(cache) = &cache {
                        tokio::time::timeout(Duration::from_secs(2), cache.write())
                            .await
                            .unwrap()
                            .insert("chat-1".into(), "http://127.0.0.1:1/not-original".into());
                    }
                }
                if status == "DROP" {
                    continue;
                }
                if status == "HOLD" {
                    let _ = (&mut stopped).await;
                    break;
                }
                if status == "TRUNCATED" {
                    stream
                        .write_all(
                            b"HTTP/1.1 200 OK\r\ncontent-length: 100\r\nconnection: close\r\n\r\n{",
                        )
                        .await
                        .unwrap();
                    continue;
                }
                let response = format!(
                    "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}",
                    response.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
            }
            requests
        });
        Self { url, stop, task, observed }
    }

    async fn finish(self) -> Vec<serde_json::Value> {
        let _ = self.stop.send(());
        tokio::time::timeout(Duration::from_secs(3), self.task).await.unwrap().unwrap()
    }
}

fn long_reply() -> String {
    format!(
        "{}\n```rust\n{}\n```",
        "正文中文😀一段。\n\n".repeat(400),
        "let value = 1;\n".repeat(60)
    )
}

async fn deliver(webhook: &Webhook, content: String) -> anyhow::Result<()> {
    let channel =
        DingTalkChannel::new("fixture-id".into(), "fixture-secret".into(), vec!["*".into()]);
    channel.session_webhooks.write().await.insert("chat-1".into(), webhook.url.clone());
    tokio::time::timeout(
        Duration::from_secs(5),
        Channel::send(&channel, &SendMessage::with_subject(content, "chat-1", "标题")),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn markdown_delivery_dingtalk_long_reply_stays_markdown() {
    let server = Webhook::start(vec![]).await;
    let result = deliver(&server, long_reply()).await;
    let requests = server.finish().await;
    result.unwrap();
    assert!(requests.len() >= 3, "expected multiple markdown requests, got {}", requests.len());
    for request in requests {
        assert_eq!(request["msgtype"], "markdown");
        assert_eq!(request["markdown"]["title"], "标题");
        assert!(request["markdown"]["text"].as_str().unwrap().len() <= 3000);
    }
}

#[tokio::test]
async fn markdown_delivery_dingtalk_business_error_stops_remaining() {
    let server = Webhook::start(vec![
        ("200 OK", r#"{"errcode":0}"#),
        ("200 OK", r#"{"errcode":130101,"errmsg":"fixture-secret"}"#),
    ])
    .await;
    let result = deliver(&server, long_reply()).await;
    let requests = server.finish().await;
    let error = result.expect_err("HTTP 200 business rejection must stop delivery");
    let progress = error.downcast_ref::<ChannelDeliveryError>().unwrap();
    assert_eq!(progress.confirmed_parts, 1);
    assert_eq!(progress.failed_part_index, 1);
    assert_eq!(progress.kind, DeliveryFailureKind::Rejected);
    assert!(progress.total_parts >= 3);
    assert_eq!(requests.len(), 2);
    assert!(!format!("{error:?}").contains("fixture-secret"));
}

#[tokio::test]
async fn markdown_delivery_dingtalk_unknown_result_does_not_fallback() {
    let server = Webhook::start(vec![("200 OK", "not-json")]).await;
    let result = deliver(&server, long_reply()).await;
    let requests = server.finish().await;
    let error = result.expect_err("malformed acknowledgement must not confirm delivery");
    assert_eq!(
        error.downcast_ref::<ChannelDeliveryError>().unwrap().kind,
        DeliveryFailureKind::Unknown
    );
    assert_eq!(requests.len(), 1);
}

#[tokio::test]
async fn markdown_delivery_dingtalk_failure_matrix_preserves_confirmed_prefix() {
    for (status, body, kind) in [
        ("429 Too Many Requests", r#"{"errcode":130101}"#, DeliveryFailureKind::Rejected),
        ("500 Internal Server Error", "fixture-secret", DeliveryFailureKind::Unknown),
        ("200 OK", "{}", DeliveryFailureKind::Unknown),
        ("200 OK", r#"{"errcode":"0"}"#, DeliveryFailureKind::Unknown),
        ("200 OK", r#"{"errcode":null}"#, DeliveryFailureKind::Unknown),
        ("200 OK", "not-json", DeliveryFailureKind::Unknown),
        ("DROP", "", DeliveryFailureKind::Unknown),
        ("TRUNCATED", "", DeliveryFailureKind::Unknown),
    ] {
        let server = Webhook::start(vec![("200 OK", r#"{"errcode":0}"#), (status, body)]).await;
        let result = deliver(&server, long_reply()).await;
        let requests = server.finish().await;
        let error = result.unwrap_err();
        let progress = error.downcast_ref::<ChannelDeliveryError>().unwrap();
        assert_eq!(progress.confirmed_parts, 1, "{status}");
        assert_eq!(progress.failed_part_index, 1);
        assert!(progress.total_parts >= 3);
        assert_eq!(progress.kind, kind, "{status}");
        assert_eq!(requests.len(), 2);
        assert!(!format!("{error:?}").contains("fixture-secret"));
        assert_eq!(requests[0]["msgtype"], "markdown");
        assert_eq!(requests[1]["msgtype"], "markdown");
    }
}

#[tokio::test]
async fn markdown_delivery_dingtalk_preflights_all_payloads_before_first_request() {
    for (content, title, later_part) in [
        ("hello".into(), "T".repeat(16 * 1024), false),
        (format!("prefix\n\n```rust\n{}\n```", "a".repeat(3001)), "T".into(), false),
        (
            format!("prefix\n\n| key | value |\n| --- | --- |\n| row | {} |", "a".repeat(3001)),
            "T".into(),
            false,
        ),
        (
            format!("```\n{}\n```\n\n{}", "a".repeat(2800), "\"".repeat(3000)),
            "T".repeat(11500),
            true,
        ),
    ] {
        let server = Webhook::start(vec![]).await;
        let channel = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);
        channel.session_webhooks.write().await.insert("chat-1".into(), server.url.clone());
        let result = tokio::time::timeout(
            Duration::from_secs(3),
            channel.send(&SendMessage::with_subject(content, "chat-1", title)),
        )
        .await
        .unwrap();
        let requests = server.finish().await;
        let error = result.unwrap_err();
        let progress = error.downcast_ref::<ChannelDeliveryError>().unwrap();
        assert_eq!(progress.kind, DeliveryFailureKind::NotSent);
        assert_eq!(progress.confirmed_parts, 0);
        if later_part {
            assert!(progress.failed_part_index > 0);
        }
        assert!(requests.is_empty());
    }
}

#[tokio::test]
async fn markdown_delivery_dingtalk_snapshots_webhook_without_holding_cache_lock() {
    let channel = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);
    let server = Webhook::start_with_cache(vec![], Some(channel.session_webhooks.clone())).await;
    channel.session_webhooks.write().await.insert("chat-1".into(), server.url.clone());
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        channel.send(&SendMessage::with_subject(long_reply(), "chat-1", "固定标题")),
    )
    .await
    .unwrap();
    let requests = server.finish().await;
    result.unwrap();
    assert!(requests.len() >= 3);
    assert!(channel.session_webhooks.read().await["chat-1"].contains("not-original"));
    for request in requests {
        assert_eq!(request["markdown"]["title"], "固定标题");
    }
}

#[tokio::test]
async fn markdown_delivery_dingtalk_short_timeout_stops_without_replay() {
    let mut server = Webhook::start(vec![("200 OK", r#"{"errcode":0}"#), ("HOLD", "")]).await;
    let channel = DingTalkChannel::new("id".into(), "secret".into(), vec!["*".into()]);
    let payloads =
        DingTalkChannel::webhook_reply_payloads(&SendMessage::new(long_reply(), "chat-1")).unwrap();
    let client = reqwest::Client::builder()
        .retry(reqwest::retry::never())
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let url = server.url.clone();
    let task = tokio::spawn(async move {
        channel.deliver_webhook_payloads(&client, &url, payloads, Duration::from_millis(100)).await
    });
    for expected in 1..=2 {
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), server.observed.recv()).await.unwrap(),
            Some(expected)
        );
    }
    let error =
        tokio::time::timeout(Duration::from_secs(2), task).await.unwrap().unwrap().unwrap_err();
    let requests = server.finish().await;
    let progress = error.downcast_ref::<ChannelDeliveryError>().unwrap();
    assert_eq!(progress.confirmed_parts, 1);
    assert_eq!(progress.failed_part_index, 1);
    assert_eq!(progress.kind, DeliveryFailureKind::Unknown);
    assert_eq!(requests.len(), 2);
}

#[tokio::test]
async fn markdown_delivery_dingtalk_preserves_code_and_table_data() {
    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
    let rows: Vec<_> = (0..150).map(|n| format!("| 行{n:03} | 中文😀值{n:03} |\n")).collect();
    let source = format!("{}\n| key | value |\n| --- | --- |\n{}", long_reply(), rows.concat());
    let mut expected_code = String::new();
    let mut source_in_code = false;
    for event in Parser::new_ext(&source, Options::ENABLE_TABLES) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => source_in_code = true,
            Event::End(TagEnd::CodeBlock) => source_in_code = false,
            Event::Text(text) if source_in_code => expected_code.push_str(&text),
            _ => {}
        }
    }
    let server = Webhook::start(vec![]).await;
    let result = deliver(&server, source.clone()).await;
    let requests = server.finish().await;
    result.unwrap();
    let mut code = String::new();
    let mut all_text = String::new();
    for request in requests {
        assert!(serde_json::to_vec(&request).unwrap().len() <= 16 * 1024);
        let text = request["markdown"]["text"].as_str().unwrap();
        assert!(text.len() <= 3000);
        let mut in_code = false;
        for event in Parser::new_ext(text, Options::ENABLE_TABLES) {
            match event {
                Event::Start(Tag::CodeBlock(_)) => in_code = true,
                Event::End(TagEnd::CodeBlock) => in_code = false,
                Event::Text(text) if in_code => code.push_str(&text),
                _ => {}
            }
        }
        all_text.push_str(text);
    }
    assert_eq!(code, expected_code);
    assert_eq!(all_text.matches("正文中文😀一段。").count(), 400);
    for row in rows {
        assert_eq!(all_text.matches(row.trim_end()).count(), 1, "{row}");
    }
}
