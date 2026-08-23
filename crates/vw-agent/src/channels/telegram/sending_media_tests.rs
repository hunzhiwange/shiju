// 本文件验证 Telegram 媒体发送的 JSON 与上传请求、可选字段及错误回退。

use super::TelegramChannel;
use axum::{
    Router,
    body::Bytes,
    extract::{OriginalUri, State},
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
    routing::post,
};
use serde_json::Value;
use std::{collections::VecDeque, sync::Arc};
use tempfile::TempDir;
use tokio::sync::{Mutex, oneshot};

#[derive(Clone, Debug)]
struct RecordedRequest {
    path: String,
    content_type: String,
    body: Vec<u8>,
}

impl RecordedRequest {
    fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("request body should be valid JSON")
    }
}

#[derive(Debug)]
struct ResponseSpec {
    status: StatusCode,
    body: String,
}

impl ResponseSpec {
    fn ok() -> Self {
        Self { status: StatusCode::OK, body: r#"{"ok":true}"#.to_string() }
    }

    fn telegram_failure() -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: "failed https://api.telegram.org/bot123:SECRET/sendMessage".to_string(),
        }
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
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind telegram media test server");
        let addr = listener.local_addr().expect("telegram media test server addr");
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("serve telegram media test server");
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
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, String) {
    state.requests.lock().await.push(RecordedRequest {
        path: uri.path().to_string(),
        content_type: headers
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string(),
        body: body.to_vec(),
    });

    let response = state
        .responses
        .lock()
        .await
        .pop_front()
        .expect("test should provide one response per request");

    (response.status, response.body)
}

fn test_channel(base_url: &str) -> TelegramChannel {
    TelegramChannel::new("123:SECRET".to_string(), vec!["*".to_string()], false)
        .with_api_base(base_url.to_string())
}

fn write_temp_file(dir: &TempDir, name: &str, contents: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, contents).expect("temporary test file should be written");
    path
}

fn assert_json_request(
    request: &RecordedRequest,
    method: &str,
    media_field: &str,
    chat_id: &str,
    thread_id: &str,
    url: &str,
    caption: &str,
) {
    assert_eq!(request.path, format!("/bot123:SECRET/{method}"));
    assert!(
        request.content_type.starts_with("application/json"),
        "expected JSON content type, got {}",
        request.content_type
    );

    let body = request.json();
    assert_eq!(body["chat_id"], chat_id);
    assert_eq!(body[media_field], url);
    assert_eq!(body["message_thread_id"], thread_id);
    assert_eq!(body["caption"], caption);
}

// 断言区分缺省字段与空值，防止发送无意义参数。
fn assert_json_request_without_optional_fields(
    request: &RecordedRequest,
    method: &str,
    media_field: &str,
    chat_id: &str,
    url: &str,
) {
    assert_eq!(request.path, format!("/bot123:SECRET/{method}"));
    assert!(
        request.content_type.starts_with("application/json"),
        "expected JSON content type, got {}",
        request.content_type
    );

    let body = request.json();
    assert_eq!(body["chat_id"], chat_id);
    assert_eq!(body[media_field], url);
    assert!(body.get("message_thread_id").is_none());
    assert!(body.get("caption").is_none());
}

fn assert_multipart_request(
    request: &RecordedRequest,
    method: &str,
    media_field: &str,
    chat_id: &str,
    thread_id: &str,
    caption: &str,
    file_name: &str,
    file_contents: &str,
) {
    assert_eq!(request.path, format!("/bot123:SECRET/{method}"));
    assert!(
        request.content_type.starts_with("multipart/form-data; boundary="),
        "expected multipart content type, got {}",
        request.content_type
    );

    let body = request.body_text();
    assert!(body.contains(r#"name="chat_id""#));
    assert!(body.contains(chat_id));
    assert!(body.contains(r#"name="message_thread_id""#));
    assert!(body.contains(thread_id));
    assert!(body.contains(r#"name="caption""#));
    assert!(body.contains(caption));
    assert!(body.contains(&format!(r#"name="{media_field}"; filename="{file_name}""#)));
    assert!(body.contains(file_contents));
}

fn assert_multipart_request_without_optional_fields(
    request: &RecordedRequest,
    method: &str,
    media_field: &str,
    chat_id: &str,
    file_name: &str,
    file_contents: &str,
) {
    assert_eq!(request.path, format!("/bot123:SECRET/{method}"));
    assert!(
        request.content_type.starts_with("multipart/form-data; boundary="),
        "expected multipart content type, got {}",
        request.content_type
    );

    let body = request.body_text();
    assert!(body.contains(r#"name="chat_id""#));
    assert!(body.contains(chat_id));
    assert!(!body.contains(r#"name="message_thread_id""#));
    assert!(!body.contains(r#"name="caption""#));
    assert!(body.contains(&format!(r#"name="{media_field}"; filename="{file_name}""#)));
    assert!(body.contains(file_contents));
}

fn assert_sanitized_error(error: anyhow::Error, expected_prefix: &str) {
    let message = error.to_string();
    assert!(message.contains(expected_prefix), "unexpected error message: {message}");
    assert!(message.contains("bot[redacted]"), "error should redact bot token: {message}");
    assert!(!message.contains("SECRET"), "error should not expose bot token: {message}");
}

#[tokio::test]
async fn url_sending_methods_omit_optional_fields_when_absent() {
    let server = TestServer::spawn(vec![
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
    ])
    .await;
    let channel = test_channel(&server.base_url);

    channel
        .send_document_by_url("chat-document", None, "https://example.test/document.pdf", None)
        .await
        .expect("document URL send should succeed without optional fields");
    channel
        .send_photo_by_url("chat-photo", None, "https://example.test/photo.jpg", None)
        .await
        .expect("photo URL send should succeed without optional fields");
    channel
        .send_video_by_url("chat-video", None, "https://example.test/video.mp4", None)
        .await
        .expect("video URL send should succeed without optional fields");
    channel
        .send_audio_by_url("chat-audio", None, "https://example.test/audio.mp3", None)
        .await
        .expect("audio URL send should succeed without optional fields");
    channel
        .send_voice_by_url("chat-voice", None, "https://example.test/voice.ogg", None)
        .await
        .expect("voice URL send should succeed without optional fields");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 5);
    assert_json_request_without_optional_fields(
        &requests[0],
        "sendDocument",
        "document",
        "chat-document",
        "https://example.test/document.pdf",
    );
    assert_json_request_without_optional_fields(
        &requests[1],
        "sendPhoto",
        "photo",
        "chat-photo",
        "https://example.test/photo.jpg",
    );
    assert_json_request_without_optional_fields(
        &requests[2],
        "sendVideo",
        "video",
        "chat-video",
        "https://example.test/video.mp4",
    );
    assert_json_request_without_optional_fields(
        &requests[3],
        "sendAudio",
        "audio",
        "chat-audio",
        "https://example.test/audio.mp3",
    );
    assert_json_request_without_optional_fields(
        &requests[4],
        "sendVoice",
        "voice",
        "chat-voice",
        "https://example.test/voice.ogg",
    );
}

#[tokio::test]
async fn url_sending_methods_send_expected_json_payloads() {
    let server = TestServer::spawn(vec![
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
    ])
    .await;
    let channel = test_channel(&server.base_url);

    channel
        .send_document_by_url(
            "chat-document",
            Some("thread-document"),
            "https://example.test/document.pdf",
            Some("document caption"),
        )
        .await
        .expect("document URL send should succeed");
    channel
        .send_photo_by_url(
            "chat-photo",
            Some("thread-photo"),
            "https://example.test/photo.jpg",
            Some("photo caption"),
        )
        .await
        .expect("photo URL send should succeed");
    channel
        .send_video_by_url(
            "chat-video",
            Some("thread-video"),
            "https://example.test/video.mp4",
            Some("video caption"),
        )
        .await
        .expect("video URL send should succeed");
    channel
        .send_audio_by_url(
            "chat-audio",
            Some("thread-audio"),
            "https://example.test/audio.mp3",
            Some("audio caption"),
        )
        .await
        .expect("audio URL send should succeed");
    channel
        .send_voice_by_url(
            "chat-voice",
            Some("thread-voice"),
            "https://example.test/voice.ogg",
            Some("voice caption"),
        )
        .await
        .expect("voice URL send should succeed");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 5);
    assert_json_request(
        &requests[0],
        "sendDocument",
        "document",
        "chat-document",
        "thread-document",
        "https://example.test/document.pdf",
        "document caption",
    );
    assert_json_request(
        &requests[1],
        "sendPhoto",
        "photo",
        "chat-photo",
        "thread-photo",
        "https://example.test/photo.jpg",
        "photo caption",
    );
    assert_json_request(
        &requests[2],
        "sendVideo",
        "video",
        "chat-video",
        "thread-video",
        "https://example.test/video.mp4",
        "video caption",
    );
    assert_json_request(
        &requests[3],
        "sendAudio",
        "audio",
        "chat-audio",
        "thread-audio",
        "https://example.test/audio.mp3",
        "audio caption",
    );
    assert_json_request(
        &requests[4],
        "sendVoice",
        "voice",
        "chat-voice",
        "thread-voice",
        "https://example.test/voice.ogg",
        "voice caption",
    );
}

#[tokio::test]
async fn url_sending_methods_redact_tokens_in_error_messages() {
    let server = TestServer::spawn(vec![
        ResponseSpec::telegram_failure(),
        ResponseSpec::telegram_failure(),
        ResponseSpec::telegram_failure(),
        ResponseSpec::telegram_failure(),
        ResponseSpec::telegram_failure(),
    ])
    .await;
    let channel = test_channel(&server.base_url);

    let document_error = channel
        .send_document_by_url(
            "chat",
            Some("thread"),
            "https://example.test/document.pdf",
            Some("cap"),
        )
        .await
        .expect_err("document URL send should fail");
    assert_sanitized_error(document_error, "Telegram sendDocument by URL failed");

    let photo_error = channel
        .send_photo_by_url("chat", Some("thread"), "https://example.test/photo.jpg", Some("cap"))
        .await
        .expect_err("photo URL send should fail");
    assert_sanitized_error(photo_error, "Telegram sendPhoto by URL failed");

    let video_error = channel
        .send_video_by_url("chat", Some("thread"), "https://example.test/video.mp4", Some("cap"))
        .await
        .expect_err("video URL send should fail");
    assert_sanitized_error(video_error, "Telegram sendVideo by URL failed");

    let audio_error = channel
        .send_audio_by_url("chat", Some("thread"), "https://example.test/audio.mp3", Some("cap"))
        .await
        .expect_err("audio URL send should fail");
    assert_sanitized_error(audio_error, "Telegram sendAudio by URL failed");

    let voice_error = channel
        .send_voice_by_url("chat", Some("thread"), "https://example.test/voice.ogg", Some("cap"))
        .await
        .expect_err("voice URL send should fail");
    assert_sanitized_error(voice_error, "Telegram sendVoice by URL failed");
}

#[tokio::test]
async fn multipart_sending_methods_send_expected_form_data() {
    let dir = tempfile::tempdir().expect("tempdir");
    let document_path = write_temp_file(&dir, "report.txt", "document file body");
    let photo_path = write_temp_file(&dir, "photo.jpg", "photo file body");
    let video_path = write_temp_file(&dir, "video.mp4", "video file body");
    let audio_path = write_temp_file(&dir, "audio.mp3", "audio file body");
    let voice_path = write_temp_file(&dir, "voice.ogg", "voice file body");

    let server = TestServer::spawn(vec![
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
    ])
    .await;
    let channel = test_channel(&server.base_url);

    channel
        .send_document(
            "chat-document",
            Some("thread-document"),
            &document_path,
            Some("document caption"),
        )
        .await
        .expect("document file send should succeed");
    channel
        .send_document_bytes(
            "chat-document-bytes",
            Some("thread-document-bytes"),
            b"document bytes body".to_vec(),
            "document-bytes.txt",
            Some("document bytes caption"),
        )
        .await
        .expect("document bytes send should succeed");
    channel
        .send_photo("chat-photo", Some("thread-photo"), &photo_path, Some("photo caption"))
        .await
        .expect("photo file send should succeed");
    channel
        .send_photo_bytes(
            "chat-photo-bytes",
            Some("thread-photo-bytes"),
            b"photo bytes body".to_vec(),
            "photo-bytes.jpg",
            Some("photo bytes caption"),
        )
        .await
        .expect("photo bytes send should succeed");
    channel
        .send_video("chat-video", Some("thread-video"), &video_path, Some("video caption"))
        .await
        .expect("video file send should succeed");
    channel
        .send_audio("chat-audio", Some("thread-audio"), &audio_path, Some("audio caption"))
        .await
        .expect("audio file send should succeed");
    channel
        .send_voice("chat-voice", Some("thread-voice"), &voice_path, Some("voice caption"))
        .await
        .expect("voice file send should succeed");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 7);
    assert_multipart_request(
        &requests[0],
        "sendDocument",
        "document",
        "chat-document",
        "thread-document",
        "document caption",
        "report.txt",
        "document file body",
    );
    assert_multipart_request(
        &requests[1],
        "sendDocument",
        "document",
        "chat-document-bytes",
        "thread-document-bytes",
        "document bytes caption",
        "document-bytes.txt",
        "document bytes body",
    );
    assert_multipart_request(
        &requests[2],
        "sendPhoto",
        "photo",
        "chat-photo",
        "thread-photo",
        "photo caption",
        "photo.jpg",
        "photo file body",
    );
    assert_multipart_request(
        &requests[3],
        "sendPhoto",
        "photo",
        "chat-photo-bytes",
        "thread-photo-bytes",
        "photo bytes caption",
        "photo-bytes.jpg",
        "photo bytes body",
    );
    assert_multipart_request(
        &requests[4],
        "sendVideo",
        "video",
        "chat-video",
        "thread-video",
        "video caption",
        "video.mp4",
        "video file body",
    );
    assert_multipart_request(
        &requests[5],
        "sendAudio",
        "audio",
        "chat-audio",
        "thread-audio",
        "audio caption",
        "audio.mp3",
        "audio file body",
    );
    assert_multipart_request(
        &requests[6],
        "sendVoice",
        "voice",
        "chat-voice",
        "thread-voice",
        "voice caption",
        "voice.ogg",
        "voice file body",
    );
}

#[tokio::test]
async fn multipart_sending_methods_omit_optional_fields_when_absent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let document_path = write_temp_file(&dir, "report.txt", "document file body");
    let photo_path = write_temp_file(&dir, "photo.jpg", "photo file body");
    let video_path = write_temp_file(&dir, "video.mp4", "video file body");
    let audio_path = write_temp_file(&dir, "audio.mp3", "audio file body");
    let voice_path = write_temp_file(&dir, "voice.ogg", "voice file body");

    let server = TestServer::spawn(vec![
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
    ])
    .await;
    let channel = test_channel(&server.base_url);

    channel
        .send_document("chat-document", None, &document_path, None)
        .await
        .expect("document file send should succeed without optional fields");
    channel
        .send_document_bytes(
            "chat-document-bytes",
            None,
            b"document bytes body".to_vec(),
            "document-bytes.txt",
            None,
        )
        .await
        .expect("document bytes send should succeed without optional fields");
    channel
        .send_photo("chat-photo", None, &photo_path, None)
        .await
        .expect("photo file send should succeed without optional fields");
    channel
        .send_photo_bytes(
            "chat-photo-bytes",
            None,
            b"photo bytes body".to_vec(),
            "photo-bytes.jpg",
            None,
        )
        .await
        .expect("photo bytes send should succeed without optional fields");
    channel
        .send_video("chat-video", None, &video_path, None)
        .await
        .expect("video file send should succeed without optional fields");
    channel
        .send_audio("chat-audio", None, &audio_path, None)
        .await
        .expect("audio file send should succeed without optional fields");
    channel
        .send_voice("chat-voice", None, &voice_path, None)
        .await
        .expect("voice file send should succeed without optional fields");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 7);
    assert_multipart_request_without_optional_fields(
        &requests[0],
        "sendDocument",
        "document",
        "chat-document",
        "report.txt",
        "document file body",
    );
    assert_multipart_request_without_optional_fields(
        &requests[1],
        "sendDocument",
        "document",
        "chat-document-bytes",
        "document-bytes.txt",
        "document bytes body",
    );
    assert_multipart_request_without_optional_fields(
        &requests[2],
        "sendPhoto",
        "photo",
        "chat-photo",
        "photo.jpg",
        "photo file body",
    );
    assert_multipart_request_without_optional_fields(
        &requests[3],
        "sendPhoto",
        "photo",
        "chat-photo-bytes",
        "photo-bytes.jpg",
        "photo bytes body",
    );
    assert_multipart_request_without_optional_fields(
        &requests[4],
        "sendVideo",
        "video",
        "chat-video",
        "video.mp4",
        "video file body",
    );
    assert_multipart_request_without_optional_fields(
        &requests[5],
        "sendAudio",
        "audio",
        "chat-audio",
        "audio.mp3",
        "audio file body",
    );
    assert_multipart_request_without_optional_fields(
        &requests[6],
        "sendVoice",
        "voice",
        "chat-voice",
        "voice.ogg",
        "voice file body",
    );
}

#[cfg(all(unix, not(target_os = "macos")))]
#[tokio::test]
async fn multipart_file_methods_use_default_names_for_non_utf8_paths() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    fn write_non_utf8_temp_file(
        dir: &TempDir,
        name_prefix: &[u8],
        contents: &str,
    ) -> std::path::PathBuf {
        let mut name = name_prefix.to_vec();
        name.push(0xff);
        let path = dir.path().join(OsString::from_vec(name));
        std::fs::write(&path, contents).expect("temporary test file should be written");
        path
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let document_path = write_non_utf8_temp_file(&dir, b"document-", "document file body");
    let photo_path = write_non_utf8_temp_file(&dir, b"photo-", "photo file body");
    let video_path = write_non_utf8_temp_file(&dir, b"video-", "video file body");
    let audio_path = write_non_utf8_temp_file(&dir, b"audio-", "audio file body");
    let voice_path = write_non_utf8_temp_file(&dir, b"voice-", "voice file body");

    let server = TestServer::spawn(vec![
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
        ResponseSpec::ok(),
    ])
    .await;
    let channel = test_channel(&server.base_url);

    channel
        .send_document("chat-document", None, &document_path, None)
        .await
        .expect("document file send should succeed");
    channel
        .send_photo("chat-photo", None, &photo_path, None)
        .await
        .expect("photo file send should succeed");
    channel
        .send_video("chat-video", None, &video_path, None)
        .await
        .expect("video file send should succeed");
    channel
        .send_audio("chat-audio", None, &audio_path, None)
        .await
        .expect("audio file send should succeed");
    channel
        .send_voice("chat-voice", None, &voice_path, None)
        .await
        .expect("voice file send should succeed");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 5);
    assert_multipart_request_without_optional_fields(
        &requests[0],
        "sendDocument",
        "document",
        "chat-document",
        "file",
        "document file body",
    );
    assert_multipart_request_without_optional_fields(
        &requests[1],
        "sendPhoto",
        "photo",
        "chat-photo",
        "photo.jpg",
        "photo file body",
    );
    assert_multipart_request_without_optional_fields(
        &requests[2],
        "sendVideo",
        "video",
        "chat-video",
        "video.mp4",
        "video file body",
    );
    assert_multipart_request_without_optional_fields(
        &requests[3],
        "sendAudio",
        "audio",
        "chat-audio",
        "audio.mp3",
        "audio file body",
    );
    assert_multipart_request_without_optional_fields(
        &requests[4],
        "sendVoice",
        "voice",
        "chat-voice",
        "voice.ogg",
        "voice file body",
    );
}

#[tokio::test]
async fn multipart_sending_methods_redact_tokens_in_error_messages() {
    let dir = tempfile::tempdir().expect("tempdir");
    let document_path = write_temp_file(&dir, "report.txt", "document file body");
    let photo_path = write_temp_file(&dir, "photo.jpg", "photo file body");
    let video_path = write_temp_file(&dir, "video.mp4", "video file body");
    let audio_path = write_temp_file(&dir, "audio.mp3", "audio file body");
    let voice_path = write_temp_file(&dir, "voice.ogg", "voice file body");

    let server = TestServer::spawn(vec![
        ResponseSpec::telegram_failure(),
        ResponseSpec::telegram_failure(),
        ResponseSpec::telegram_failure(),
        ResponseSpec::telegram_failure(),
        ResponseSpec::telegram_failure(),
        ResponseSpec::telegram_failure(),
        ResponseSpec::telegram_failure(),
    ])
    .await;
    let channel = test_channel(&server.base_url);

    let document_error = channel
        .send_document("chat", Some("thread"), &document_path, Some("caption"))
        .await
        .expect_err("document file send should fail");
    assert_sanitized_error(document_error, "Telegram sendDocument failed");

    let document_bytes_error = channel
        .send_document_bytes(
            "chat",
            Some("thread"),
            b"document bytes".to_vec(),
            "doc.txt",
            Some("caption"),
        )
        .await
        .expect_err("document bytes send should fail");
    assert_sanitized_error(document_bytes_error, "Telegram sendDocument failed");

    let photo_error = channel
        .send_photo("chat", Some("thread"), &photo_path, Some("caption"))
        .await
        .expect_err("photo file send should fail");
    assert_sanitized_error(photo_error, "Telegram sendPhoto failed");

    let photo_bytes_error = channel
        .send_photo_bytes(
            "chat",
            Some("thread"),
            b"photo bytes".to_vec(),
            "photo.jpg",
            Some("caption"),
        )
        .await
        .expect_err("photo bytes send should fail");
    assert_sanitized_error(photo_bytes_error, "Telegram sendPhoto failed");

    let video_error = channel
        .send_video("chat", Some("thread"), &video_path, Some("caption"))
        .await
        .expect_err("video file send should fail");
    assert_sanitized_error(video_error, "Telegram sendVideo failed");

    let audio_error = channel
        .send_audio("chat", Some("thread"), &audio_path, Some("caption"))
        .await
        .expect_err("audio file send should fail");
    assert_sanitized_error(audio_error, "Telegram sendAudio failed");

    let voice_error = channel
        .send_voice("chat", Some("thread"), &voice_path, Some("caption"))
        .await
        .expect_err("voice file send should fail");
    assert_sanitized_error(voice_error, "Telegram sendVoice failed");
}

#[tokio::test]
async fn url_methods_include_only_thread_id_when_caption_absent() {
    let server =
        TestServer::spawn(vec![ResponseSpec::ok(), ResponseSpec::ok(), ResponseSpec::ok()]).await;
    let channel = test_channel(&server.base_url);

    // Inline document URL path: only thread_id present.
    channel
        .send_document_by_url(
            "chat-document",
            Some("thread-document"),
            "https://example.test/document.pdf",
            None,
        )
        .await
        .expect("document URL send should succeed with only thread id");
    // Inline photo URL path: only thread_id present.
    channel
        .send_photo_by_url(
            "chat-photo",
            Some("thread-photo"),
            "https://example.test/photo.jpg",
            None,
        )
        .await
        .expect("photo URL send should succeed with only thread id");
    // Shared helper path (send_media_by_url): only thread_id present.
    channel
        .send_video_by_url(
            "chat-video",
            Some("thread-video"),
            "https://example.test/video.mp4",
            None,
        )
        .await
        .expect("video URL send should succeed with only thread id");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 3);
    for (request, media_field, chat_id, thread_id, url) in [
        (
            &requests[0],
            "document",
            "chat-document",
            "thread-document",
            "https://example.test/document.pdf",
        ),
        (&requests[1], "photo", "chat-photo", "thread-photo", "https://example.test/photo.jpg"),
        (&requests[2], "video", "chat-video", "thread-video", "https://example.test/video.mp4"),
    ] {
        let body = request.json();
        assert_eq!(body["chat_id"], chat_id);
        assert_eq!(body[media_field], url);
        assert_eq!(body["message_thread_id"], thread_id);
        assert!(body.get("caption").is_none(), "caption should be absent: {}", request.body_text());
    }
}

#[tokio::test]
async fn url_methods_include_only_caption_when_thread_id_absent() {
    let server =
        TestServer::spawn(vec![ResponseSpec::ok(), ResponseSpec::ok(), ResponseSpec::ok()]).await;
    let channel = test_channel(&server.base_url);

    // Inline document URL path: only caption present.
    channel
        .send_document_by_url(
            "chat-document",
            None,
            "https://example.test/document.pdf",
            Some("document caption"),
        )
        .await
        .expect("document URL send should succeed with only caption");
    // Inline photo URL path: only caption present.
    channel
        .send_photo_by_url(
            "chat-photo",
            None,
            "https://example.test/photo.jpg",
            Some("photo caption"),
        )
        .await
        .expect("photo URL send should succeed with only caption");
    // Shared helper path (send_media_by_url): only caption present.
    channel
        .send_audio_by_url(
            "chat-audio",
            None,
            "https://example.test/audio.mp3",
            Some("audio caption"),
        )
        .await
        .expect("audio URL send should succeed with only caption");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 3);
    for (request, media_field, chat_id, url, caption) in [
        (
            &requests[0],
            "document",
            "chat-document",
            "https://example.test/document.pdf",
            "document caption",
        ),
        (&requests[1], "photo", "chat-photo", "https://example.test/photo.jpg", "photo caption"),
        (&requests[2], "audio", "chat-audio", "https://example.test/audio.mp3", "audio caption"),
    ] {
        let body = request.json();
        assert_eq!(body["chat_id"], chat_id);
        assert_eq!(body[media_field], url);
        assert!(
            body.get("message_thread_id").is_none(),
            "thread id should be absent: {}",
            request.body_text()
        );
        assert_eq!(body["caption"], caption);
    }
}

#[tokio::test]
async fn multipart_methods_include_only_thread_id_when_caption_absent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let document_path = write_temp_file(&dir, "report.txt", "document file body");

    let server = TestServer::spawn(vec![ResponseSpec::ok(), ResponseSpec::ok()]).await;
    let channel = test_channel(&server.base_url);

    // File-based multipart: only thread_id present.
    channel
        .send_document("chat-document", Some("thread-document"), &document_path, None)
        .await
        .expect("document file send should succeed with only thread id");
    // Bytes-based multipart: only thread_id present.
    channel
        .send_document_bytes(
            "chat-document-bytes",
            Some("thread-document-bytes"),
            b"document bytes body".to_vec(),
            "document-bytes.txt",
            None,
        )
        .await
        .expect("document bytes send should succeed with only thread id");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 2);
    for request in &requests {
        let body = request.body_text();
        assert!(body.contains(r#"name="message_thread_id""#), "thread id missing: {body}");
        assert!(!body.contains(r#"name="caption""#), "caption should be absent: {body}");
    }
}

#[tokio::test]
async fn multipart_methods_include_only_caption_when_thread_id_absent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let document_path = write_temp_file(&dir, "report.txt", "document file body");

    let server = TestServer::spawn(vec![ResponseSpec::ok(), ResponseSpec::ok()]).await;
    let channel = test_channel(&server.base_url);

    // File-based multipart: only caption present.
    channel
        .send_document("chat-document", None, &document_path, Some("document caption"))
        .await
        .expect("document file send should succeed with only caption");
    // Bytes-based multipart: only caption present.
    channel
        .send_document_bytes(
            "chat-document-bytes",
            None,
            b"document bytes body".to_vec(),
            "document-bytes.txt",
            Some("document bytes caption"),
        )
        .await
        .expect("document bytes send should succeed with only caption");

    let requests = server.requests().await;
    assert_eq!(requests.len(), 2);
    for request in &requests {
        let body = request.body_text();
        assert!(
            !body.contains(r#"name="message_thread_id""#),
            "thread id should be absent: {body}"
        );
        assert!(body.contains(r#"name="caption""#), "caption missing: {body}");
    }
}

#[tokio::test]
async fn multipart_file_methods_return_io_errors_for_missing_files() {
    let channel = TelegramChannel::new("123:SECRET".to_string(), vec!["*".to_string()], false);
    let missing_path = std::path::Path::new("/definitely/missing/telegram-media.file");

    let document_error = channel
        .send_document("chat", Some("thread"), missing_path, Some("caption"))
        .await
        .expect_err("missing document file should fail");
    let photo_error = channel
        .send_photo("chat", Some("thread"), missing_path, Some("caption"))
        .await
        .expect_err("missing photo file should fail");
    let video_error = channel
        .send_video("chat", Some("thread"), missing_path, Some("caption"))
        .await
        .expect_err("missing video file should fail");
    let audio_error = channel
        .send_audio("chat", Some("thread"), missing_path, Some("caption"))
        .await
        .expect_err("missing audio file should fail");
    let voice_error = channel
        .send_voice("chat", Some("thread"), missing_path, Some("caption"))
        .await
        .expect_err("missing voice file should fail");

    for error in [document_error, photo_error, video_error, audio_error, voice_error] {
        let message = error.to_string();
        assert!(
            message.contains("No such file")
                || message.contains("not found")
                || message.contains("os error"),
            "unexpected IO error: {message}"
        );
    }
}
