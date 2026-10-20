// 本文件验证 QQ 出站消息请求、附件发送、鉴权重试与错误脱敏。

use super::*;

use crate::app::agent::channels::traits::SendMessage;
use rustls::ServerConfig;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

const TEST_CERT_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIIDCTCCAfGgAwIBAgIUOBV6sEAKMDTS2zjDowZ7av4OXcUwDQYJKoZIhvcNAQEL
BQAwFDESMBAGA1UEAwwJbG9jYWxob3N0MB4XDTI2MDYyOTA5MDQyMVoXDTI2MDYz
MDA5MDQyMVowFDESMBAGA1UEAwwJbG9jYWxob3N0MIIBIjANBgkqhkiG9w0BAQEF
AAOCAQ8AMIIBCgKCAQEAwEdCISG9WGL0bVyyoR8v6oEa/mxn6kWKceg4dFb4L+go
h7eYYNrduvnNIFrpSeseknk2jKe8bmDx3bgV+7n1qNZgvh4ex1VQyLzgE+enmYjv
CK2r8GBuBcgZeEco2otASYMVWNtrdFEM2sfIJvYnwAeGTKonBZ5ThttFqeeISk61
25OhER4CExCVqR2i9pFMWq3qO5wMR3A40Jk26bpgD5DO97YnrAO2f4lRyNeSn6rh
zvjXuFUIrBhxNp9nCC3CcBviNLjhrqSkmDe6mUWP30LzIs4XfTWhIA3q+GzwHHYk
02M5kNhixHnkeqhJiLVNAD4SpwkH17pM3EAI+DBebwIDAQABo1MwUTAdBgNVHQ4E
FgQUA4wDD24rtN2Xy7uu9Tu6sx7DJh4wHwYDVR0jBBgwFoAUA4wDD24rtN2Xy7uu
9Tu6sx7DJh4wDwYDVR0TAQH/BAUwAwEB/zANBgkqhkiG9w0BAQsFAAOCAQEArsn6
3r7xp58yUm3uNk0bHlUASi9OXoWA9cjmHOZ1xaJEHIHYwOHVnK+YeQfJNLjahLUW
nh4oyWqvLU2jZoEr9J6HlyyeFzb36nJGhK4sXFsFlM7J/qS8crPhm7E4UySEeNB6
1a2H7/wmc2rOJLyBwj7dcBuJHuwdfnHkiIG3QUohXPWyyuhe21zgAm1PDeSBZ5XS
xeXTAuJlMOi5NZFdjG4tTqKfsruMVpKyhEUaJR2vPelqAocLuFoNM0A6RM9s9lIm
IqyMztOtzNyEnRQ/9/R+rhZmgL27kQntKYvf/GZe3qG6Sl5+zehEeSJE1f7fc39G
gcpBydVs/apVUtrQXw==
-----END CERTIFICATE-----
"#;

const TEST_KEY_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQDAR0IhIb1YYvRt
XLKhHy/qgRr+bGfqRYpx6Dh0Vvgv6CiHt5hg2t26+c0gWulJ6x6SeTaMp7xuYPHd
uBX7ufWo1mC+Hh7HVVDIvOAT56eZiO8IravwYG4FyBl4Ryjai0BJgxVY22t0UQza
x8gm9ifAB4ZMqicFnlOG20Wp54hKTrXbk6ERHgITEJWpHaL2kUxareo7nAxHcDjQ
mTbpumAPkM73tiesA7Z/iVHI15KfquHO+Ne4VQisGHE2n2cILcJwG+I0uOGupKSY
N7qZRY/fQvMizhd9NaEgDer4bPAcdiTTYzmQ2GLEeeR6qEmItU0APhKnCQfXukzc
QAj4MF5vAgMBAAECggEAA7DaehxVsB6mJntDCoi4uL2lsQCPthWDpNAaVVFeDj1c
mc5SZHmPDAnDgD1+gOa8fX9KJsUJfuzpd6IS+fQF3SnQTZXybU0b8d5+RqOJug7A
19/Njx2qvv2qT36+MAPARij3/e6oT2FG5+Qx2V2cQbljQJF7kkYJwGrxGWPSX+q+
TSS6HT48h12EgWB9Nb/IOoMWKMrSxSLVnFg0DSAYeLOGzGcXvg0okM6aZ17bFL5F
MwtJMUMywrZQFosxkDg2xRkKd2W0dJoBFmIPWSxVrDAjZqMUXAnSaaNZ2vGJcGxO
PGUL0Eszj+ymQAAwKfjcNJlTP1UZZ51zw53uTOEfYQKBgQD3u3VfY4VX3ru5lBgy
vtxQb7Sp/aF+w5jKY3yrYIEblygHGT7weBld8tbWHcKboE6JdLzpui7YCHtthXVV
M50KevCIIoLY0Yqh+u33SSClHKBX7gxNHAkoaM7y7+sm350QkI+HERYcOFObTRRG
J7AYCrhYWNWMi9nmUrIFNh04FwKBgQDGsgUshTuCZmHo7Ia+T+y/NzibBtWa9ECw
7vsXRaHygIyyuDrzaKQ3lwqN/QniUoUOazqpGVaeI5kwSYbXv9qXIZIA2ZqXqBlC
bxBuaDiwC+7MS3Nx1YmU6IpduNwPp/CM/wE9AwMEYpfNUbYq0y12mGp9h4RS28dL
DROGpUuraQKBgQD0vBSwL7lDnZ8HLu7Y8wTj0fdBGHe6q7D+AFrhU204vJOmr6Im
Hf+xeH4U+UGQTUZJNTKMFIrCkV6eTAImMd8qw1lso+twF1JFw4fjjDLb6vPc6ibs
5o2FhkUsayh8Nt0LyESCwrZ9Gog+HF+zDvJAThbf8KLHPkH98CQ+Ub978QKBgQCJ
0E/mYUK1X23psKSEu8rpldOdM1BU+bUlA50Etzdsi90K2N2KlrnQFOMtOKTyZWoE
Zjt5ril15hsKcvUkD939lTs/e67OIbnjakVrTr6C3MP0zyBzhDi5H9FY0C9SbJlc
MkMZnk0pbXj84AAKFA8kgvLZvnT5Q5cnZxpz+ZJSgQKBgQCUflgj3ADvLZxPNTFA
aKS+6aJqHgBaoqoEHgrzA/GZos3/RSmXjqgPNQaOXObK2c1pCymBeGLxui0Z9YiY
asnfH7XV9wRCDoByrWAMEdb24lR7MNRIv3ZynE2DLJ7hRnvf7aok7DB01XuAX/jT
zPhTg0YXzGKzjyipf9D4WaRuJQ==
-----END PRIVATE KEY-----
"#;

#[derive(Debug)]
struct CapturedRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: String,
}

#[derive(Clone, Debug)]
struct MockResponse {
    path: &'static str,
    status: u16,
    content_type: &'static str,
    body: &'static str,
}

fn install_rustls_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        _ => "Error",
    }
}

fn content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            (name.trim().eq_ignore_ascii_case("content-length")).then(|| value.trim())
        })
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0)
}

async fn read_request<S>(stream: &mut S) -> CapturedRequest
where
    S: AsyncReadExt + Unpin,
{
    let mut buffer = Vec::new();
    let mut header_end = None;
    let mut body_len = 0usize;

    loop {
        let mut chunk = [0u8; 2048];
        let read = stream.read(&mut chunk).await.expect("request bytes");
        assert!(read > 0, "connection closed before request completed");
        buffer.extend_from_slice(&chunk[..read]);

        if header_end.is_none()
            && let Some(position) = buffer.windows(4).position(|window| window == b"\r\n\r\n")
        {
            let headers = String::from_utf8(buffer[..position].to_vec()).expect("headers utf8");
            body_len = content_length(&headers);
            header_end = Some(position + 4);
        }

        if let Some(end) = header_end
            && buffer.len() >= end + body_len
        {
            let headers = String::from_utf8(buffer[..end - 4].to_vec()).expect("headers utf8");
            let mut lines = headers.lines();
            let request_line = lines.next().expect("request line");
            let mut parts = request_line.split_whitespace();
            let method = parts.next().expect("method").to_string();
            let path = parts.next().expect("path").to_string();
            let headers = lines
                .filter_map(|line| line.split_once(':'))
                .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
                .collect();
            let body = String::from_utf8(buffer[end..end + body_len].to_vec()).expect("body utf8");

            return CapturedRequest { method, path, headers, body };
        }
    }
}

async fn spawn_https_server(
    responses: Vec<MockResponse>,
) -> (String, tokio::task::JoinHandle<Vec<CapturedRequest>>) {
    install_rustls_provider();
    let certificate = CertificateDer::from_pem_slice(TEST_CERT_PEM.as_bytes()).expect("test cert");
    let private_key = PrivateKeyDer::from_pem_slice(TEST_KEY_PEM.as_bytes()).expect("test key");
    let tls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![certificate], private_key)
        .expect("tls config");
    let acceptor = TlsAcceptor::from(Arc::new(tls_config));

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind listener");
    let port = listener.local_addr().expect("listener addr").port();
    let handle = tokio::spawn(async move {
        let mut requests = Vec::new();
        for response in responses {
            let (stream, _) = listener.accept().await.expect("accept connection");
            let mut stream = acceptor.accept(stream).await.expect("tls accept");
            let request = read_request(&mut stream).await;
            assert_eq!(request.path, response.path);

            let reply = format!(
                "HTTP/1.1 {} {}\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                response.status,
                status_text(response.status),
                response.content_type,
                response.body.len(),
                response.body
            );
            stream.write_all(reply.as_bytes()).await.expect("write response");
            stream.shutdown().await.expect("shutdown stream");
            requests.push(request);
        }
        requests
    });

    (format!("https://localhost:{port}"), handle)
}

fn test_client() -> reqwest::Client {
    reqwest::Client::builder().danger_accept_invalid_certs(true).build().expect("test client")
}

fn parse_json(body: &str) -> Value {
    serde_json::from_str(body).expect("json body")
}

#[test]
fn resolve_send_endpoints_sanitizes_direct_user_recipient() {
    let (messages, files) = resolve_send_endpoints("user:abc-123!");

    assert!(messages.ends_with("/v2/users/abc123/messages"));
    assert!(files.ends_with("/v2/users/abc123/files"));
}

#[test]
fn resolve_send_endpoints_preserves_group_recipient() {
    let (messages, files) = resolve_send_endpoints("group:g-1");

    assert!(messages.ends_with("/v2/groups/g-1/messages"));
    assert!(files.ends_with("/v2/groups/g-1/files"));
}

#[test]
fn resolve_send_endpoints_treats_plain_recipient_as_user_and_keeps_underscores() {
    let (messages, files) = resolve_send_endpoints("abc_DEF-123!");

    assert!(messages.ends_with("/v2/users/abc_DEF123/messages"));
    assert!(files.ends_with("/v2/users/abc_DEF123/files"));
}

#[test]
fn resolve_send_endpoints_allows_empty_sanitized_user_id() {
    let (messages, files) = resolve_send_endpoints("user:!!!");

    assert!(messages.ends_with("/v2/users//messages"));
    assert!(files.ends_with("/v2/users//files"));
}

#[tokio::test]
async fn channel_post_json_reuses_https_validation() {
    let channel = QQChannel::new("app".into(), "secret".into(), vec![]);

    let error = channel
        .post_json("token", "http://127.0.0.1:9/messages", &json!({"ok": true}), "send message")
        .await
        .unwrap_err();

    assert!(error.to_string().contains("URL scheme must be https"));
}

#[tokio::test]
async fn post_json_with_client_posts_json_and_auth_header() {
    let (base_url, handle) = spawn_https_server(vec![MockResponse {
        path: "/messages",
        status: 200,
        content_type: "application/json",
        body: "{}",
    }])
    .await;
    let client = test_client();
    let body = json!({"content": "hello"});

    post_json_with_client(&client, "test-token", &format!("{base_url}/messages"), &body, "send")
        .await
        .expect("post should succeed");

    let requests = handle.await.expect("server join");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "POST");
    assert_eq!(
        requests[0].headers.get("authorization").map(String::as_str),
        Some("QQBot test-token")
    );
    assert_eq!(parse_json(&requests[0].body), body);
}

#[tokio::test]
async fn post_json_with_client_returns_sanitized_error_on_http_failure() {
    let (base_url, handle) = spawn_https_server(vec![MockResponse {
        path: "/messages",
        status: 500,
        content_type: "text/plain",
        body: "upstream failed",
    }])
    .await;
    let client = test_client();

    let error = post_json_with_client(
        &client,
        "test-token",
        &format!("{base_url}/messages"),
        &json!({"content": "hello"}),
        "send message",
    )
    .await
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("QQ send message failed (500 Internal Server Error): upstream failed")
    );
    handle.await.expect("server join");
}

#[tokio::test]
async fn channel_upload_media_file_info_reuses_https_validation() {
    let channel = QQChannel::new("app".into(), "secret".into(), vec![]);

    let error = channel
        .upload_media_file_info("token", "https://localhost/files", "http://127.0.0.1/media.png")
        .await
        .unwrap_err();

    assert!(error.to_string().contains("URL scheme must be https"));
}

#[tokio::test]
async fn upload_media_file_info_with_client_returns_trimmed_file_info() {
    let (base_url, handle) = spawn_https_server(vec![MockResponse {
        path: "/files",
        status: 200,
        content_type: "application/json",
        body: r#"{"file_info":"  file-123  "}"#,
    }])
    .await;
    let client = test_client();

    let file_info = upload_media_file_info_with_client(
        &client,
        "test-token",
        &format!("{base_url}/files"),
        "https://cdn.qq.test/image.png",
    )
    .await
    .expect("upload should succeed");

    let requests = handle.await.expect("server join");
    assert_eq!(file_info, "file-123");
    assert_eq!(requests.len(), 1);
    assert_eq!(
        parse_json(&requests[0].body),
        json!({
            "file_type": 1,
            "url": "https://cdn.qq.test/image.png",
            "srv_send_msg": false
        })
    );
}

#[tokio::test]
async fn upload_media_file_info_with_client_rejects_missing_file_info() {
    let (base_url, handle) = spawn_https_server(vec![MockResponse {
        path: "/files",
        status: 200,
        content_type: "application/json",
        body: "{}",
    }])
    .await;
    let client = test_client();

    let error = upload_media_file_info_with_client(
        &client,
        "test-token",
        &format!("{base_url}/files"),
        "https://cdn.qq.test/image.png",
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("missing file_info"));
    handle.await.expect("server join");
}

#[tokio::test]
async fn upload_media_file_info_with_client_returns_sanitized_error_on_http_failure() {
    let (base_url, handle) = spawn_https_server(vec![MockResponse {
        path: "/files",
        status: 502,
        content_type: "text/plain",
        body: "upload failed",
    }])
    .await;
    let client = test_client();

    let error = upload_media_file_info_with_client(
        &client,
        "test-token",
        &format!("{base_url}/files"),
        "https://cdn.qq.test/image.png",
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("QQ upload media failed (502 Bad Gateway): upload failed"));
    handle.await.expect("server join");
}

#[tokio::test]
// 缓存令牌支撑正常发送，空载荷则在网络请求前短路。
async fn send_message_uses_cached_token_and_skips_empty_payload_without_network() {
    let channel = QQChannel::new("app".into(), "secret".into(), vec![]);
    {
        let mut cache = channel.token_cache.write().await;
        *cache = Some(("cached-token".to_string(), u64::MAX));
    }

    channel
        .send_message(&SendMessage::new("   ", "user:test_user"))
        .await
        .expect("empty payload should be ignored");
}

#[tokio::test]
async fn send_message_with_client_posts_text_and_media_with_incrementing_sequences() {
    let (base_url, handle) = spawn_https_server(vec![
        MockResponse {
            path: "/messages",
            status: 200,
            content_type: "application/json",
            body: "{}",
        },
        MockResponse {
            path: "/files",
            status: 200,
            content_type: "application/json",
            body: r#"{"file_info":"file-a"}"#,
        },
        MockResponse {
            path: "/messages",
            status: 200,
            content_type: "application/json",
            body: "{}",
        },
        MockResponse {
            path: "/files",
            status: 200,
            content_type: "application/json",
            body: r#"{"file_info":"file-b"}"#,
        },
        MockResponse {
            path: "/messages",
            status: 200,
            content_type: "application/json",
            body: "{}",
        },
    ])
    .await;
    let client = test_client();
    let message = SendMessage::new(
        "hello\n[IMAGE:https://cdn.qq.test/a.png]\n[IMAGE:https://cdn.qq.test/b.png]",
        "user:test_user",
    )
    .in_thread(Some("  thread-1  ".to_string()));

    send_message_with_client(
        &client,
        "test-token",
        &format!("{base_url}/messages"),
        &format!("{base_url}/files"),
        &message,
    )
    .await
    .expect("send should succeed");

    let requests = handle.await.expect("server join");
    assert_eq!(requests.len(), 5);
    assert_eq!(
        parse_json(&requests[0].body),
        json!({
            "markdown": {"content": "hello"},
            "msg_type": 2,
            "msg_id": "thread-1",
            "msg_seq": 1
        })
    );
    assert_eq!(
        parse_json(&requests[1].body),
        json!({
            "file_type": 1,
            "url": "https://cdn.qq.test/a.png",
            "srv_send_msg": false
        })
    );
    assert_eq!(
        parse_json(&requests[2].body),
        json!({
            "content": " ",
            "msg_type": 7,
            "media": { "file_info": "file-a" },
            "msg_id": "thread-1",
            "msg_seq": 2
        })
    );
    assert_eq!(
        parse_json(&requests[3].body),
        json!({
            "file_type": 1,
            "url": "https://cdn.qq.test/b.png",
            "srv_send_msg": false
        })
    );
    assert_eq!(
        parse_json(&requests[4].body),
        json!({
            "content": " ",
            "msg_type": 7,
            "media": { "file_info": "file-b" },
            "msg_id": "thread-1",
            "msg_seq": 3
        })
    );
}

#[tokio::test]
async fn send_message_with_client_omits_passive_fields_without_thread_id() {
    let (base_url, handle) = spawn_https_server(vec![
        MockResponse {
            path: "/messages",
            status: 200,
            content_type: "application/json",
            body: "{}",
        },
        MockResponse {
            path: "/files",
            status: 200,
            content_type: "application/json",
            body: r#"{"file_info":"file-a"}"#,
        },
        MockResponse {
            path: "/messages",
            status: 200,
            content_type: "application/json",
            body: "{}",
        },
    ])
    .await;
    let client = test_client();
    let message = SendMessage::new("hello\n[IMAGE:https://cdn.qq.test/a.png]", "user:test_user")
        .in_thread(Some("   ".to_string()));

    send_message_with_client(
        &client,
        "test-token",
        &format!("{base_url}/messages"),
        &format!("{base_url}/files"),
        &message,
    )
    .await
    .expect("send should succeed");

    let requests = handle.await.expect("server join");
    let text_body = parse_json(&requests[0].body);
    let media_body = parse_json(&requests[2].body);
    assert_eq!(text_body["markdown"]["content"], "hello");
    assert_eq!(text_body["msg_type"], 2);
    assert!(text_body.get("msg_id").is_none());
    assert!(text_body.get("msg_seq").is_none());
    assert_eq!(media_body["media"]["file_info"], "file-a");
    assert!(media_body.get("msg_id").is_none());
    assert!(media_body.get("msg_seq").is_none());
}

#[tokio::test]
async fn send_message_with_client_falls_back_after_markdown_validation_error() {
    let (base_url, handle) = spawn_https_server(vec![
        MockResponse {
            path: "/messages",
            status: 400,
            content_type: "application/json",
            body: r#"{"code":40034012,"message":"markdown not allowed"}"#,
        },
        MockResponse {
            path: "/messages",
            status: 200,
            content_type: "application/json",
            body: "{}",
        },
    ])
    .await;
    let client = test_client();
    let message = SendMessage::new("访问 https://example.com", "user:test_user");

    send_message_with_client(
        &client,
        "test-token",
        &format!("{base_url}/messages"),
        &format!("{base_url}/files"),
        &message,
    )
    .await
    .expect("plain-text fallback should succeed");

    let requests = handle.await.expect("server join");
    assert_eq!(parse_json(&requests[0].body)["msg_type"], 2);
    assert_eq!(
        parse_json(&requests[1].body),
        json!({"content": "访问 [链接已省略]", "msg_type": 0})
    );
}
