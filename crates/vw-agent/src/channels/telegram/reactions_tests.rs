use super::TelegramChannel;
use super::message_utils::{TELEGRAM_ACK_REACTIONS, build_telegram_ack_reaction_request};
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::post};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

#[derive(Clone)]
struct ReactionServerState {
    requests: Arc<Mutex<Vec<Value>>>,
    status: StatusCode,
    response_body: String,
}

async fn capture_reaction(
    State(state): State<ReactionServerState>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    state.requests.lock().expect("request lock poisoned").push(payload);
    (state.status, state.response_body)
}

async fn spawn_reaction_server(
    status: StatusCode,
    response_body: &str,
) -> (String, Arc<Mutex<Vec<Value>>>, oneshot::Sender<()>, JoinHandle<()>) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let state = ReactionServerState {
        requests: Arc::clone(&requests),
        status,
        response_body: response_body.to_string(),
    };
    let app = Router::new()
        .route("/bot123:SECRET/setMessageReaction", post(capture_reaction))
        .with_state(state);
    let listener =
        tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.expect("listener should bind");
    let base_url =
        format!("http://{}", listener.local_addr().expect("listener should have local addr"));
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
            .expect("reaction test server should exit cleanly");
    });
    (base_url, requests, shutdown_tx, handle)
}

async fn wait_for_request(requests: &Arc<Mutex<Vec<Value>>>) -> Value {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(payload) = requests.lock().expect("request lock poisoned").last().cloned() {
            return payload;
        }
        assert!(Instant::now() < deadline, "timed out waiting for reaction request");
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

fn unused_local_base_url() -> String {
    let listener =
        std::net::TcpListener::bind("127.0.0.1:0").expect("unused local port should bind");
    let addr = listener.local_addr().expect("unused listener should expose local addr");
    drop(listener);
    format!("http://{addr}")
}

#[test]
fn reaction_request_uses_telegram_emoji_shape() {
    let request = build_telegram_ack_reaction_request("1", 2, "👌");

    assert_eq!(request["reaction"][0]["type"], "emoji");
    assert_eq!(request["reaction"][0]["emoji"], "👌");
}

#[tokio::test(flavor = "current_thread")]
async fn try_add_ack_reaction_nonblocking_posts_expected_payload() {
    let (base_url, requests, shutdown_tx, handle) =
        spawn_reaction_server(StatusCode::OK, "ok").await;
    let channel = TelegramChannel::new("123:SECRET".to_string(), vec!["alice".to_string()], false)
        .with_api_base(base_url);

    channel.try_add_ack_reaction_nonblocking("chat-42".to_string(), 99);

    let payload = wait_for_request(&requests).await;
    let emoji = payload["reaction"][0]["emoji"].as_str().expect("emoji should be a string");

    assert_eq!(payload["chat_id"], "chat-42");
    assert_eq!(payload["message_id"], 99);
    assert_eq!(payload["reaction"][0]["type"], "emoji");
    assert!(TELEGRAM_ACK_REACTIONS.contains(&emoji));

    let _ = shutdown_tx.send(());
    handle.await.expect("reaction server task should join");
}

#[tokio::test(flavor = "current_thread")]
async fn try_add_ack_reaction_nonblocking_still_posts_when_api_returns_error() {
    let (base_url, requests, shutdown_tx, handle) = spawn_reaction_server(
        StatusCode::INTERNAL_SERVER_ERROR,
        "failed https://api.telegram.org/bot123:SECRET/setMessageReaction",
    )
    .await;
    let channel = TelegramChannel::new("123:SECRET".to_string(), vec!["alice".to_string()], false)
        .with_api_base(base_url);

    channel.try_add_ack_reaction_nonblocking("chat-500".to_string(), 0);

    let payload = wait_for_request(&requests).await;

    assert_eq!(payload["chat_id"], "chat-500");
    assert_eq!(payload["message_id"], 0);
    assert_eq!(payload["reaction"][0]["type"], "emoji");

    let _ = shutdown_tx.send(());
    handle.await.expect("reaction server task should join");
}

#[tokio::test(flavor = "current_thread")]
async fn try_add_ack_reaction_nonblocking_ignores_network_failures() {
    let channel = TelegramChannel::new("123:SECRET".to_string(), vec!["alice".to_string()], false)
        .with_api_base(unused_local_base_url());

    channel.try_add_ack_reaction_nonblocking("chat-offline".to_string(), 7);

    tokio::time::sleep(Duration::from_millis(100)).await;
}
// 本文件验证 Telegram 确认反应的请求形状、非阻塞发送与失败容忍。
