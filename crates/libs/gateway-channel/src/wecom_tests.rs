use super::*;

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn fixture(
    content_type: &'static str,
    bodies: Vec<&'static str>,
) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind WeCom fixture");
    let address = listener.local_addr().expect("WeCom fixture address");
    let handle = tokio::spawn(async move {
        let mut requests = Vec::new();
        for body in bodies {
            let (mut stream, _) = listener.accept().await.expect("accept WeCom request");
            let mut buffer = [0_u8; 2048];
            let read = stream.read(&mut buffer).await.expect("read WeCom request");
            requests.push(String::from_utf8_lossy(&buffer[..read]).into_owned());
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.expect("write WeCom response");
        }
        requests
    });
    (format!("http://{address}"), handle)
}

#[tokio::test]
async fn wecom_qr_fetch_extracts_only_required_settings() {
    let html = r#"<script>window.settings = {"scode":"scan-code","auth_url":"https://work.weixin.qq.com/ai/qc/auth"};</script>"#;
    let (base_url, requests) = fixture("text/html", vec![html]).await;

    let result = fetch_at(&base_url).await.expect("fetch succeeds");
    let requests = requests.await.expect("WeCom fixture completes");

    assert_eq!(result.poll_state, "scan-code");
    assert_eq!(result.scan_url, "https://work.weixin.qq.com/ai/qc/auth");
    assert!(requests[0].starts_with("GET /ai/qc/gen?source=vibewindow&state="));
    assert!(requests[0].contains("&timestamp="));
}

#[tokio::test]
async fn wecom_qr_maps_success_and_legacy_confirmed_bot_credentials() {
    for status in ["success", "confirmed"] {
        let payload = format!(
            r#"{{"data":{{"status":"{status}","bot_info":{{"botid":"bot-1","secret":"secret-1"}}}}}}"#
        );
        let payload: &'static str = Box::leak(payload.into_boxed_str());
        let (base_url, requests) = fixture("application/json", vec![payload]).await;

        let result = poll_at(&base_url, "scan code").await.expect("poll succeeds");
        let requests = requests.await.expect("WeCom fixture completes");

        assert_eq!(result.status, ChannelQrStatus::Authorized);
        assert_eq!(
            result.credentials,
            Some(json!({"bot_id": "bot-1", "secret": "secret-1"}).as_object().unwrap().clone())
        );
        assert!(requests[0].starts_with("GET /ai/qc/query_result?scode=scan+code HTTP/1.1"));
    }
}

#[tokio::test]
async fn wecom_qr_maps_waiting_scanned_expired_and_error() {
    for (payload, expected) in [
        (r#"{"data":{"status":"waiting"}}"#, ChannelQrStatus::Pending),
        (r#"{"data":{"status":"scanned"}}"#, ChannelQrStatus::Scanned),
        (r#"{"data":{"status":"expired"}}"#, ChannelQrStatus::Expired),
        (r#"{"data":{"status":"error"}}"#, ChannelQrStatus::Pending),
    ] {
        let (base_url, requests) = fixture("application/json", vec![payload]).await;
        let result = poll_at(&base_url, "scan-code").await.expect("poll succeeds");
        requests.await.expect("WeCom fixture completes");
        assert_eq!(result.status, expected);
        assert_eq!(result.credentials, None);
    }
}

#[tokio::test]
async fn wecom_qr_rejects_missing_fetch_and_confirmed_fields() {
    let (base_url, requests) = fixture("text/html", vec!["<html></html>"]).await;
    let error = fetch_at(&base_url).await.expect_err("missing settings rejected");
    requests.await.expect("WeCom fixture completes");
    assert_eq!(error.to_string(), "WeCom authorization page settings were invalid");

    let payload = r#"{"data":{"status":"success","bot_info":{"botid":"bot-1"}}}"#;
    let (base_url, requests) = fixture("application/json", vec![payload]).await;
    let result = poll_at(&base_url, "scan-code").await.expect("poll succeeds");
    requests.await.expect("WeCom fixture completes");
    assert_eq!(result.status, ChannelQrStatus::Failed);
    assert_eq!(result.credentials, None);
}
