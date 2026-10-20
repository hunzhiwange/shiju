use super::*;

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn fixture(body: &'static str) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind WeChat fixture");
    let address = listener.local_addr().expect("WeChat fixture address");
    let handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept WeChat request");
        let mut buffer = [0_u8; 2048];
        let read = stream.read(&mut buffer).await.expect("read WeChat request");
        let request = String::from_utf8_lossy(&buffer[..read]).into_owned();
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.expect("write WeChat response");
        request
    });
    (format!("http://{address}"), handle)
}

#[tokio::test]
async fn wechat_qr_fetch_maps_qrcode_without_credentials_in_state() {
    let (base_url, request) = fixture(r#"{"qrcode":"qr value","bot_token":"must-not-leak"}"#).await;

    let result = fetch_at(&base_url).await.expect("fetch succeeds");
    let request = request.await.expect("WeChat fixture completes");

    assert_eq!(result.poll_state, "qr value");
    assert!(!result.poll_state.contains("must-not-leak"));
    assert_eq!(
        result.scan_url,
        "https://liteapp.weixin.qq.com/q/7GiQu1?qrcode=qr%20value&bot_type=3"
    );
    assert!(request.starts_with("GET /ilink/bot/get_bot_qrcode?bot_type=3 HTTP/1.1"));
}

#[tokio::test]
async fn wechat_qr_maps_provider_and_legacy_waiting_states() {
    for (payload, expected) in [
        (r#"{"status":"wait"}"#, ChannelQrStatus::Pending),
        (r#"{"status":"waiting"}"#, ChannelQrStatus::Pending),
        (r#"{"status":"scaned"}"#, ChannelQrStatus::Scanned),
        (r#"{"status":"scanned"}"#, ChannelQrStatus::Scanned),
        (r#"{"status":"expired"}"#, ChannelQrStatus::Expired),
        (r#"{"status":"error"}"#, ChannelQrStatus::Failed),
    ] {
        let (base_url, request) = fixture(payload).await;
        let result = poll_at(&base_url, "qr-value").await.expect("poll succeeds");
        let request = request.await.expect("WeChat fixture completes");
        assert!(request.to_ascii_lowercase().contains("ilink-app-clientversion: 1"));
        assert_eq!(result.status, expected);
        assert_eq!(result.credentials, None);
    }
}

#[tokio::test]
async fn wechat_qr_maps_confirmed_credentials() {
    let payload =
        r#"{"status":"confirmed","bot_token":"token-1","baseurl":"https://ilinkai.weixin.qq.com"}"#;
    let (base_url, request) = fixture(payload).await;

    let result = poll_at(&base_url, "qr value").await.expect("poll succeeds");
    let request = request.await.expect("WeChat fixture completes");

    assert_eq!(result.status, ChannelQrStatus::Authorized);
    assert_eq!(
        result.credentials,
        Some(
            json!({"bot_token": "token-1", "base_url": "https://ilinkai.weixin.qq.com"})
                .as_object()
                .unwrap()
                .clone()
        )
    );
    assert!(request.starts_with("GET /ilink/bot/get_qrcode_status?qrcode=qr+value HTTP/1.1"));
}

#[tokio::test]
async fn wechat_qr_rejects_untrusted_base_url() {
    for baseurl in [
        "https://evil.example",
        "http://ilinkai.weixin.qq.com",
        "https://ilinkai.weixin.qq.com.evil.example",
    ] {
        let payload =
            format!(r#"{{"status":"confirmed","bot_token":"token-1","baseurl":"{baseurl}"}}"#);
        let leaked: &'static str = Box::leak(payload.into_boxed_str());
        let (base_url, request) = fixture(leaked).await;
        let result = poll_at(&base_url, "qr-value").await.expect("poll succeeds");
        request.await.expect("WeChat fixture completes");
        assert_eq!(result.status, ChannelQrStatus::Failed);
        assert_eq!(result.credentials, None);
    }
}

#[tokio::test]
async fn wechat_qr_rejects_incomplete_confirmed_and_empty_qrcode() {
    let (base_url, request) = fixture(r#"{"qrcode":""}"#).await;
    let error = fetch_at(&base_url).await.expect_err("empty qrcode rejected");
    request.await.expect("WeChat fixture completes");
    assert_eq!(error.to_string(), "WeChat returned empty QR code data");

    let payload =
        r#"{"status":"confirmed","bot_token":"","baseurl":"https://ilinkai.weixin.qq.com"}"#;
    let (base_url, request) = fixture(payload).await;
    let result = poll_at(&base_url, "qr-value").await.expect("poll succeeds");
    request.await.expect("WeChat fixture completes");
    assert_eq!(result.status, ChannelQrStatus::Failed);
    assert_eq!(result.credentials, None);
}
