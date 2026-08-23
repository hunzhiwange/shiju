use super::attachments::{
    extension_from_media_path, is_image_attachment, normalize_content_type, process_attachments,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn content_type_and_extension_helpers_normalize_media_values() {
    assert_eq!(normalize_content_type("Image/PNG; charset=utf-8"), "image/png");
    assert_eq!(
        extension_from_media_path("https://cdn.test/photo.JPEG?x=1"),
        Some("jpeg".to_string())
    );
}

#[test]
fn image_detection_trusts_explicit_content_type_before_extension() {
    assert!(is_image_attachment("image/png", "file.bin", "https://cdn.test/file.bin"));
    assert!(!is_image_attachment("text/plain", "image.png", "https://cdn.test/image.png"));
    assert!(is_image_attachment("application/octet-stream", "file.bin", "https://cdn.test/a.svg"));
}

async fn serve_once(status: &str, body: &str) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0_u8; 8192];
        let _ = socket.read(&mut request).await;
        socket.write_all(response.as_bytes()).await.unwrap();
    });
    format!("http://{addr}/attachment")
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
    ];

    let output = process_attachments(&attachments, &reqwest::Client::new()).await;

    assert_eq!(
        output,
        "[IMAGE:https://cdn.example.test/photo.png]\n---\n[note.txt]\nhello from attachment"
    );
}

#[tokio::test]
async fn process_attachments_skips_failed_text_fetches_and_audio() {
    let text_url = serve_once("404 Not Found", "nope").await;
    let attachments = vec![
        serde_json::json!({
            "content_type": "text/plain",
            "filename": "missing.txt",
            "url": text_url
        }),
        serde_json::json!({
            "content_type": "audio/ogg",
            "filename": "voice.ogg",
            "url": "http://127.0.0.1:9/voice.ogg"
        }),
    ];

    assert!(process_attachments(&attachments, &reqwest::Client::new()).await.is_empty());
}
// 本文件验证 Discord 入站附件的媒体识别、下载与内容内联行为。
