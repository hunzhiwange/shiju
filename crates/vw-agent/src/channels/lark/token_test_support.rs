//! Lark 令牌相关测试的本地 HTTP 服务、请求捕获与通道夹具。

use super::super::super::LarkChannel;
use super::super::CachedTenantToken;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::time::timeout;

pub(super) struct MockResponse {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
}

impl MockResponse {
    pub(super) fn json(status: u16, body: &str) -> Self {
        Self { status, content_type: "application/json", body: body.as_bytes().to_vec() }
    }

    pub(super) fn text(status: u16, body: &str) -> Self {
        Self { status, content_type: "text/plain", body: body.as_bytes().to_vec() }
    }

    pub(super) fn bytes(status: u16, content_type: &'static str, body: &[u8]) -> Self {
        Self { status, content_type, body: body.to_vec() }
    }
}

pub(super) struct TestServer {
    pub(super) base_url: String,
    requests: mpsc::Receiver<String>,
    handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    pub(super) async fn spawn(responses: Vec<MockResponse>) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, requests) = mpsc::channel(responses.len().max(1));

        let handle = tokio::spawn(async move {
            for response in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let request = read_http_request(&mut stream).await;
                let _ = tx.send(request).await;
                stream.write_all(&http_response(response)).await.unwrap();
            }
        });

        Self { base_url: format!("http://{addr}"), requests, handle }
    }

    pub(super) async fn next_request(&mut self) -> String {
        timeout(Duration::from_secs(2), self.requests.recv())
            .await
            .expect("request should arrive")
            .expect("request channel should stay open")
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

fn http_response(response: MockResponse) -> Vec<u8> {
    let reason = match response.status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        500 => "Internal Server Error",
        _ => "Error",
    };
    let headers = format!(
        "HTTP/1.1 {} {reason}\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        response.status,
        response.content_type,
        response.body.len()
    );
    let mut raw = headers.into_bytes();
    raw.extend_from_slice(&response.body);
    raw
}

async fn read_http_request(stream: &mut tokio::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0_u8; 1024];

    loop {
        let n = stream.read(&mut tmp).await.unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);

        let Some(header_end) = buf.windows(4).position(|window| window == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&buf[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);

        if buf.len() >= header_end + 4 + content_length {
            break;
        }
    }

    String::from_utf8_lossy(&buf).into_owned()
}

pub(super) fn request_path(request: &str) -> &str {
    request.lines().next().and_then(|line| line.split_whitespace().nth(1)).unwrap_or_default()
}

fn request_body(request: &str) -> &str {
    request.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or_default()
}

pub(super) fn request_json_body(request: &str) -> serde_json::Value {
    serde_json::from_str(request_body(request)).unwrap()
}

pub(super) fn header_value(request: &str, header_name: &str) -> Option<String> {
    request.lines().take_while(|line| !line.is_empty()).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case(header_name).then(|| value.trim().to_string())
    })
}

fn channel() -> LarkChannel {
    LarkChannel::new(
        "app-id".to_string(),
        "app-secret".to_string(),
        "verify".to_string(),
        None,
        vec!["*".to_string()],
        false,
    )
}

pub(super) fn test_channel(base_url: &str) -> LarkChannel {
    channel().with_api_base(base_url.to_string())
}

pub(super) fn mention_channel(base_url: &str) -> LarkChannel {
    LarkChannel::new(
        "app-id".to_string(),
        "app-secret".to_string(),
        "verify".to_string(),
        None,
        vec!["*".to_string()],
        true,
    )
    .with_api_base(base_url.to_string())
}

pub(super) async fn cache_token(ch: &LarkChannel, value: &str, refresh_after: Instant) {
    let mut cached = ch.tenant_token.write().await;
    *cached = Some(CachedTenantToken { value: value.to_string(), refresh_after });
}
