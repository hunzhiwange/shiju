use super::attachments::{
    audio_extension_from_content_type, extension_from_media_path, has_audio_extension,
    infer_audio_filename, is_audio_attachment, is_image_attachment, is_supported_audio_extension,
    normalize_content_type, parse_attachment_duration_secs, process_attachments,
};
use crate::app::agent::config::TranscriptionConfig;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn content_type_and_extension_helpers_normalize_media_values() {
    assert_eq!(normalize_content_type("Audio/MPEG; charset=utf-8"), "audio/mpeg");
    assert_eq!(
        extension_from_media_path("https://cdn.test/voice.OGG?x=1"),
        Some("ogg".to_string())
    );
    assert!(is_supported_audio_extension("mp3"));
    assert!(has_audio_extension("clip.webm#frag"));
    assert_eq!(audio_extension_from_content_type("audio/x-wav"), Some("wav"));
}

#[test]
fn attachment_type_detection_trusts_explicit_content_type_before_extension() {
    assert!(is_image_attachment("image/png", "file.bin", "https://cdn.test/file.bin"));
    assert!(!is_image_attachment("text/plain", "image.png", "https://cdn.test/image.png"));
    assert!(is_audio_attachment("application/octet-stream", "voice.ogg", ""));
    assert!(!is_audio_attachment("text/plain", "voice.ogg", ""));
}

#[test]
fn duration_and_audio_filename_are_inferred_deterministically() {
    assert_eq!(parse_attachment_duration_secs(&serde_json::json!({"duration_secs": 1.2})), Some(2));
    assert_eq!(parse_attachment_duration_secs(&serde_json::json!({"duration_secs": -1})), None);
    assert_eq!(
        infer_audio_filename("", "https://cdn.test/path/audio.mp3?download=1", ""),
        "audio.mp3"
    );
    assert_eq!(infer_audio_filename("raw", "", "audio/ogg"), "audio.ogg");
}

#[test]
fn media_extension_helpers_cover_images_and_fallbacks() {
    assert_eq!(extension_from_media_path("no-extension"), None);
    assert_eq!(extension_from_media_path("photo.JPEG#preview"), Some("jpeg".to_string()));
    assert!(is_image_attachment("", "photo.avif", ""));
    assert!(is_image_attachment("application/octet-stream", "file.bin", "https://cdn.test/a.svg"));
    assert!(!is_supported_audio_extension("txt"));
    assert_eq!(audio_extension_from_content_type("application/json"), None);
    assert_eq!(infer_audio_filename("  voice.webm  ", "", ""), "voice.webm");
    assert_eq!(infer_audio_filename("raw", "https://cdn.test/no-extension", ""), "audio.ogg");
}

#[test]
fn duration_parser_accepts_integer_zero_and_rejects_non_numbers() {
    assert_eq!(parse_attachment_duration_secs(&serde_json::json!({"duration_secs": 0})), Some(0));
    assert_eq!(parse_attachment_duration_secs(&serde_json::json!({"duration_secs": "1"})), None);
    assert_eq!(parse_attachment_duration_secs(&serde_json::json!({})), None);
}

#[test]
fn duration_parser_rejects_values_above_u64_range() {
    assert_eq!(parse_attachment_duration_secs(&serde_json::json!({"duration_secs": 1.0e30})), None);
}

fn http_response(status: &str, content_type: &str, body: &[u8]) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(body);
    response
}

async fn serve_responses(responses: Vec<Vec<u8>>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        for response in responses {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 8192];
            let _ = socket.read(&mut request).await;
            socket.write_all(&response).await.unwrap();
        }
    });

    format!("http://{addr}")
}

async fn serve_once(status: &str, body: &str) -> String {
    let base = serve_responses(vec![http_response(status, "text/plain", body.as_bytes())]).await;
    format!("{base}/attachment")
}

async fn unused_local_url(path: &str) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    format!("http://{addr}/{path}")
}

fn set_transcription_api_key() {
    unsafe { std::env::set_var("GROQ_API_KEY", "test-key") };
}

fn transcription_config(api_url: String) -> TranscriptionConfig {
    TranscriptionConfig {
        enabled: true,
        api_url,
        max_duration_secs: 10,
        ..TranscriptionConfig::default()
    }
}

async fn serve_audio_and_transcription(transcript: &str) -> String {
    serve_responses(vec![
        http_response("200 OK", "audio/ogg", &[1, 2, 3, 4]),
        http_response(
            "200 OK",
            "application/json",
            format!(r#"{{"text":"{transcript}"}}"#).as_bytes(),
        ),
    ])
    .await
}

fn truncated_http_response() -> Vec<u8> {
    b"HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\nabc".to_vec()
}

async fn unused_audio_url() -> String {
    unused_local_url("audio.ogg").await
}

#[tokio::test]
async fn process_attachments_inlines_images_and_text_files() {
    let text_url = serve_once("200 OK", "hello from attachment").await;
    let attachments = vec![
        serde_json::json!({
            "content_type": "image/png",
            "filename": "photo.png",
            "url": "https://cdn.example.test/photo.png"
        }),
        serde_json::json!({
            "content_type": "text/plain; charset=utf-8",
            "filename": "note.txt",
            "url": text_url
        }),
        serde_json::json!({
            "content_type": "image/png",
            "filename": "missing-url.png"
        }),
        serde_json::json!({
            "content_type": "application/pdf",
            "filename": "manual.pdf",
            "url": "https://cdn.example.test/manual.pdf"
        }),
    ];

    let output = process_attachments(&attachments, &reqwest::Client::new(), None).await;

    assert_eq!(
        output,
        "[IMAGE:https://cdn.example.test/photo.png]\n---\n[note.txt]\nhello from attachment"
    );
}

#[tokio::test]
async fn process_attachments_skips_failed_text_fetch_and_long_audio() {
    let text_url = serve_once("404 Not Found", "nope").await;
    let text_error_url = unused_local_url("missing.txt").await;
    let mut transcription = TranscriptionConfig {
        enabled: true,
        max_duration_secs: 1,
        ..TranscriptionConfig::default()
    };
    transcription.api_url = "http://127.0.0.1:9/transcribe".to_string();
    let attachments = vec![
        serde_json::json!({
            "content_type": "text/plain",
            "filename": "missing.txt",
            "url": text_url
        }),
        serde_json::json!({
            "content_type": "text/plain",
            "filename": "error.txt",
            "url": text_error_url
        }),
        serde_json::json!({
            "content_type": "audio/ogg",
            "filename": "too-long.ogg",
            "url": "http://127.0.0.1:9/audio.ogg",
            "duration_secs": 2
        }),
        serde_json::json!({
            "content_type": "audio/ogg",
            "filename": "disabled.ogg",
            "url": "http://127.0.0.1:9/disabled.ogg"
        }),
    ];

    let output =
        process_attachments(&attachments[..3], &reqwest::Client::new(), Some(&transcription)).await;
    assert!(output.is_empty());

    let output = process_attachments(&attachments[3..], &reqwest::Client::new(), None).await;
    assert!(output.is_empty());
}

#[tokio::test]
async fn process_attachments_transcribes_audio_and_uses_url_extension() {
    set_transcription_api_key();
    let base = serve_audio_and_transcription("  hello from audio  ").await;
    let transcription = transcription_config(format!("{base}/transcribe"));
    let attachments = vec![serde_json::json!({
        "content_type": "application/octet-stream",
        "filename": "blob",
        "url": format!("{base}/clip.mp3?download=1"),
        "duration_secs": 1.2
    })];

    let output =
        process_attachments(&attachments, &reqwest::Client::new(), Some(&transcription)).await;

    assert_eq!(output, "[Voice:audio.mp3] hello from audio");
}

#[tokio::test]
async fn process_attachments_skips_empty_transcript_and_audio_fetch_failures() {
    set_transcription_api_key();
    let base = serve_responses(vec![
        http_response("200 OK", "audio/ogg", &[1, 2, 3, 4]),
        http_response("200 OK", "application/json", br#"{"text":"   "}"#),
        http_response("503 Service Unavailable", "text/plain", b"no"),
    ])
    .await;
    let transcription = transcription_config(format!("{base}/transcribe"));
    let attachments = vec![
        serde_json::json!({
            "content_type": "audio/ogg",
            "filename": "empty.ogg",
            "url": format!("{base}/empty.ogg")
        }),
        serde_json::json!({
            "content_type": "audio/ogg",
            "filename": "unavailable.ogg",
            "url": format!("{base}/unavailable.ogg")
        }),
        serde_json::json!({
            "content_type": "audio/ogg",
            "filename": "network.ogg",
            "url": unused_audio_url().await
        }),
    ];

    let output =
        process_attachments(&attachments, &reqwest::Client::new(), Some(&transcription)).await;

    assert!(output.is_empty());
}

#[tokio::test]
async fn process_attachments_skips_audio_body_errors_and_transcription_errors() {
    set_transcription_api_key();
    let base = serve_responses(vec![
        truncated_http_response(),
        http_response("200 OK", "audio/ogg", &[1, 2, 3, 4]),
        http_response(
            "500 Internal Server Error",
            "application/json",
            br#"{"error":{"message":"no transcript"}}"#,
        ),
    ])
    .await;
    let transcription = transcription_config(format!("{base}/transcribe"));
    let attachments = vec![
        serde_json::json!({
            "content_type": "audio/ogg",
            "filename": "bad-body.ogg",
            "url": format!("{base}/bad-body.ogg")
        }),
        serde_json::json!({
            "content_type": "audio/ogg",
            "filename": "api-error.ogg",
            "url": format!("{base}/api-error.ogg")
        }),
    ];

    let output =
        process_attachments(&attachments, &reqwest::Client::new(), Some(&transcription)).await;

    assert!(output.is_empty());
}
