use super::*;
use axum::{Json, Router, routing::post};
use serde_json::{Value, json};
use tokio::sync::mpsc;

#[tokio::test]
async fn wecom_webhook_sends_real_markdown_request() {
    let (tx, mut rx) = mpsc::channel(4);
    let app = Router::new().route(
        "/",
        post(move |Json(body): Json<Value>| {
            let tx = tx.clone();
            async move {
                tx.send(body).await.unwrap();
                Json(json!({"errcode":0}))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/", listener.local_addr().unwrap()).parse().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let sender = WeComWebhook::new_for_test(endpoint, Duration::from_secs(2));
    let result = sender.send("你好").await;
    server.abort();
    result.unwrap();
    assert_eq!(
        rx.recv().await.unwrap(),
        json!({"msgtype":"markdown","markdown":{"content":"你好"}})
    );
}

#[test]
fn wecom_webhook_rejects_unsafe_urls() {
    for url in [
        "http://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=x",
        "https://qyapi.weixin.qq.com.evil.test/cgi-bin/webhook/send?key=x",
        "https://qyapi.weixin.qq.com:444/cgi-bin/webhook/send?key=x",
        "https://@qyapi.weixin.qq.com/cgi-bin/webhook/send?key=x",
        "https://u:p@qyapi.weixin.qq.com/cgi-bin/webhook/send?key=x",
        "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=x#fragment",
        "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=x&key=y",
        "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=x&extra=y",
        "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=",
        "https://qyapi.weixin.qq.com/cgi-bin/webhook/send",
        "https://qyapi.weixin.qq.com/cgi-bin/webhook/send/?key=x",
        "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=%20",
        "https://127.0.0.1/cgi-bin/webhook/send?key=x",
    ] {
        assert!(WeComWebhook::new(url).is_err(), "accepted unsafe URL");
    }
    assert!(
        WeComWebhook::new("https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=fixture").is_ok()
    );
}

use crate::channels::outbound_delivery::{ChannelDeliveryError, DeliveryFailureKind};
use axum::{
    body::Body,
    http::{Response, StatusCode, header},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct Fixture {
    sender: WeComWebhook,
    requests: mpsc::UnboundedReceiver<Value>,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn fixture(responses: Vec<Response<Body>>, delay: Duration) -> Fixture {
    let (tx, requests) = mpsc::unbounded_channel();
    let responses = Arc::new(tokio::sync::Mutex::new(std::collections::VecDeque::from(responses)));
    let app = Router::new().route(
        "/",
        post(move |Json(body): Json<Value>| {
            let tx = tx.clone();
            let responses = responses.clone();
            async move {
                tx.send(body).unwrap();
                tokio::time::sleep(delay).await;
                responses.lock().await.pop_front().unwrap_or_else(|| reply(200, r#"{"errcode":0}"#))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/", listener.local_addr().unwrap()).parse().unwrap();
    let sender = WeComWebhook::new_for_test(endpoint, Duration::from_secs(2));
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    Fixture { sender, requests, server }
}
fn reply(status: u16, body: &str) -> Response<Body> {
    Response::builder().status(status).body(Body::from(body.to_owned())).unwrap()
}

#[tokio::test]
async fn wecom_webhook_splits_chinese_bytes_with_markers() {
    let mut f = fixture(vec![], Duration::ZERO).await;
    let content = "中🙂文".repeat(1500);
    f.sender.send(&content).await.unwrap();
    let mut parts = vec![];
    while let Ok(body) = f.requests.try_recv() {
        assert_eq!(body["msgtype"], "markdown");
        let text = body["markdown"]["content"].as_str().unwrap().to_owned();
        assert!(text.len() <= 3000);
        parts.push(text);
    }
    assert!(parts.len() > 1);
    let mut reconstructed = String::new();
    for (index, part) in parts.iter().enumerate() {
        let marker = format!("[{}/{}]\n", index + 1, parts.len());
        reconstructed.push_str(part.strip_prefix(&marker).unwrap());
    }
    assert_eq!(reconstructed, content);
}

#[tokio::test]
async fn wecom_webhook_normalizes_tables_before_send() {
    let mut f = fixture(vec![], Duration::ZERO).await;
    let content = "| 姓名 | 数量 |\n| --- | --- |\n| 示例 | 2 |\n";
    f.sender.send(content).await.unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), f.requests.recv()).await.unwrap().unwrap()["markdown"]
            ["content"],
        crate::channels::outbound_format::format_wecom_markdown_tables(content)
    );
}

#[tokio::test]
async fn wecom_webhook_empty_and_unsplittable_results_do_not_send() {
    let mut f = fixture(vec![], Duration::ZERO).await;
    for text in ["".to_owned(), " \n\t".to_owned(), format!("```\n{}\n```", "x".repeat(4000))] {
        let err = f.sender.send(&text).await.unwrap_err();
        assert_eq!(
            err.downcast_ref::<ChannelDeliveryError>().unwrap().kind,
            DeliveryFailureKind::NotSent
        );
        assert!(f.requests.try_recv().is_err());
    }
}

#[tokio::test]
async fn wecom_webhook_validates_ack_and_redacts_errors() {
    for (status, body, kind) in [
        (
            200,
            r#"{"errcode":40058,"errmsg":"secret-url-sensitive-body"}"#,
            DeliveryFailureKind::Rejected,
        ),
        (200, r#"{"errmsg":"secret-url-sensitive-body"}"#, DeliveryFailureKind::Unknown),
        (200, r#"{"errcode":"0"}"#, DeliveryFailureKind::Unknown),
        (200, "secret-url-sensitive-body", DeliveryFailureKind::Unknown),
        (403, "secret-url-sensitive-body", DeliveryFailureKind::Unknown),
        (500, r#"{"errcode":0}"#, DeliveryFailureKind::Unknown),
    ] {
        let mut f = fixture(vec![reply(status, body)], Duration::ZERO).await;
        let err = f.sender.send("secret-url-sensitive-body").await.unwrap_err();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(3), f.requests.recv()).await.unwrap().unwrap()
                ["msgtype"],
            "markdown"
        );
        let progress = err.downcast_ref::<ChannelDeliveryError>().unwrap();
        assert_eq!(
            (
                progress.confirmed_parts,
                progress.total_parts,
                progress.failed_part_index,
                progress.kind
            ),
            (0, 1, 0, kind)
        );
        let diagnostic = format!("{err:?}");
        assert!(!diagnostic.contains("secret-url-sensitive-body"));
        assert!(!diagnostic.contains("127.0.0.1"));
        assert!(!diagnostic.contains("http://"));
    }
}

#[tokio::test]
async fn wecom_webhook_timeout_is_unknown_and_not_retried() {
    let mut f = fixture(vec![], Duration::from_secs(2)).await;
    f.sender.timeout = Duration::from_millis(100);
    let err = f.sender.send("fixture").await.unwrap_err();
    assert_eq!(
        err.downcast_ref::<ChannelDeliveryError>().unwrap().kind,
        DeliveryFailureKind::Unknown
    );
    assert!(f.requests.try_recv().is_ok());
    assert!(f.requests.try_recv().is_err());
}

#[tokio::test]
async fn wecom_webhook_partial_failure_stops_at_confirmed_prefix() {
    let mut f = fixture(
        vec![reply(200, r#"{"errcode":0}"#), reply(200, r#"{"errcode":45009}"#)],
        Duration::ZERO,
    )
    .await;
    let err = f.sender.send(&"字".repeat(4000)).await.unwrap_err();
    let progress = err.downcast_ref::<ChannelDeliveryError>().unwrap();
    assert_eq!(progress.confirmed_parts, 1);
    assert_eq!(progress.failed_part_index, 1);
    assert!(progress.total_parts > 2);
    assert_eq!(progress.kind, DeliveryFailureKind::Rejected);
    assert!(f.requests.try_recv().is_ok());
    assert!(f.requests.try_recv().is_ok());
    assert!(f.requests.try_recv().is_err());
}

#[tokio::test]
async fn wecom_webhook_limits_response_size() {
    let body = format!("{{\"errcode\":0,\"padding\":\"{}\"}}", "x".repeat(17000));
    let f = fixture(vec![reply(200, &body)], Duration::ZERO).await;
    let err = f.sender.send("fixture").await.unwrap_err();
    assert_eq!(
        err.downcast_ref::<ChannelDeliveryError>().unwrap().kind,
        DeliveryFailureKind::Unknown
    );
}

#[tokio::test]
async fn wecom_webhook_refuses_redirects() {
    let count = Arc::new(AtomicUsize::new(0));
    let seen = count.clone();
    let target = Router::new().route(
        "/",
        post(move || {
            seen.fetch_add(1, Ordering::SeqCst);
            async { Json(json!({"errcode":0})) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let location = format!("http://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, target).await.unwrap() });
    for status in [301, 302, 303, 307, 308] {
        let response = Response::builder()
            .status(StatusCode::from_u16(status).unwrap())
            .header(header::LOCATION, &location)
            .body(Body::empty())
            .unwrap();
        let f = fixture(vec![response], Duration::ZERO).await;
        assert!(f.sender.send("fixture").await.is_err());
    }
    server.abort();
    assert_eq!(count.load(Ordering::SeqCst), 0);
}
