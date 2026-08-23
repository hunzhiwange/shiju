// 本文件覆盖 Telegram 长轮询、机器人身份获取、确认动作与停止条件。

use super::TelegramChannel;
use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
};
use serde_json::{Value, json};
use std::{collections::VecDeque, sync::Arc, time::Duration};
use tokio::{
    net::TcpListener,
    sync::{Mutex, Notify},
    task::JoinHandle,
};

#[derive(Clone, Debug)]
enum MockReply {
    Json(StatusCode, Value),
    Text(StatusCode, String),
    DelayedJson(Duration, StatusCode, Value),
}

impl MockReply {
    fn ok_json(body: Value) -> Self {
        Self::Json(StatusCode::OK, body)
    }
}

#[derive(Default)]
struct MockTelegramData {
    get_updates_replies: VecDeque<MockReply>,
    get_me_replies: VecDeque<MockReply>,
    get_updates_bodies: Vec<Value>,
    send_chat_action_bodies: Vec<Value>,
    ack_reaction_bodies: Vec<Value>,
    get_me_count: usize,
    total_request_count: usize,
}

#[derive(Clone, Default)]
struct MockTelegramState {
    inner: Arc<Mutex<MockTelegramData>>,
    notify: Arc<Notify>,
}

impl MockTelegramState {
    async fn push_get_updates_reply(&self, reply: MockReply) {
        self.inner.lock().await.get_updates_replies.push_back(reply);
    }

    async fn push_get_me_reply(&self, reply: MockReply) {
        self.inner.lock().await.get_me_replies.push_back(reply);
    }

    async fn total_request_count(&self) -> usize {
        self.inner.lock().await.total_request_count
    }

    async fn get_me_count(&self) -> usize {
        self.inner.lock().await.get_me_count
    }

    async fn get_updates_bodies(&self) -> Vec<Value> {
        self.inner.lock().await.get_updates_bodies.clone()
    }

    async fn send_chat_action_bodies(&self) -> Vec<Value> {
        self.inner.lock().await.send_chat_action_bodies.clone()
    }

    async fn ack_reaction_bodies(&self) -> Vec<Value> {
        self.inner.lock().await.ack_reaction_bodies.clone()
    }

    async fn wait_for_request_count(&self, expected: usize) {
        loop {
            let notified = self.notify.notified();
            if self.total_request_count().await >= expected {
                return;
            }
            notified.await;
        }
    }
}

async fn handle_mock_request(
    State(state): State<MockTelegramState>,
    request: Request<Body>,
) -> Response {
    let path = request.uri().path().to_string();
    let body_bytes =
        to_bytes(request.into_body(), usize::MAX).await.expect("request body should be readable");
    let body_json = if body_bytes.is_empty() {
        None
    } else {
        Some(
            serde_json::from_slice::<Value>(&body_bytes)
                .expect("request body should be valid json"),
        )
    };

    let reply = {
        let mut inner = state.inner.lock().await;
        inner.total_request_count += 1;

        let method = path.rsplit('/').next().unwrap_or_default();
        match method {
            "getUpdates" => {
                if let Some(body) = body_json {
                    inner.get_updates_bodies.push(body);
                }
                inner.get_updates_replies.pop_front().unwrap_or_else(|| {
                    MockReply::ok_json(json!({
                        "ok": true,
                        "result": []
                    }))
                })
            }
            "getMe" => {
                inner.get_me_count += 1;
                inner.get_me_replies.pop_front().unwrap_or_else(|| {
                    MockReply::ok_json(json!({
                        "ok": true,
                        "result": { "username": "testbot" }
                    }))
                })
            }
            "sendChatAction" => {
                if let Some(body) = body_json {
                    inner.send_chat_action_bodies.push(body);
                }
                MockReply::ok_json(json!({ "ok": true, "result": true }))
            }
            "setMessageReaction" => {
                if let Some(body) = body_json {
                    inner.ack_reaction_bodies.push(body);
                }
                MockReply::ok_json(json!({ "ok": true, "result": true }))
            }
            _ => MockReply::Json(StatusCode::NOT_FOUND, json!({ "ok": false })),
        }
    };

    state.notify.notify_waiters();

    match reply {
        MockReply::Json(status, body) => (status, Json(body)).into_response(),
        MockReply::Text(status, body) => (status, body).into_response(),
        MockReply::DelayedJson(delay, status, body) => {
            tokio::time::sleep(delay).await;
            (status, Json(body)).into_response()
        }
    }
}

// 脚本服务器记录轮询与确认请求，复现更新偏移和停止时序。
async fn spawn_mock_server(state: MockTelegramState) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("mock listener should bind");
    let address = listener.local_addr().expect("listener should have local addr");
    let app = Router::new().fallback(any(handle_mock_request)).with_state(state);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock server should stay alive");
    });
    (format!("http://{address}"), server)
}

fn text_update(
    update_id: i64,
    chat_id: i64,
    message_id: i64,
    sender_id: i64,
    username: &str,
    text: &str,
    chat_type: &str,
) -> Value {
    json!({
        "update_id": update_id,
        "message": {
            "message_id": message_id,
            "text": text,
            "from": {
                "id": sender_id,
                "username": username
            },
            "chat": {
                "id": chat_id,
                "type": chat_type
            }
        }
    })
}

fn approval_callback_update(
    update_id: i64,
    chat_id: i64,
    message_id: i64,
    sender_id: i64,
    username: &str,
    data: &str,
) -> Value {
    json!({
        "update_id": update_id,
        "callback_query": {
            "id": format!("callback-{update_id}"),
            "data": data,
            "from": {
                "id": sender_id,
                "username": username
            },
            "message": {
                "message_id": message_id,
                "chat": {
                    "id": chat_id,
                    "type": "private"
                }
            }
        }
    })
}

#[tokio::test]
async fn listen_impl_advances_offset_and_returns_when_sender_closed() {
    let state = MockTelegramState::default();
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": [{ "update_id": 41 }]
        })))
        .await;
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": [text_update(42, 100, 7, 1, "alice", "hello", "private")]
        })))
        .await;

    let (base_url, server) = spawn_mock_server(state.clone()).await;
    let channel = TelegramChannel::new("token".to_string(), vec!["*".to_string()], false)
        .with_api_base(base_url);
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    drop(rx);

    channel.listen_impl(tx).await.expect("closed receiver should stop polling");
    state.wait_for_request_count(4).await;

    let get_updates_bodies = state.get_updates_bodies().await;
    assert_eq!(get_updates_bodies.len(), 2);
    assert_eq!(get_updates_bodies[0]["offset"], json!(0));
    assert_eq!(get_updates_bodies[0]["timeout"], json!(0));
    assert_eq!(get_updates_bodies[1]["offset"], json!(42));
    assert_eq!(get_updates_bodies[1]["timeout"], json!(30));

    let send_chat_action_bodies = state.send_chat_action_bodies().await;
    assert_eq!(send_chat_action_bodies.len(), 1);
    assert_eq!(send_chat_action_bodies[0]["chat_id"], json!("100"));
    assert_eq!(send_chat_action_bodies[0]["action"], json!("typing"));

    let ack_reaction_bodies = state.ack_reaction_bodies().await;
    assert_eq!(ack_reaction_bodies.len(), 1);
    assert_eq!(ack_reaction_bodies[0]["chat_id"], json!("100"));
    assert_eq!(ack_reaction_bodies[0]["message_id"], json!(7));

    server.abort();
}

#[tokio::test]
async fn listen_impl_continues_after_successful_send_with_next_offset() {
    let state = MockTelegramState::default();
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": []
        })))
        .await;
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": [text_update(50, 200, 9, 1, "alice", "first", "private")]
        })))
        .await;
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": [text_update(51, 200, 10, 1, "alice", "second", "private")]
        })))
        .await;

    let (base_url, server) = spawn_mock_server(state.clone()).await;
    let channel = TelegramChannel::new("token".to_string(), vec!["*".to_string()], false)
        .with_api_base(base_url);
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    let listen = tokio::spawn(async move { channel.listen_impl(tx).await });
    let msg = rx.recv().await.expect("first poll message should be delivered");
    assert_eq!(msg.content, "first");
    assert_eq!(msg.reply_target, "200");
    drop(rx);

    listen.await.expect("listen task should finish").expect("closed receiver should stop polling");

    let get_updates_bodies = state.get_updates_bodies().await;
    assert_eq!(get_updates_bodies.len(), 3);
    assert_eq!(get_updates_bodies[1]["offset"], json!(0));
    assert_eq!(get_updates_bodies[2]["offset"], json!(51));

    let send_chat_action_bodies = state.send_chat_action_bodies().await;
    assert_eq!(send_chat_action_bodies.len(), 2);

    server.abort();
}

#[tokio::test]
async fn listen_impl_delivers_approval_callback_queries() {
    let state = MockTelegramState::default();
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": []
        })))
        .await;
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": [
                approval_callback_update(60, 300, 11, 1, "alice", "zcapr:yes:req-123")
            ]
        })))
        .await;
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": [
                text_update(61, 300, 12, 1, "alice", "after callback", "private")
            ]
        })))
        .await;

    let (base_url, server) = spawn_mock_server(state.clone()).await;
    let channel = TelegramChannel::new("token".to_string(), vec!["alice".to_string()], false)
        .with_api_base(base_url);
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    let listen = tokio::spawn(async move { channel.listen_impl(tx).await });
    let msg = rx.recv().await.expect("callback query should be delivered");
    assert_eq!(msg.id, "telegram_cb_300_11_callback-60");
    assert_eq!(msg.sender, "alice");
    assert_eq!(msg.reply_target, "300");
    assert_eq!(msg.content, "/approve-allow req-123");
    drop(rx);

    listen.await.expect("listen task should finish").expect("closed receiver should stop polling");

    let get_updates_bodies = state.get_updates_bodies().await;
    assert_eq!(get_updates_bodies.len(), 3);
    assert_eq!(get_updates_bodies[2]["offset"], json!(61));

    let send_chat_action_bodies = state.send_chat_action_bodies().await;
    assert_eq!(send_chat_action_bodies.len(), 2);
    assert_eq!(send_chat_action_bodies[0]["chat_id"], json!("300"));

    let ack_reaction_bodies = state.ack_reaction_bodies().await;
    assert_eq!(ack_reaction_bodies.len(), 1);
    assert_eq!(ack_reaction_bodies[0]["chat_id"], json!("300"));
    assert_eq!(ack_reaction_bodies[0]["message_id"], json!(12));

    server.abort();
}

#[tokio::test(start_paused = true)]
async fn listen_impl_retries_startup_probe_errors_before_entering_main_loop() {
    let state = MockTelegramState::default();
    state.push_get_updates_reply(MockReply::Text(StatusCode::OK, "{invalid".to_string())).await;
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": false,
            "error_code": 409
        })))
        .await;
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": false,
            "error_code": 500,
            "description": "boom"
        })))
        .await;
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": []
        })))
        .await;
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": [text_update(1, 321, 8, 1, "alice", "startup recovered", "private")]
        })))
        .await;

    let (base_url, server) = spawn_mock_server(state.clone()).await;
    let channel = TelegramChannel::new("token".to_string(), vec!["*".to_string()], false)
        .with_api_base(base_url);
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    drop(rx);

    let listen = tokio::spawn(async move { channel.listen_impl(tx).await });
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(16)).await;
    tokio::task::yield_now().await;

    listen.await.expect("listen task should finish").expect("closed receiver should stop polling");

    let get_updates_bodies = state.get_updates_bodies().await;
    assert_eq!(get_updates_bodies.len(), 5);
    assert_eq!(get_updates_bodies[4]["timeout"], json!(30));

    server.abort();
}

#[tokio::test(start_paused = true)]
async fn listen_impl_uses_defaults_for_sparse_api_responses() {
    let state = MockTelegramState::default();
    state.push_get_updates_reply(MockReply::ok_json(json!({}))).await;
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": [{}, { "update_id": 70 }]
        })))
        .await;
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "result": [text_update(71, 400, 13, 1, "alice", "sparse response", "private")]
        })))
        .await;

    let (base_url, server) = spawn_mock_server(state.clone()).await;
    let channel = TelegramChannel::new("token".to_string(), vec!["*".to_string()], false)
        .with_api_base(base_url);
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    drop(rx);

    let listen = tokio::spawn(async move { channel.listen_impl(tx).await });
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(6)).await;
    tokio::task::yield_now().await;

    listen.await.expect("listen task should finish").expect("closed receiver should stop polling");

    let get_updates_bodies = state.get_updates_bodies().await;
    assert_eq!(get_updates_bodies.len(), 3);
    assert_eq!(get_updates_bodies[2]["offset"], json!(71));

    server.abort();
}

#[tokio::test(start_paused = true)]
async fn listen_impl_retries_main_loop_errors_and_skips_unauthorized_updates() {
    let state = MockTelegramState::default();
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": []
        })))
        .await;
    state.push_get_updates_reply(MockReply::Text(StatusCode::OK, "{invalid".to_string())).await;
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": false,
            "error_code": 409,
            "description": "conflict"
        })))
        .await;
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": false,
            "error_code": 500,
            "description": "boom"
        })))
        .await;
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": [
                text_update(2, 777, 10, 9, "mallory", "unauthorized", "private"),
                text_update(3, 777, 11, 1, "alice", "authorized", "private")
            ]
        })))
        .await;

    let (base_url, server) = spawn_mock_server(state.clone()).await;
    let channel = TelegramChannel::new("token".to_string(), vec!["alice".to_string()], false)
        .with_api_base(base_url);
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    drop(rx);

    let listen = tokio::spawn(async move { channel.listen_impl(tx).await });
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(46)).await;
    tokio::task::yield_now().await;

    listen.await.expect("listen task should finish").expect("closed receiver should stop polling");

    let get_updates_bodies = state.get_updates_bodies().await;
    assert_eq!(get_updates_bodies.len(), 5);
    assert_eq!(get_updates_bodies[1]["timeout"], json!(30));

    let send_chat_action_bodies = state.send_chat_action_bodies().await;
    assert_eq!(send_chat_action_bodies.len(), 1);
    assert_eq!(send_chat_action_bodies[0]["chat_id"], json!("777"));

    state.wait_for_request_count(7).await;
    let ack_reaction_bodies = state.ack_reaction_bodies().await;
    assert_eq!(ack_reaction_bodies.len(), 1);
    assert_eq!(ack_reaction_bodies[0]["message_id"], json!(11));

    server.abort();
}

#[tokio::test(start_paused = true)]
async fn listen_impl_prefetches_and_recovers_bot_username_in_mention_only_mode() {
    let state = MockTelegramState::default();
    state
        .push_get_me_reply(MockReply::Json(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "ok": false }),
        ))
        .await;
    state
        .push_get_me_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": { "username": "mybot" }
        })))
        .await;
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": []
        })))
        .await;
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": [text_update(4, -1001, 12, 1, "alice", "@mybot ping", "group")]
        })))
        .await;

    let (base_url, server) = spawn_mock_server(state.clone()).await;
    let channel = TelegramChannel::new("token".to_string(), vec!["*".to_string()], true)
        .with_api_base(base_url);
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    drop(rx);

    channel.listen_impl(tx).await.expect("closed receiver should stop polling");

    assert_eq!(state.get_me_count().await, 2);
    assert_eq!(channel.bot_username.lock().as_deref(), Some("mybot"));

    let send_chat_action_bodies = state.send_chat_action_bodies().await;
    assert_eq!(send_chat_action_bodies.len(), 1);
    assert_eq!(send_chat_action_bodies[0]["chat_id"], json!("-1001"));

    server.abort();
}

#[tokio::test(start_paused = true)]
async fn listen_impl_handles_startup_network_errors() {
    let channel = TelegramChannel::new("token".to_string(), vec!["*".to_string()], false)
        .with_api_base("http://127.0.0.1:1".to_string());
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    drop(rx);

    let listen = tokio::spawn(async move { channel.listen_impl(tx).await });
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(6)).await;
    tokio::task::yield_now().await;

    assert!(!listen.is_finished(), "network retry loop should keep running");
    listen.abort();
}

#[tokio::test(start_paused = true)]
async fn listen_impl_handles_main_poll_network_errors_after_startup() {
    let state = MockTelegramState::default();
    state
        .push_get_updates_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": []
        })))
        .await;

    let (base_url, server) = spawn_mock_server(state.clone()).await;
    let channel = TelegramChannel::new("token".to_string(), vec!["*".to_string()], false)
        .with_api_base(base_url);
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    drop(rx);

    let listen = tokio::spawn(async move { channel.listen_impl(tx).await });
    state.wait_for_request_count(1).await;
    server.abort();
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(6)).await;
    tokio::task::yield_now().await;

    assert!(!listen.is_finished(), "main poll network retry loop should keep running");
    listen.abort();
}

#[tokio::test]
async fn health_check_impl_returns_true_for_successful_status() {
    let state = MockTelegramState::default();
    state
        .push_get_me_reply(MockReply::ok_json(json!({
            "ok": true,
            "result": { "username": "healthy_bot" }
        })))
        .await;
    let (base_url, server) = spawn_mock_server(state).await;
    let channel = TelegramChannel::new("token".to_string(), vec!["*".to_string()], false)
        .with_api_base(base_url);

    assert!(channel.health_check_impl().await);

    server.abort();
}

#[tokio::test]
async fn health_check_impl_returns_false_when_request_fails() {
    let channel = TelegramChannel::new("token".to_string(), vec!["*".to_string()], false)
        .with_api_base("http://127.0.0.1:1".to_string());

    assert!(!channel.health_check_impl().await);
}

#[tokio::test]
async fn health_check_impl_returns_false_for_unsuccessful_status() {
    let state = MockTelegramState::default();
    state
        .push_get_me_reply(MockReply::Json(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "ok": false }),
        ))
        .await;
    let (base_url, server) = spawn_mock_server(state).await;
    let channel = TelegramChannel::new("token".to_string(), vec!["*".to_string()], false)
        .with_api_base(base_url);

    assert!(!channel.health_check_impl().await);

    server.abort();
}

#[tokio::test(start_paused = true)]
async fn health_check_impl_returns_false_on_timeout() {
    let state = MockTelegramState::default();
    state
        .push_get_me_reply(MockReply::DelayedJson(
            Duration::from_secs(10),
            StatusCode::OK,
            json!({
                "ok": true,
                "result": { "username": "slow_bot" }
            }),
        ))
        .await;
    let (base_url, server) = spawn_mock_server(state).await;
    let channel = TelegramChannel::new("token".to_string(), vec!["*".to_string()], false)
        .with_api_base(base_url);

    let health = tokio::spawn(async move { channel.health_check_impl().await });
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(5)).await;
    tokio::task::yield_now().await;

    assert!(!health.await.expect("health task should finish"));

    server.abort();
}
