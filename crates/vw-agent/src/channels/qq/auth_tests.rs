use super::*;
use serde_json::json;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc,
    time::timeout,
};

struct TestHttpServer {
    base_url: String,
    requests: mpsc::Receiver<String>,
    handle: tokio::task::JoinHandle<()>,
}

impl TestHttpServer {
    async fn spawn(responses: Vec<(u16, String)>) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, requests) = mpsc::channel(responses.len().max(1));

        let handle = tokio::spawn(async move {
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let request = read_http_request(&mut stream).await;
                let _ = tx.send(request).await;
                let response = http_response(status, &body);
                stream.write_all(response.as_bytes()).await.unwrap();
            }
        });

        Self { base_url: format!("http://{addr}"), requests, handle }
    }

    async fn next_request(&mut self) -> String {
        timeout(Duration::from_secs(2), self.requests.recv())
            .await
            .expect("request should arrive")
            .expect("request channel should stay open")
    }
}

impl Drop for TestHttpServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

fn response(status: u16, body: impl Into<String>) -> (u16, String) {
    (status, body.into())
}

fn http_response(status: u16, body: &str) -> String {
    let reason = match status {
        200 => "OK",
        401 => "Unauthorized",
        500 => "Internal Server Error",
        _ => "Error",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
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

fn request_body(request: &str) -> &str {
    request.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or_default()
}

fn request_json_body(request: &str) -> serde_json::Value {
    serde_json::from_str(request_body(request)).unwrap()
}

fn request_header<'a>(request: &'a str, name: &str) -> Option<&'a str> {
    request.lines().find_map(|line| {
        let (header_name, value) = line.split_once(':')?;
        header_name.eq_ignore_ascii_case(name).then_some(value.trim())
    })
}

fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

#[test]
fn ensure_https_accepts_only_https_urls() {
    assert!(ensure_https("https://api.sgroup.qq.com").is_ok());
    assert!(ensure_https("http://api.sgroup.qq.com").is_err());
    assert!(ensure_https("file:///tmp/token").is_err());
    assert!(ensure_https("HTTPS://api.sgroup.qq.com").is_err());
    assert!(ensure_https(" https://api.sgroup.qq.com").is_err());
}

#[tokio::test]
async fn fetch_access_token_posts_credentials_and_adjusts_expiry() {
    let mut server = TestHttpServer::spawn(vec![response(
        200,
        json!({
            "access_token": "token-1",
            "expires_in": "120"
        })
        .to_string(),
    )])
    .await;
    let channel = QQChannel::new("app-1".to_string(), "secret-1".to_string(), vec![])
        .with_auth_url(format!("{}/token", server.base_url));
    let before = unix_now();

    let (token, expiry) = channel.fetch_access_token().await.unwrap();
    let after = unix_now();
    let request = server.next_request().await;

    assert!(request.starts_with("POST /token "));
    assert_eq!(
        request_json_body(&request),
        json!({
            "appId": "app-1",
            "clientSecret": "secret-1"
        })
    );
    assert_eq!(token, "token-1");
    assert!(expiry >= before + 60);
    assert!(expiry <= after + 60);
}

#[tokio::test]
async fn fetch_access_token_defaults_expiry_when_missing() {
    let mut server = TestHttpServer::spawn(vec![response(
        200,
        json!({
            "access_token": "token-2"
        })
        .to_string(),
    )])
    .await;
    let channel = QQChannel::new("app-2".to_string(), "secret-2".to_string(), vec![])
        .with_auth_url(format!("{}/token", server.base_url));
    let before = unix_now();

    let (token, expiry) = channel.fetch_access_token().await.unwrap();
    let after = unix_now();

    assert_eq!(server.next_request().await.lines().next().unwrap(), "POST /token HTTP/1.1");
    assert_eq!(token, "token-2");
    assert!(expiry >= before + 7_140);
    assert!(expiry <= after + 7_140);
}

#[tokio::test]
async fn fetch_access_token_reports_http_errors() {
    let mut server = TestHttpServer::spawn(vec![response(401, "denied")]).await;
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec![])
        .with_auth_url(format!("{}/token", server.base_url));

    let err = channel.fetch_access_token().await.unwrap_err().to_string();

    assert_eq!(server.next_request().await.lines().next().unwrap(), "POST /token HTTP/1.1");
    assert!(err.contains("QQ token request failed (401 Unauthorized): denied"));
}

#[tokio::test]
async fn fetch_access_token_requires_access_token_field() {
    let mut server = TestHttpServer::spawn(vec![response(
        200,
        json!({
            "expires_in": "120"
        })
        .to_string(),
    )])
    .await;
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec![])
        .with_auth_url(format!("{}/token", server.base_url));

    let err = channel.fetch_access_token().await.unwrap_err().to_string();

    assert_eq!(server.next_request().await.lines().next().unwrap(), "POST /token HTTP/1.1");
    assert!(err.contains("Missing access_token in QQ response"));
}

#[tokio::test]
async fn get_token_uses_cached_value_without_refresh() {
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec![])
        .with_auth_url("http://127.0.0.1:1/token".to_string());
    {
        let mut cache = channel.token_cache.write().await;
        *cache = Some(("cached-token".to_string(), unix_now() + 3_600));
    }

    let token = channel.get_token().await.unwrap();

    assert_eq!(token, "cached-token");
}

#[tokio::test]
async fn get_token_refreshes_expired_cache_and_updates_it() {
    let mut server = TestHttpServer::spawn(vec![response(
        200,
        json!({
            "access_token": "fresh-token",
            "expires_in": "90"
        })
        .to_string(),
    )])
    .await;
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec![])
        .with_auth_url(format!("{}/token", server.base_url));
    {
        let mut cache = channel.token_cache.write().await;
        *cache = Some(("stale-token".to_string(), 1));
    }
    let before = unix_now();

    let token = channel.get_token().await.unwrap();
    let after = unix_now();
    let request = server.next_request().await;
    let cache = channel.token_cache.read().await.clone();

    assert!(request.starts_with("POST /token "));
    assert_eq!(token, "fresh-token");
    assert_eq!(cache.as_ref().unwrap().0, "fresh-token");
    assert!(cache.as_ref().unwrap().1 >= before + 30);
    assert!(cache.as_ref().unwrap().1 <= after + 30);
}

#[tokio::test]
async fn get_gateway_url_uses_authorization_header_and_returns_url() {
    let mut server = TestHttpServer::spawn(vec![response(
        200,
        json!({
            "url": "wss://gateway.qq.test/ws"
        })
        .to_string(),
    )])
    .await;
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec![])
        .with_api_base(server.base_url.clone());

    let url = channel.get_gateway_url("bot-token").await.unwrap();
    let request = server.next_request().await;

    assert_eq!(url, "wss://gateway.qq.test/ws");
    assert!(request.starts_with("GET /gateway "));
    assert_eq!(request_header(&request, "authorization"), Some("QQBot bot-token"));
}

#[tokio::test]
async fn get_gateway_url_reports_http_errors() {
    let mut server = TestHttpServer::spawn(vec![response(500, "oops")]).await;
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec![])
        .with_api_base(server.base_url.clone());

    let err = channel.get_gateway_url("bot-token").await.unwrap_err().to_string();

    assert_eq!(server.next_request().await.lines().next().unwrap(), "GET /gateway HTTP/1.1");
    assert!(err.contains("QQ gateway request failed (500 Internal Server Error): oops"));
}

#[tokio::test]
async fn get_gateway_url_requires_url_field() {
    let mut server = TestHttpServer::spawn(vec![response(
        200,
        json!({
            "session_start_limit": { "remaining": 1 }
        })
        .to_string(),
    )])
    .await;
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec![])
        .with_api_base(server.base_url.clone());

    let err = channel.get_gateway_url("bot-token").await.unwrap_err().to_string();

    assert_eq!(server.next_request().await.lines().next().unwrap(), "GET /gateway HTTP/1.1");
    assert!(err.contains("Missing gateway URL in QQ response"));
}
