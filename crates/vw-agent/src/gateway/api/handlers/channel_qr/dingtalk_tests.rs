use super::*;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn json_server(bodies: Vec<&'static str>) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind DingTalk fixture");
    let address = listener.local_addr().expect("DingTalk fixture address");
    let handle = tokio::spawn(async move {
        let mut requests = Vec::new();
        for body in bodies {
            let (mut stream, _) = listener.accept().await.expect("accept DingTalk request");
            requests.push(read_request(&mut stream).await);
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.expect("write DingTalk response");
        }
        requests
    });
    (format!("http://{address}"), handle)
}

async fn read_request(stream: &mut tokio::net::TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    let mut expected_length = None;
    loop {
        let read = stream.read(&mut buffer).await.expect("read DingTalk request");
        request.extend_from_slice(&buffer[..read]);
        let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") else {
            continue;
        };
        let body_start = header_end + 4;
        let content_length = *expected_length.get_or_insert_with(|| {
            String::from_utf8_lossy(&request[..header_end])
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0)
        });
        if request.len() >= body_start + content_length {
            break;
        }
    }
    String::from_utf8(request).expect("DingTalk request UTF-8")
}

fn body(request: &str) -> Value {
    serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap_or_default())
        .expect("DingTalk request JSON")
}

#[tokio::test]
async fn dingtalk_qr_fetch_posts_init_then_begin() {
    let (base_url, requests) = json_server(vec![
        r#"{"errcode":0,"nonce":"nonce-1"}"#,
        r#"{"errcode":0,"device_code":"device-1","verification_uri_complete":"https://example.com/scan"}"#,
    ])
    .await;

    let result = fetch_at(&base_url).await.expect("fetch succeeds");
    let requests = requests.await.expect("DingTalk fixture completes");

    assert_eq!(result.poll_state, "device-1");
    assert_eq!(result.scan_url, "https://example.com/scan");
    assert!(requests[0].starts_with("POST /app/registration/init HTTP/1.1"));
    assert_eq!(body(&requests[0]), json!({"source": "QWENPAW"}));
    assert!(requests[1].starts_with("POST /app/registration/begin HTTP/1.1"));
    assert_eq!(body(&requests[1]), json!({"nonce": "nonce-1"}));
}

#[tokio::test]
async fn dingtalk_qr_accepts_issued_credentials_during_approving() {
    let (base_url, requests) = json_server(vec![
        r#"{"errcode":0,"status":"APPROVING","client_id":"app-id","client_secret":"app-secret"}"#,
    ])
    .await;

    let result = poll_at(&base_url, "device-code").await.expect("poll succeeds");
    let requests = requests.await.expect("DingTalk fixture completes");

    assert_eq!(result.status, ChannelQrStatus::Authorized);
    assert_eq!(
        result.credentials,
        Some(
            json!({"client_id": "app-id", "client_secret": "app-secret"})
                .as_object()
                .unwrap()
                .clone()
        )
    );
    assert!(requests[0].starts_with("POST /app/registration/poll HTTP/1.1"));
    assert_eq!(body(&requests[0]), json!({"device_code": "device-code"}));
}

#[tokio::test]
async fn dingtalk_qr_distinguishes_pending_failed_and_expired() {
    for (payload, expected) in [
        (r#"{"errcode":0,"status":"WAITING"}"#, ChannelQrStatus::Pending),
        (r#"{"errcode":0,"status":"unexpected"}"#, ChannelQrStatus::Pending),
        (r#"{"errcode":0,"status":"FAIL"}"#, ChannelQrStatus::Failed),
        (r#"{"errcode":0,"status":"EXPIRED"}"#, ChannelQrStatus::Expired),
    ] {
        let (base_url, requests) = json_server(vec![payload]).await;
        let result = poll_at(&base_url, "device-code").await.expect("poll succeeds");
        requests.await.expect("DingTalk fixture completes");
        assert_eq!(result.status, expected);
        assert_eq!(result.credentials, None);
    }
}

#[tokio::test]
async fn dingtalk_qr_rejects_errcode_and_empty_flow_values() {
    let (base_url, requests) =
        json_server(vec![r#"{"errcode":400,"errmsg":"secret detail"}"#]).await;
    let error = fetch_at(&base_url).await.expect_err("init errcode is rejected");
    requests.await.expect("DingTalk fixture completes");
    assert_eq!(error.to_string(), "DingTalk init failed");

    let (base_url, requests) = json_server(vec![r#"{"errcode":0,"nonce":"  "}"#]).await;
    let error = fetch_at(&base_url).await.expect_err("empty nonce is rejected");
    requests.await.expect("DingTalk fixture completes");
    assert_eq!(error.to_string(), "DingTalk returned empty nonce");

    let (base_url, requests) = json_server(vec![
        r#"{"errcode":0,"nonce":"nonce-1"}"#,
        r#"{"errcode":0,"device_code":"","verification_uri_complete":"https://example.com/scan"}"#,
    ])
    .await;
    let error = fetch_at(&base_url).await.expect_err("empty device code is rejected");
    requests.await.expect("DingTalk fixture completes");
    assert_eq!(error.to_string(), "DingTalk returned empty device_code or verification URI");
}
