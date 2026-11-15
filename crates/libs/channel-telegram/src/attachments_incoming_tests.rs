// 本文件验证 Telegram 入站附件元数据选择、下载与消息内容拼装。

use super::TelegramChannel;
use super::attachments::{IncomingAttachmentKind, TELEGRAM_MAX_FILE_DOWNLOAD_BYTES};
use axum::{
    Router,
    body::Body,
    extract::{OriginalUri, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::Response,
    routing::get,
};
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::{Mutex, oneshot};

#[derive(Clone, Debug)]
struct RecordedRequest {
    path_and_query: String,
}

#[derive(Clone, Debug)]
struct ResponseSpec {
    status: StatusCode,
    body: Vec<u8>,
    content_type: &'static str,
}

impl ResponseSpec {
    fn json(body: &str) -> Self {
        Self {
            status: StatusCode::OK,
            body: body.as_bytes().to_vec(),
            content_type: "application/json",
        }
    }

    fn bytes(body: &[u8]) -> Self {
        Self {
            status: StatusCode::OK,
            body: body.to_vec(),
            content_type: "application/octet-stream",
        }
    }

    fn status(status: StatusCode) -> Self {
        Self { status, body: Vec::new(), content_type: "text/plain" }
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
        let app = Router::new().route("/{*path}", get(record_request)).with_state(state.clone());
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
) -> Response<Body> {
    state.requests.lock().await.push(RecordedRequest {
        path_and_query: uri.path_and_query().map(ToString::to_string).unwrap_or_default(),
    });

    let response = state
        .responses
        .lock()
        .await
        .pop_front()
        .expect("test should provide one response per request");

    Response::builder()
        .status(response.status)
        .header(CONTENT_TYPE, response.content_type)
        .body(Body::from(response.body))
        .expect("response should build")
}

fn test_channel(base_url: &str, workspace: &std::path::Path) -> TelegramChannel {
    TelegramChannel::new("123:ABC".to_string(), vec!["tester".to_string()], false)
        .with_api_base(base_url.to_string())
        .with_workspace_dir(workspace.to_path_buf())
}

#[test]
// 照片存在多个尺寸变体，入站解析选择信息最完整的一项。
fn parse_attachment_metadata_selects_largest_photo_variant() {
    let message = serde_json::json!({
        "photo": [
            {"file_id": "small", "file_size": 10},
            {"file_id": "large", "file_size": 20}
        ],
        "caption": "hello"
    });

    let attachment = TelegramChannel::parse_attachment_metadata(&message).unwrap();

    assert_eq!(attachment.kind, IncomingAttachmentKind::Photo);
    assert_eq!(attachment.file_id, "large");
    assert_eq!(attachment.caption.as_deref(), Some("hello"));
}

#[test]
fn parse_attachment_metadata_extracts_document_fields() {
    let message = serde_json::json!({
        "document": {
            "file_id": "doc-id",
            "file_name": "../report.pdf",
            "file_size": 42
        },
        "caption": "monthly report"
    });

    let attachment = TelegramChannel::parse_attachment_metadata(&message).unwrap();

    assert_eq!(attachment.kind, IncomingAttachmentKind::Document);
    assert_eq!(attachment.file_id, "doc-id");
    assert_eq!(attachment.file_name.as_deref(), Some("../report.pdf"));
    assert_eq!(attachment.file_size, Some(42));
    assert_eq!(attachment.caption.as_deref(), Some("monthly report"));
}

#[test]
fn parse_attachment_metadata_rejects_missing_file_ids_and_empty_photos() {
    assert!(
        TelegramChannel::parse_attachment_metadata(&serde_json::json!({
            "document": {"file_name": "report.pdf"}
        }))
        .is_none()
    );
    assert!(
        TelegramChannel::parse_attachment_metadata(&serde_json::json!({"photo": []})).is_none()
    );
    assert!(
        TelegramChannel::parse_attachment_metadata(&serde_json::json!({"text": "hi"})).is_none()
    );
}

#[test]
fn parse_attachment_metadata_rejects_non_string_file_ids() {
    assert!(
        TelegramChannel::parse_attachment_metadata(&serde_json::json!({
            "document": {"file_id": 123, "file_name": "report.pdf"}
        }))
        .is_none()
    );
    assert!(
        TelegramChannel::parse_attachment_metadata(&serde_json::json!({
            "photo": [{"file_id": "small"}, {"file_id": 123}]
        }))
        .is_none()
    );
}

#[tokio::test]
async fn try_parse_attachment_message_downloads_document_to_workspace() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let server = TestServer::spawn(vec![
        ResponseSpec::json(r#"{"ok":true,"result":{"file_path":"docs/report.pdf"}}"#),
        ResponseSpec::bytes(b"report-data"),
    ])
    .await;
    let channel = test_channel(&server.base_url, tempdir.path());
    let update = serde_json::json!({
        "message": {
            "message_id": 44,
            "date": 1700000000,
            "chat": {"id": 99, "type": "private"},
            "from": {"id": 123, "username": "tester"},
            "document": {"file_id": "doc-id", "file_name": "../report.pdf", "file_size": 11},
            "caption": "monthly report"
        }
    });

    let message = channel
        .try_parse_attachment_message(&update)
        .await
        .expect("attachment message should parse");

    assert_eq!(message.id, "telegram_99_44");
    assert_eq!(message.sender, "tester");
    assert_eq!(message.reply_target, "99");
    assert_eq!(message.channel, "telegram");
    assert!(message.content.contains("[Document: report.pdf]"));
    assert!(message.content.contains("monthly report"));

    let saved = tempdir.path().join("telegram_files/report.pdf");
    assert_eq!(tokio::fs::read(&saved).await.expect("saved file"), b"report-data");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].path_and_query, "/bot123:ABC/getFile?file_id=doc-id");
    assert_eq!(requests[1].path_and_query, "/file/bot123:ABC/docs/report.pdf");
}

#[tokio::test]
async fn try_parse_attachment_message_uses_fallback_document_filename_and_quote() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let server = TestServer::spawn(vec![
        ResponseSpec::json(r#"{"ok":true,"result":{"file_path":"docs/original.bin"}}"#),
        ResponseSpec::bytes(b"data"),
    ])
    .await;
    let channel = test_channel(&server.base_url, tempdir.path());
    let update = serde_json::json!({
        "message": {
            "message_id": 5,
            "chat": {"id": 77, "type": "private"},
            "from": {"id": 123, "username": "tester"},
            "reply_to_message": {
                "message_id": 4,
                "from": {"first_name": "Alice"},
                "text": "line one\nline two"
            },
            "document": {"file_id": "doc-id", "file_name": "../", "file_size": 4},
            "caption": ""
        }
    });

    let message = channel
        .try_parse_attachment_message(&update)
        .await
        .expect("attachment message should parse");

    assert_eq!(message.id, "telegram_77_5");
    assert!(message.content.starts_with("> @Alice:\n> line one\n> line two\n\n"));
    assert!(message.content.contains("[Document: attachment_77_5.bin]"));
    assert!(!message.content.ends_with("\n\n"));

    let saved = tempdir.path().join("telegram_files/attachment_77_5.bin");
    assert_eq!(tokio::fs::read(&saved).await.expect("saved file"), b"data");
}

#[tokio::test]
async fn try_parse_attachment_message_generates_photo_filename_and_thread_target() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let server = TestServer::spawn(vec![
        ResponseSpec::json(r#"{"ok":true,"result":{"file_path":"photos/pic.WeBp"}}"#),
        ResponseSpec::bytes(b"photo-data"),
    ])
    .await;
    let mut channel = test_channel(&server.base_url, tempdir.path());
    channel.mention_only = true;
    *channel.bot_username.lock() = Some("vibebot".to_string());

    let update = serde_json::json!({
        "message": {
            "message_id": 7,
            "message_thread_id": 88,
            "chat": {"id": -100, "type": "supergroup"},
            "from": {"id": 123, "username": "tester"},
            "photo": [{"file_id": "small"}, {"file_id": "photo-id", "file_size": 10}],
            "caption": "@vibebot see this"
        }
    });

    let message =
        channel.try_parse_attachment_message(&update).await.expect("mentioned photo should parse");

    assert_eq!(message.reply_target, "-100:88");
    assert_eq!(message.thread_ts.as_deref(), Some("88"));
    assert!(message.content.contains("[IMAGE:"));
    assert!(message.content.contains("@vibebot see this"));

    let saved = tempdir.path().join("telegram_files/photo_-100_7.webp");
    assert_eq!(tokio::fs::read(&saved).await.expect("saved photo"), b"photo-data");
}

#[tokio::test]
async fn try_parse_attachment_message_sanitizes_photo_extension_without_suffix() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let server = TestServer::spawn(vec![
        ResponseSpec::json(r#"{"ok":true,"result":{"file_path":"photos/file."}}"#),
        ResponseSpec::bytes(b"photo-data"),
    ])
    .await;
    let channel = test_channel(&server.base_url, tempdir.path());
    let update = serde_json::json!({
        "message": {
            "message_id": 9,
            "chat": {"id": 42, "type": "private"},
            "from": {"id": 123, "username": "tester"},
            "photo": [{"file_id": "photo-id", "file_size": 10}]
        }
    });

    let message = channel.try_parse_attachment_message(&update).await.expect("photo should parse");

    assert!(message.content.contains("[IMAGE:"));
    let saved = tempdir.path().join("telegram_files/photo_42_9.jpg");
    assert_eq!(tokio::fs::read(&saved).await.expect("saved photo"), b"photo-data");
}

#[tokio::test]
async fn try_parse_attachment_message_returns_none_when_get_file_path_fails() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let server = TestServer::spawn(vec![ResponseSpec::json(r#"{"ok":true,"result":{}}"#)]).await;
    let channel = test_channel(&server.base_url, tempdir.path());
    let update = serde_json::json!({
        "message": {
            "message_id": 1,
            "chat": {"id": 2, "type": "private"},
            "from": {"id": 123, "username": "tester"},
            "document": {"file_id": "doc-id", "file_name": "report.pdf", "file_size": 4}
        }
    });

    assert!(channel.try_parse_attachment_message(&update).await.is_none());

    let requests = server.requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path_and_query, "/bot123:ABC/getFile?file_id=doc-id");
}

#[tokio::test]
async fn try_parse_attachment_message_returns_none_when_download_fails() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let server = TestServer::spawn(vec![
        ResponseSpec::json(r#"{"ok":true,"result":{"file_path":"docs/report.pdf"}}"#),
        ResponseSpec::status(StatusCode::INTERNAL_SERVER_ERROR),
    ])
    .await;
    let channel = test_channel(&server.base_url, tempdir.path());
    let update = serde_json::json!({
        "message": {
            "message_id": 1,
            "chat": {"id": 2, "type": "private"},
            "from": {"id": 123, "username": "tester"},
            "document": {"file_id": "doc-id", "file_name": "report.pdf", "file_size": 4}
        }
    });

    assert!(channel.try_parse_attachment_message(&update).await.is_none());
    assert!(!tempdir.path().join("telegram_files/report.pdf").exists());

    let requests = server.requests().await;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].path_and_query, "/file/bot123:ABC/docs/report.pdf");
}

#[tokio::test]
async fn try_parse_attachment_message_filters_before_downloading() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let server = TestServer::spawn(Vec::new()).await;
    let channel = test_channel(&server.base_url, tempdir.path());

    let oversized = serde_json::json!({
        "message": {
            "chat": {"id": 1, "type": "private"},
            "from": {"id": 123, "username": "tester"},
            "document": {
                "file_id": "too-large",
                "file_name": "huge.bin",
                "file_size": TELEGRAM_MAX_FILE_DOWNLOAD_BYTES + 1
            }
        }
    });
    assert!(channel.try_parse_attachment_message(&oversized).await.is_none());

    let unauthorized = serde_json::json!({
        "message": {
            "chat": {"id": 1, "type": "private"},
            "from": {"id": 999, "username": "intruder"},
            "document": {"file_id": "doc", "file_name": "report.pdf", "file_size": 1}
        }
    });
    assert!(channel.try_parse_attachment_message(&unauthorized).await.is_none());

    assert!(server.requests().await.is_empty());
}

#[tokio::test]
async fn try_parse_attachment_message_requires_group_mention_when_configured() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let server = TestServer::spawn(Vec::new()).await;
    let mut channel = test_channel(&server.base_url, tempdir.path());
    channel.mention_only = true;
    *channel.bot_username.lock() = Some("vibebot".to_string());

    let update = serde_json::json!({
        "message": {
            "chat": {"id": -100, "type": "group"},
            "from": {"id": 123, "username": "tester"},
            "document": {"file_id": "doc", "file_name": "report.pdf", "file_size": 1},
            "caption": "not for the bot"
        }
    });

    assert!(channel.try_parse_attachment_message(&update).await.is_none());
    assert!(server.requests().await.is_empty());
}

#[tokio::test]
async fn try_parse_attachment_message_rejects_group_when_bot_username_is_unavailable() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let server = TestServer::spawn(Vec::new()).await;
    let mut channel = test_channel(&server.base_url, tempdir.path());
    channel.mention_only = true;

    let update = serde_json::json!({
        "message": {
            "chat": {"id": -100, "type": "group"},
            "from": {"id": 123, "username": "tester"},
            "document": {"file_id": "doc", "file_name": "report.pdf", "file_size": 1},
            "caption": "@vibebot see this"
        }
    });

    assert!(channel.try_parse_attachment_message(&update).await.is_none());
    assert!(server.requests().await.is_empty());
}

#[tokio::test]
async fn try_parse_attachment_message_requires_workspace_dir() {
    let channel = TelegramChannel::new("123:ABC".to_string(), vec!["tester".to_string()], false);
    let update = serde_json::json!({
        "message": {
            "chat": {"id": 1, "type": "private"},
            "from": {"id": 123, "username": "tester"},
            "document": {"file_id": "doc", "file_name": "report.pdf", "file_size": 1}
        }
    });

    assert!(channel.try_parse_attachment_message(&update).await.is_none());
}
