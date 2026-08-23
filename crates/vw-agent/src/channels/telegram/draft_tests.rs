use super::TelegramChannel;
use super::message_utils::TELEGRAM_MAX_MESSAGE_LENGTH;
use crate::app::agent::channels::traits::SendMessage;
use crate::app::agent::config::StreamMode;
use axum::{
    Router,
    body::Bytes,
    extract::{OriginalUri, State},
    http::StatusCode,
    routing::post,
};
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::{Mutex, oneshot};

#[test]
fn new_channel_has_no_draft_edit_timestamps() {
    let channel = TelegramChannel::new("token".to_string(), vec![], true);

    assert!(channel.last_draft_edit.lock().is_empty());
}

#[derive(Clone, Debug)]
struct RecordedRequest {
    path: String,
    body: serde_json::Value,
}

#[derive(Clone, Debug)]
struct ResponseSpec {
    status: StatusCode,
    body: &'static str,
}

impl ResponseSpec {
    fn ok_json(body: &'static str) -> Self {
        Self { status: StatusCode::OK, body }
    }

    fn ok() -> Self {
        Self::ok_json(r#"{"ok":true,"result":{"message_id":222}}"#)
    }

    fn bad() -> Self {
        Self { status: StatusCode::BAD_REQUEST, body: "bad bot123:SECRET" }
    }
}

struct TestServerState {
    requests: Mutex<Vec<RecordedRequest>>,
    responses: Mutex<VecDeque<ResponseSpec>>,
}

struct TestServer {
    base_url: String,
    state: Arc<TestServerState>,
    shutdown: Option<oneshot::Sender<()>>,
}

impl TestServer {
    async fn spawn(responses: Vec<ResponseSpec>) -> Self {
        let state = Arc::new(TestServerState {
            requests: Mutex::new(Vec::new()),
            responses: Mutex::new(VecDeque::from(responses)),
        });
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
) -> (StatusCode, String) {
    let body = serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::Value::Null);
    state.requests.lock().await.push(RecordedRequest { path: uri.path().to_string(), body });

    let response = state.responses.lock().await.pop_front().unwrap_or_else(ResponseSpec::ok);
    (response.status, response.body.to_string())
}

fn streaming_channel(base_url: &str) -> TelegramChannel {
    TelegramChannel::new("123:SECRET".to_string(), vec!["tester".to_string()], false)
        .with_api_base(base_url.to_string())
        .with_streaming(StreamMode::Partial, 0)
}

#[tokio::test]
async fn send_draft_returns_none_when_streaming_disabled() {
    let channel = TelegramChannel::new("token".to_string(), vec!["tester".to_string()], false);

    let result =
        channel.send_draft_impl(&SendMessage::new("hello", "chat-1")).await.expect("draft send");

    assert_eq!(result, None);
    assert!(channel.last_draft_edit.lock().is_empty());
}

#[tokio::test]
async fn send_draft_posts_initial_text_and_records_edit_time() {
    let server = TestServer::spawn(vec![ResponseSpec::ok_json(
        r#"{"ok":true,"result":{"message_id":123}}"#,
    )])
    .await;
    let channel = streaming_channel(&server.base_url);

    let message_id =
        channel.send_draft_impl(&SendMessage::new("", "chat-1:9")).await.expect("draft send");

    assert_eq!(message_id.as_deref(), Some("123"));
    assert!(channel.last_draft_edit.lock().contains_key("chat-1"));

    let requests = server.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/bot123:SECRET/sendMessage");
    assert_eq!(requests[0].body["chat_id"], "chat-1");
    assert_eq!(requests[0].body["message_thread_id"], "9");
    assert_eq!(requests[0].body["text"], "...");
}

#[tokio::test]
async fn send_draft_uses_content_without_thread_and_allows_missing_message_id() {
    let server = TestServer::spawn(vec![ResponseSpec::ok_json(r#"{"ok":true,"result":{}}"#)]).await;
    let channel = streaming_channel(&server.base_url);

    let message_id = channel
        .send_draft_impl(&SendMessage::new("draft text", "chat-1"))
        .await
        .expect("draft send");

    assert_eq!(message_id, None);
    assert!(channel.last_draft_edit.lock().contains_key("chat-1"));

    let requests = server.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/bot123:SECRET/sendMessage");
    assert_eq!(requests[0].body["chat_id"], "chat-1");
    assert!(requests[0].body.get("message_thread_id").is_none());
    assert_eq!(requests[0].body["text"], "draft text");
}

#[tokio::test]
async fn send_draft_reports_sanitized_non_success_response() {
    let server = TestServer::spawn(vec![ResponseSpec::bad()]).await;
    let channel = streaming_channel(&server.base_url);

    let error = channel
        .send_draft_impl(&SendMessage::new("hello", "chat-1"))
        .await
        .expect_err("bad status should fail")
        .to_string();

    assert!(error.contains("Telegram sendMessage (draft) failed"));
    assert!(!error.contains("SECRET"));
}

#[tokio::test]
async fn update_draft_skips_when_throttled_or_message_id_invalid() {
    let server = TestServer::spawn(Vec::new()).await;
    let channel = TelegramChannel::new("123:SECRET".to_string(), vec!["tester".to_string()], false)
        .with_api_base(server.base_url.clone())
        .with_streaming(StreamMode::Partial, 60_000);
    channel.last_draft_edit.lock().insert("chat-1".to_string(), std::time::Instant::now());

    assert_eq!(channel.update_draft_impl("chat-1", "123", "hello").await.expect("throttled"), None);
    assert_eq!(
        channel.update_draft_impl("chat-2", "not-a-number", "hello").await.expect("invalid id"),
        None
    );
    assert!(server.requests().await.is_empty());
}

#[tokio::test]
async fn update_draft_posts_edit_request_and_truncates_utf8_text() {
    let server = TestServer::spawn(vec![ResponseSpec::ok()]).await;
    let channel = streaming_channel(&server.base_url);
    let text = format!("{}{}", "a".repeat(4095), "éé");

    channel.update_draft_impl("chat-1", "321", &text).await.expect("update draft");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/bot123:SECRET/editMessageText");
    assert_eq!(requests[0].body["chat_id"], "chat-1");
    assert_eq!(requests[0].body["message_id"], 321);
    assert_eq!(requests[0].body["text"].as_str().expect("text").len(), 4095);
    assert!(channel.last_draft_edit.lock().contains_key("chat-1"));
}

#[tokio::test]
async fn update_draft_ignores_failed_edit_without_recording_edit_time() {
    let server = TestServer::spawn(vec![ResponseSpec::bad()]).await;
    let channel = streaming_channel(&server.base_url);

    channel.update_draft_impl("chat-1", "321", "hello").await.expect("update draft");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/bot123:SECRET/editMessageText");
    assert_eq!(requests[0].body["chat_id"], "chat-1");
    assert_eq!(requests[0].body["message_id"], 321);
    assert_eq!(requests[0].body["text"], "hello");
    assert!(!channel.last_draft_edit.lock().contains_key("chat-1"));
}

#[tokio::test]
async fn finalize_draft_edits_html_successfully() {
    let server = TestServer::spawn(vec![ResponseSpec::ok()]).await;
    let channel = streaming_channel(&server.base_url);

    channel.finalize_draft_impl("chat-1", "55", "**done**").await.expect("finalize html");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/bot123:SECRET/editMessageText");
    assert_eq!(requests[0].body["chat_id"], "chat-1");
    assert_eq!(requests[0].body["message_id"], 55);
    assert_eq!(requests[0].body["parse_mode"], "HTML");
    assert_eq!(requests[0].body["text"], "<b>done</b>");
}

#[tokio::test]
async fn finalize_draft_edits_html_then_plain_text_fallback() {
    let server = TestServer::spawn(vec![ResponseSpec::bad(), ResponseSpec::ok()]).await;
    let channel = streaming_channel(&server.base_url);
    channel.last_draft_edit.lock().insert("chat-1".to_string(), std::time::Instant::now());

    channel.finalize_draft_impl("chat-1", "55", "**done**").await.expect("finalize fallback");

    assert!(!channel.last_draft_edit.lock().contains_key("chat-1"));
    let requests = server.requests().await;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].body["parse_mode"], "HTML");
    assert_eq!(requests[0].body["text"], "<b>done</b>");
    assert!(requests[1].body.get("parse_mode").is_none());
    assert_eq!(requests[1].body["text"], "**done**");
}

#[tokio::test]
async fn finalize_draft_falls_back_to_send_message_when_edits_fail() {
    let server =
        TestServer::spawn(vec![ResponseSpec::bad(), ResponseSpec::bad(), ResponseSpec::ok()]).await;
    let channel = streaming_channel(&server.base_url);

    channel
        .finalize_draft_impl("chat-1:8", "55", "<tool_call>hidden</tool_call>visible")
        .await
        .expect("finalize send fallback");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].path, "/bot123:SECRET/editMessageText");
    assert_eq!(requests[0].body["parse_mode"], "HTML");
    assert_eq!(requests[1].path, "/bot123:SECRET/editMessageText");
    assert!(requests[1].body.get("parse_mode").is_none());
    assert_eq!(requests[2].path, "/bot123:SECRET/sendMessage");
    assert_eq!(requests[2].body["message_thread_id"], "8");
    assert_eq!(requests[2].body["text"], "visible");
}

#[tokio::test]
async fn finalize_draft_with_invalid_id_sends_new_text_message() {
    let server = TestServer::spawn(vec![ResponseSpec::ok()]).await;
    let channel = streaming_channel(&server.base_url);

    channel.finalize_draft_impl("chat-1:8", "invalid", "hello").await.expect("finalize invalid id");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/bot123:SECRET/sendMessage");
    assert_eq!(requests[0].body["chat_id"], "chat-1");
    assert_eq!(requests[0].body["message_thread_id"], "8");
}

#[tokio::test]
async fn finalize_draft_with_long_text_deletes_draft_and_sends_chunks() {
    let server =
        TestServer::spawn(vec![ResponseSpec::ok(), ResponseSpec::ok(), ResponseSpec::ok()]).await;
    let channel = streaming_channel(&server.base_url);
    let text = "a".repeat(TELEGRAM_MAX_MESSAGE_LENGTH + 1);

    channel.finalize_draft_impl("chat-1:8", "55", &text).await.expect("finalize long text");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].path, "/bot123:SECRET/deleteMessage");
    assert_eq!(requests[0].body["message_id"], 55);
    assert_eq!(requests[1].path, "/bot123:SECRET/sendMessage");
    assert_eq!(requests[1].body["message_thread_id"], "8");
    assert!(requests[1].body["text"].as_str().expect("first chunk").contains("(continues...)"));
    assert_eq!(requests[2].path, "/bot123:SECRET/sendMessage");
    assert!(requests[2].body["text"].as_str().expect("second chunk").contains("(continued)"));
}

#[tokio::test]
async fn finalize_draft_with_attachments_deletes_draft_and_resends_parts() {
    let server =
        TestServer::spawn(vec![ResponseSpec::ok(), ResponseSpec::ok(), ResponseSpec::ok()]).await;
    let channel = streaming_channel(&server.base_url);

    channel
        .finalize_draft_impl("chat-1", "55", "caption [IMAGE:https://example.test/a.jpg]")
        .await
        .expect("finalize attachments");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].path, "/bot123:SECRET/deleteMessage");
    assert_eq!(requests[1].path, "/bot123:SECRET/sendMessage");
    assert_eq!(requests[2].path, "/bot123:SECRET/sendPhoto");
}

#[tokio::test]
async fn finalize_draft_with_only_attachment_skips_empty_text_message() {
    let server = TestServer::spawn(vec![ResponseSpec::ok(), ResponseSpec::ok()]).await;
    let channel = streaming_channel(&server.base_url);

    channel
        .finalize_draft_impl("chat-1", "55", "[IMAGE:https://example.test/a.jpg]")
        .await
        .expect("finalize attachment");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].path, "/bot123:SECRET/deleteMessage");
    assert_eq!(requests[1].path, "/bot123:SECRET/sendPhoto");
}

#[tokio::test]
async fn finalize_draft_with_attachment_and_invalid_id_skips_delete() {
    let server = TestServer::spawn(vec![ResponseSpec::ok()]).await;
    let channel = streaming_channel(&server.base_url);

    channel
        .finalize_draft_impl("chat-1", "invalid", "[IMAGE:https://example.test/a.jpg]")
        .await
        .expect("finalize attachment with invalid draft id");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/bot123:SECRET/sendPhoto");
}

#[tokio::test]
async fn cancel_draft_removes_state_and_deletes_valid_message() {
    let server = TestServer::spawn(vec![ResponseSpec::bad()]).await;
    let channel = streaming_channel(&server.base_url);
    channel.last_draft_edit.lock().insert("chat-1".to_string(), std::time::Instant::now());

    channel.cancel_draft_impl("chat-1", "44").await.expect("cancel draft");
    channel.cancel_draft_impl("chat-2", "invalid").await.expect("cancel invalid");

    assert!(!channel.last_draft_edit.lock().contains_key("chat-1"));
    let requests = server.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/bot123:SECRET/deleteMessage");
    assert_eq!(requests[0].body["message_id"], 44);
}
// 本文件验证 Telegram 流式草稿的创建、更新节流与文本回退。
