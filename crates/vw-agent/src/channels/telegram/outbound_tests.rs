use super::TelegramChannel;
use super::attachments::{TelegramAttachment, TelegramAttachmentKind};
use crate::channels::traits::SendMessage;
use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
struct TestServerState {
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    statuses: Arc<HashMap<String, StatusCode>>,
}

#[derive(Debug, Clone)]
struct CapturedRequest {
    path: String,
    content_type: Option<String>,
    body: String,
}

struct TestServer {
    api_base: String,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    handle: tokio::task::JoinHandle<std::io::Result<()>>,
}

impl TestServer {
    async fn spawn(statuses: &[(&str, StatusCode)]) -> Self {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let statuses = Arc::new(
            statuses
                .iter()
                .map(|(method, status)| ((*method).to_string(), *status))
                .collect::<HashMap<_, _>>(),
        );
        let state = TestServerState { requests: requests.clone(), statuses };
        let app = Router::new().fallback(capture_request).with_state(state);
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind test server");
        let api_base = format!("http://{}", listener.local_addr().expect("server addr"));
        let handle = tokio::spawn(axum::serve(listener, app).into_future());

        Self { api_base, requests, handle }
    }

    fn requests(&self) -> Vec<CapturedRequest> {
        self.requests.lock().expect("requests mutex").clone()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn capture_request(
    State(state): State<TestServerState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, &'static str) {
    let api_method = uri.path().rsplit('/').next().unwrap_or_default().to_string();
    state.requests.lock().expect("requests mutex").push(CapturedRequest {
        path: uri.path().to_string(),
        content_type: headers
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned),
        body: String::from_utf8_lossy(&body).into_owned(),
    });

    let status = state.statuses.get(&api_method).copied().unwrap_or(StatusCode::OK);
    let response = if status.is_success() { "ok" } else { "telegram error" };
    let _ = method;
    (status, response)
}

fn test_channel(api_base: &str) -> TelegramChannel {
    TelegramChannel::new("123:ABC".to_string(), vec!["tester".to_string()], false)
        .with_api_base(api_base.to_string())
}

#[test]
fn outbound_attachment_keeps_kind_and_target() {
    let attachment = TelegramAttachment {
        kind: TelegramAttachmentKind::Document,
        target: "/tmp/report.pdf".to_string(),
    };

    assert_eq!(attachment.kind, TelegramAttachmentKind::Document);
    assert_eq!(attachment.target, "/tmp/report.pdf");
}

#[tokio::test]
async fn send_attachment_uses_url_endpoint_for_each_kind() {
    let server = TestServer::spawn(&[]).await;
    let channel = test_channel(&server.api_base);
    let cases = [
        (TelegramAttachmentKind::Image, "sendPhoto", "photo", "https://example.test/image.jpg"),
        (
            TelegramAttachmentKind::Document,
            "sendDocument",
            "document",
            "https://example.test/report.pdf",
        ),
        (TelegramAttachmentKind::Video, "sendVideo", "video", "https://example.test/video.mp4"),
        (TelegramAttachmentKind::Audio, "sendAudio", "audio", "https://example.test/audio.mp3"),
        (TelegramAttachmentKind::Voice, "sendVoice", "voice", "https://example.test/voice.ogg"),
    ];

    for (kind, _, _, target) in cases {
        let attachment = TelegramAttachment { kind, target: target.to_string() };
        channel
            .send_attachment("chat-42", Some("7"), &attachment)
            .await
            .expect("url attachment should be sent");
    }

    let requests = server.requests();
    assert_eq!(requests.len(), cases.len());

    for (request, (_, method, field, target)) in requests.iter().zip(cases.iter()) {
        assert_eq!(request.path, format!("/bot123:ABC/{method}"));

        let body: serde_json::Value =
            serde_json::from_str(&request.body).expect("json request body should parse");
        assert_eq!(body["chat_id"], "chat-42");
        assert_eq!(body["message_thread_id"], "7");
        assert_eq!(body[*field], *target);
    }
}

#[tokio::test]
async fn send_attachment_falls_back_to_text_when_url_send_fails() {
    let server = TestServer::spawn(&[("sendPhoto", StatusCode::INTERNAL_SERVER_ERROR)]).await;
    let channel = test_channel(&server.api_base);
    let attachment = TelegramAttachment {
        kind: TelegramAttachmentKind::Image,
        target: "https://example.test/fallback.jpg".to_string(),
    };

    channel
        .send_attachment("chat-42", Some("9"), &attachment)
        .await
        .expect("fallback text should be sent");

    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].path, "/bot123:ABC/sendPhoto");
    assert_eq!(requests[1].path, "/bot123:ABC/sendMessage");

    let body: serde_json::Value =
        serde_json::from_str(&requests[1].body).expect("fallback body should parse");
    assert_eq!(body["chat_id"], "chat-42");
    assert_eq!(body["message_thread_id"], "9");
    assert_eq!(body["parse_mode"], "HTML");
    assert_eq!(body["text"], "Image: https://example.test/fallback.jpg");
}

#[tokio::test]
async fn send_attachment_uses_kind_label_when_each_url_send_fails() {
    let server = TestServer::spawn(&[
        ("sendPhoto", StatusCode::BAD_GATEWAY),
        ("sendDocument", StatusCode::BAD_GATEWAY),
        ("sendVideo", StatusCode::BAD_GATEWAY),
        ("sendAudio", StatusCode::BAD_GATEWAY),
        ("sendVoice", StatusCode::BAD_GATEWAY),
    ])
    .await;
    let channel = test_channel(&server.api_base);
    let cases = [
        (TelegramAttachmentKind::Image, "sendPhoto", "Image", "https://example.test/image.jpg"),
        (
            TelegramAttachmentKind::Document,
            "sendDocument",
            "Document",
            "https://example.test/report.pdf",
        ),
        (TelegramAttachmentKind::Video, "sendVideo", "Video", "https://example.test/video.mp4"),
        (TelegramAttachmentKind::Audio, "sendAudio", "Audio", "https://example.test/audio.mp3"),
        (TelegramAttachmentKind::Voice, "sendVoice", "Voice", "https://example.test/voice.ogg"),
    ];

    for (kind, _, _, target) in cases {
        let attachment = TelegramAttachment { kind, target: target.to_string() };
        channel
            .send_attachment("chat-42", None, &attachment)
            .await
            .expect("fallback text should be sent");
    }

    let requests = server.requests();
    assert_eq!(requests.len(), cases.len() * 2);

    for (index, (_, method, label, target)) in cases.iter().enumerate() {
        let media_request = &requests[index * 2];
        let fallback_request = &requests[index * 2 + 1];
        assert_eq!(media_request.path, format!("/bot123:ABC/{method}"));
        assert_eq!(fallback_request.path, "/bot123:ABC/sendMessage");

        let body: serde_json::Value =
            serde_json::from_str(&fallback_request.body).expect("fallback body should parse");
        assert_eq!(body["chat_id"], "chat-42");
        assert_eq!(body["text"], format!("{label}: {target}"));
    }
}

#[tokio::test]
async fn send_attachment_rejects_local_files_without_workspace_dir() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let file_path = tempdir.path().join("report.pdf");
    std::fs::write(&file_path, b"report").expect("write attachment file");
    let channel = TelegramChannel::new("123:ABC".to_string(), vec!["tester".to_string()], false);
    let attachment = TelegramAttachment {
        kind: TelegramAttachmentKind::Document,
        target: file_path.display().to_string(),
    };

    let error = channel
        .send_attachment("chat-42", None, &attachment)
        .await
        .expect_err("local attachment should require workspace dir");

    assert!(error.to_string().contains("workspace_dir is not configured"));
}

#[tokio::test]
async fn send_attachment_rejects_local_files_that_escape_workspace() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let outside = tempfile::tempdir().expect("outside tempdir");
    let file_path = outside.path().join("report.pdf");
    std::fs::write(&file_path, b"report").expect("write outside attachment file");
    let channel = TelegramChannel::new("123:ABC".to_string(), vec!["tester".to_string()], false)
        .with_workspace_dir(workspace.path().to_path_buf());
    let attachment = TelegramAttachment {
        kind: TelegramAttachmentKind::Document,
        target: file_path.display().to_string(),
    };

    let error = channel
        .send_attachment("chat-42", None, &attachment)
        .await
        .expect_err("local attachment should stay inside workspace");

    assert!(error.to_string().contains("escapes workspace"));
}

#[tokio::test]
async fn send_attachment_uses_local_upload_endpoint_for_each_kind() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let server = TestServer::spawn(&[]).await;
    let channel = test_channel(&server.api_base).with_workspace_dir(tempdir.path().to_path_buf());
    let cases = [
        (TelegramAttachmentKind::Image, "sendPhoto", "photo.jpg"),
        (TelegramAttachmentKind::Document, "sendDocument", "report.pdf"),
        (TelegramAttachmentKind::Video, "sendVideo", "video.mp4"),
        (TelegramAttachmentKind::Audio, "sendAudio", "audio.mp3"),
        (TelegramAttachmentKind::Voice, "sendVoice", "voice.ogg"),
    ];

    for (_, _, name) in cases {
        std::fs::write(tempdir.path().join(name), format!("fixture:{name}"))
            .expect("write attachment fixture");
    }

    for (kind, _, name) in cases {
        let attachment = TelegramAttachment { kind, target: name.to_string() };
        channel
            .send_attachment("chat-42", None, &attachment)
            .await
            .expect("local attachment should be uploaded");
    }

    let requests = server.requests();
    assert_eq!(requests.len(), cases.len());

    for (request, (_, method, name)) in requests.iter().zip(cases.iter()) {
        assert_eq!(request.path, format!("/bot123:ABC/{method}"));
        assert!(
            request
                .content_type
                .as_deref()
                .is_some_and(|value| value.contains("multipart/form-data"))
        );
        assert!(request.body.contains("name=\"chat_id\""));
        assert!(request.body.contains("chat-42"));
        assert!(request.body.contains(name));
    }
}

#[tokio::test]
async fn send_outbound_sends_text_then_attachments_and_strips_tool_tags() {
    let server = TestServer::spawn(&[]).await;
    let channel = test_channel(&server.api_base);
    let message = SendMessage::new(
        "hello<tool_call>{\"tool\":\"x\"}</tool_call> world [IMAGE:https://example.test/a.jpg] [DOCUMENT:https://example.test/b.pdf]",
        "chat-42:11",
    );

    channel.send_outbound(&message).await.expect("outbound message should be sent");

    let requests = server.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].path, "/bot123:ABC/sendMessage");
    assert_eq!(requests[1].path, "/bot123:ABC/sendPhoto");
    assert_eq!(requests[2].path, "/bot123:ABC/sendDocument");

    let text_body: serde_json::Value =
        serde_json::from_str(&requests[0].body).expect("text body should parse");
    assert_eq!(text_body["chat_id"], "chat-42");
    assert_eq!(text_body["message_thread_id"], "11");
    assert_eq!(text_body["parse_mode"], "HTML");
    assert_eq!(text_body["text"], "hello world");
    assert!(!requests[0].body.contains("tool_call"));
}

#[tokio::test]
async fn send_outbound_sends_marker_only_attachment_without_empty_text() {
    let server = TestServer::spawn(&[]).await;
    let channel = test_channel(&server.api_base);
    let message = SendMessage::new("[DOCUMENT:https://example.test/report.pdf]", "chat-42:11");

    channel.send_outbound(&message).await.expect("marker-only attachment should be sent");

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/bot123:ABC/sendDocument");

    let body: serde_json::Value =
        serde_json::from_str(&requests[0].body).expect("document body should parse");
    assert_eq!(body["chat_id"], "chat-42");
    assert_eq!(body["message_thread_id"], "11");
    assert_eq!(body["document"], "https://example.test/report.pdf");
}

#[tokio::test]
async fn send_outbound_stops_before_attachments_when_text_send_fails() {
    let server = TestServer::spawn(&[("sendMessage", StatusCode::BAD_REQUEST)]).await;
    let channel = test_channel(&server.api_base);
    let message = SendMessage::new("intro [IMAGE:https://example.test/a.jpg]", "chat-42:11");

    let error = channel
        .send_outbound(&message)
        .await
        .expect_err("text send failure should stop outbound processing");

    assert!(error.to_string().contains("Telegram sendMessage failed"));
    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].path, "/bot123:ABC/sendMessage");
    assert_eq!(requests[1].path, "/bot123:ABC/sendMessage");
}

#[tokio::test]
async fn send_outbound_handles_path_only_attachments_and_plain_text() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let file_path = tempdir.path().join("report.pdf");
    std::fs::write(&file_path, b"report").expect("write attachment file");

    let server = TestServer::spawn(&[]).await;
    let channel = test_channel(&server.api_base).with_workspace_dir(tempdir.path().to_path_buf());

    channel
        .send_outbound(&SendMessage::new(file_path.display().to_string(), "chat-42"))
        .await
        .expect("path-only attachment should be sent");
    channel
        .send_outbound(&SendMessage::new("plain text", "chat-42"))
        .await
        .expect("plain text should be sent");

    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].path, "/bot123:ABC/sendDocument");
    assert_eq!(requests[1].path, "/bot123:ABC/sendMessage");

    let text_body: serde_json::Value =
        serde_json::from_str(&requests[1].body).expect("plain text body should parse");
    assert_eq!(text_body["chat_id"], "chat-42");
    assert_eq!(text_body["text"], "plain text");
    assert!(text_body.get("message_thread_id").is_none());
}
// 本文件验证 Telegram 各类附件的远程发送、本地路径约束与文本降级。
