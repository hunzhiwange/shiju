// 本文件验证 Telegram 审批回调解析、请求载荷与异步清理调度。

use super::TelegramChannel;
use axum::{
    Router,
    body::Bytes,
    extract::{OriginalUri, State},
    http::StatusCode,
    routing::post,
};
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};

#[test]
fn approval_callback_parser_rejects_unknown_prefix() {
    assert!(TelegramChannel::parse_approval_callback_command("other:abc").is_none());
}

#[derive(Clone, Debug)]
struct RecordedRequest {
    path: String,
    body: serde_json::Value,
}

struct TestServerState {
    requests: Mutex<Vec<RecordedRequest>>,
}

struct TestServer {
    base_url: String,
    state: Arc<TestServerState>,
    shutdown: Option<oneshot::Sender<()>>,
}

impl TestServer {
    async fn spawn() -> Self {
        let state = Arc::new(TestServerState { requests: Mutex::new(Vec::new()) });
        let app = Router::new().route("/{*path}", post(record_request)).with_state(state.clone());
        let listener =
            tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.expect("bind test server");
        let addr = listener.local_addr().expect("server addr");
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("serve test server");
        });

        Self { base_url: format!("http://{addr}"), state, shutdown: Some(shutdown_tx) }
    }

    async fn requests(&self) -> Vec<RecordedRequest> {
        self.state.requests.lock().await.clone()
    }

    async fn wait_for_requests(&self, expected: usize) -> Vec<RecordedRequest> {
        for _ in 0..50 {
            let requests = self.requests().await;
            if requests.len() >= expected {
                return requests;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        self.requests().await
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

async fn record_request(
    State(state): State<Arc<TestServerState>>,
    uri: OriginalUri,
    body: Bytes,
) -> StatusCode {
    let body = serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::Value::Null);
    state.requests.lock().await.push(RecordedRequest { path: uri.path().to_string(), body });
    StatusCode::OK
}

fn callback_update(username: &str, user_id: i64, data: &str) -> serde_json::Value {
    serde_json::json!({
        "callback_query": {
            "id": "cb-123",
            "data": data,
            "from": {"id": user_id, "username": username},
            "message": {
                "message_id": 77,
                "message_thread_id": 9,
                "chat": {"id": -100}
            }
        }
    })
}

fn callback_update_without_thread_or_message_id(user_id: i64, data: &str) -> serde_json::Value {
    serde_json::json!({
        "callback_query": {
            "id": "cb-plain",
            "data": data,
            "from": {"id": user_id},
            "message": {
                "chat": {"id": 42}
            }
        }
    })
}

#[test]
fn approval_callback_parser_maps_approve_and_deny() {
    assert_eq!(
        TelegramChannel::parse_approval_callback_command("zcapr:yes: req-1 ").as_deref(),
        Some("/approve-allow req-1")
    );
    assert_eq!(
        TelegramChannel::parse_approval_callback_command("zcapr:no:req-2").as_deref(),
        Some("/approve-deny req-2")
    );
    assert!(TelegramChannel::parse_approval_callback_command("zcapr:yes:   ").is_none());
    assert!(TelegramChannel::parse_approval_callback_command("zcapr:no:").is_none());
}

#[tokio::test]
async fn nonblocking_callback_helpers_post_expected_bodies() {
    let server = TestServer::spawn().await;
    let channel = TelegramChannel::new("123:ABC".to_string(), vec!["*".to_string()], false)
        .with_api_base(server.base_url.clone());

    channel.answer_callback_query_nonblocking("cb-1".to_string(), "ok");
    channel.clear_callback_inline_keyboard_nonblocking(
        "chat-1".to_string(),
        42,
        Some("7".to_string()),
    );

    let requests = server.wait_for_requests(2).await;
    assert_eq!(requests[0].path, "/bot123:ABC/answerCallbackQuery");
    assert_eq!(requests[0].body["callback_query_id"], "cb-1");
    assert_eq!(requests[0].body["text"], "ok");
    assert_eq!(requests[0].body["show_alert"], false);

    assert_eq!(requests[1].path, "/bot123:ABC/editMessageReplyMarkup");
    assert_eq!(requests[1].body["chat_id"], "chat-1");
    assert_eq!(requests[1].body["message_id"], 42);
    assert_eq!(requests[1].body["message_thread_id"], "7");
    assert_eq!(requests[1].body["reply_markup"]["inline_keyboard"], serde_json::json!([]));
}

#[tokio::test]
// 审批回调既生成渠道消息，也安排按钮清理以免重复操作。
async fn approval_callback_query_builds_channel_message_and_schedules_cleanup() {
    let server = TestServer::spawn().await;
    let channel = TelegramChannel::new("123:ABC".to_string(), vec!["tester".to_string()], false)
        .with_api_base(server.base_url.clone());

    let message = channel
        .try_parse_approval_callback_query(&callback_update("tester", 123, "zcapr:no:req-9"))
        .expect("callback should parse");

    assert_eq!(message.id, "telegram_cb_-100_77_cb-123");
    assert_eq!(message.sender, "tester");
    assert_eq!(message.reply_target, "-100:9");
    assert_eq!(message.content, "/approve-deny req-9");
    assert_eq!(message.thread_ts.as_deref(), Some("9"));

    let requests = server.wait_for_requests(2).await;
    assert_eq!(requests.len(), 2);
}

#[tokio::test]
async fn approval_callback_query_uses_sender_id_and_plain_reply_target() {
    let server = TestServer::spawn().await;
    let channel = TelegramChannel::new("123:ABC".to_string(), vec!["555".to_string()], false)
        .with_api_base(server.base_url.clone());

    let message = channel
        .try_parse_approval_callback_query(&callback_update_without_thread_or_message_id(
            555,
            "zcapr:yes:req-plain",
        ))
        .expect("callback should parse");

    assert_eq!(message.id, "telegram_cb_42_0_cb-plain");
    assert_eq!(message.sender, "555");
    assert_eq!(message.reply_target, "42");
    assert_eq!(message.content, "/approve-allow req-plain");
    assert_eq!(message.channel, "telegram");
    assert_eq!(message.thread_ts, None);

    let requests = server.wait_for_requests(2).await;
    let cleanup = requests
        .iter()
        .find(|request| request.path == "/bot123:ABC/editMessageReplyMarkup")
        .expect("cleanup request should be sent");
    assert_eq!(cleanup.body["chat_id"], "42");
    assert_eq!(cleanup.body["message_id"], 0);
    assert!(cleanup.body.get("message_thread_id").is_none());
}

#[test]
fn approval_callback_query_rejects_unknown_or_unauthorized_callbacks() {
    let channel = TelegramChannel::new("123:ABC".to_string(), vec!["tester".to_string()], false);

    assert!(
        channel
            .try_parse_approval_callback_query(&callback_update("tester", 123, "other:req"))
            .is_none()
    );
    assert!(
        channel
            .try_parse_approval_callback_query(&callback_update("intruder", 999, "zcapr:yes:req"))
            .is_none()
    );
    assert!(channel.try_parse_approval_callback_query(&serde_json::json!({})).is_none());
}

#[test]
fn approval_callback_query_rejects_missing_required_fields() {
    let channel = TelegramChannel::new("123:ABC".to_string(), vec!["*".to_string()], false);
    let updates = [
        serde_json::json!({
            "callback_query": {
                "data": "zcapr:yes:req",
                "from": {"id": 1},
                "message": {"message_id": 1, "chat": {"id": 1}}
            }
        }),
        serde_json::json!({
            "callback_query": {
                "id": "cb",
                "from": {"id": 1},
                "message": {"message_id": 1, "chat": {"id": 1}}
            }
        }),
        serde_json::json!({
            "callback_query": {
                "id": "cb",
                "data": "zcapr:yes:req",
                "from": {"id": 1}
            }
        }),
        serde_json::json!({
            "callback_query": {
                "id": "cb",
                "data": "zcapr:yes:req",
                "from": {"id": 1},
                "message": {"message_id": 1, "chat": {}}
            }
        }),
    ];

    for update in updates {
        assert!(channel.try_parse_approval_callback_query(&update).is_none());
    }
}
