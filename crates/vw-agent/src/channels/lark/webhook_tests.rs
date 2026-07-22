use super::*;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use std::time::Duration;
use tower::ServiceExt;

fn make_channel(port: Option<u16>, allowed_users: Vec<String>) -> LarkChannel {
    LarkChannel::new(
        "app-id".to_string(),
        "secret".to_string(),
        "verify".to_string(),
        port,
        allowed_users,
        false,
    )
}

fn text_payload(message_id: Option<&str>) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "header": {"event_type": "im.message.receive_v1"},
        "event": {
            "sender": {"sender_id": {"open_id": "ou_allowed"}},
            "message": {
                "message_type": "text",
                "chat_type": "p2p",
                "chat_id": "oc_chat",
                "create_time": "1710000000123",
                "content": "{\"text\":\"hello webhook\"}"
            }
        }
    });
    if let Some(message_id) = message_id {
        payload["event"]["message"]["message_id"] = serde_json::json!(message_id);
    }
    payload
}

async fn post_payload(
    channel: &LarkChannel,
    tx: tokio::sync::mpsc::Sender<super::super::ChannelMessage>,
    payload: serde_json::Value,
) -> axum::response::Response {
    build_lark_webhook_router(build_app_state(channel, tx))
        .oneshot(
            Request::post("/lark")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn response_body_json(response: axum::response::Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn listen_http_requires_port_before_binding_server() {
    let channel = make_channel(None, vec!["*".to_string()]);
    let (tx, _rx) = tokio::sync::mpsc::channel(1);

    let err = channel.listen_http(tx).await.expect_err("missing port should fail");

    assert!(err.to_string().contains("requires `port`"));
}

#[tokio::test]
async fn listen_http_binds_configured_port_until_cancelled() {
    let channel = make_channel(Some(0), vec!["*".to_string()]);
    let (tx, _rx) = tokio::sync::mpsc::channel(1);

    let task = tokio::spawn(async move { channel.listen_http(tx).await });
    tokio::time::sleep(Duration::from_millis(20)).await;

    assert!(!task.is_finished());
    task.abort();
    let err = task.await.expect_err("listener should be cancelled");
    assert!(err.is_cancelled());
}

#[tokio::test]
async fn handle_event_accepts_matching_challenge_token() {
    let channel = make_channel(Some(0), vec!["*".to_string()]);
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    let response = post_payload(
        &channel,
        tx,
        serde_json::json!({"challenge": "challenge-value", "token": "verify"}),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body_json(response).await;
    assert_eq!(body["challenge"], "challenge-value");
}

#[tokio::test]
async fn handle_event_accepts_challenge_without_token() {
    let channel = make_channel(Some(0), vec!["*".to_string()]);
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    let response =
        post_payload(&channel, tx, serde_json::json!({"challenge": "challenge-value"})).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_body_json(response).await;
    assert_eq!(body["challenge"], "challenge-value");
}

#[tokio::test]
async fn handle_event_rejects_mismatched_challenge_token() {
    let channel = make_channel(Some(0), vec!["*".to_string()]);
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    let response = post_payload(
        &channel,
        tx,
        serde_json::json!({"challenge": "challenge-value", "token": "wrong"}),
    )
    .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn handle_event_dispatches_text_message() {
    let channel = make_channel(Some(0), vec!["ou_allowed".to_string()]);
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let response = post_payload(&channel, tx, text_payload(Some("om_message"))).await;

    assert_eq!(response.status(), StatusCode::OK);
    let message = rx.recv().await.expect("message should be dispatched");
    assert_eq!(message.sender, "oc_chat");
    assert_eq!(message.reply_target, "oc_chat");
    assert_eq!(message.content, "hello webhook");
    assert_eq!(message.channel, "lark");
    assert_eq!(message.timestamp, 1_710_000_000);
}

#[tokio::test]
async fn handle_event_returns_ok_when_receiver_is_closed() {
    let channel = make_channel(Some(0), vec!["ou_allowed".to_string()]);
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    drop(rx);

    let response = post_payload(&channel, tx, text_payload(None)).await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn handle_event_ignores_non_message_event_without_dispatch() {
    let channel = make_channel(Some(0), vec!["*".to_string()]);
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    let response = post_payload(
        &channel,
        tx,
        serde_json::json!({"header": {"event_type": "contact.user.created_v3"}}),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(rx.try_recv().is_err());
}
