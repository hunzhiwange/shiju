use super::GatewayClient;
use crate::endpoint::GatewayEndpoint;
use crate::stream::{GatewayChatStreamEvent, GatewayChatStreamRequest};

#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::net::{SocketAddr, TcpListener};
#[cfg(not(target_arch = "wasm32"))]
use std::thread::JoinHandle;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

/// 最小 SSE 测试服务器，返回一次固定的 `text/event-stream` 响应。
#[cfg(not(target_arch = "wasm32"))]
struct StreamServer {
    addr: SocketAddr,
    handle: Option<JoinHandle<String>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl StreamServer {
    fn respond(status: &str, body: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let addr = listener.local_addr().expect("test server addr should be available");
        let status = status.to_string();
        let body = body.as_bytes().to_vec();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("test connection should be accepted");
            let mut request = [0_u8; 4096];
            let bytes_read = stream.read(&mut request).expect("request should be readable");
            let request = String::from_utf8_lossy(&request[..bytes_read]).to_string();
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("headers should be written");
            stream.write_all(&body).expect("body should be written");
            request
        });
        Self { addr, handle: Some(handle) }
    }

    fn endpoint(&self) -> GatewayEndpoint {
        GatewayEndpoint::new("127.0.0.1", self.addr.port())
    }

    fn join(&mut self) -> String {
        self.handle.take().expect("server should not be joined").join().unwrap()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for StreamServer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[test]
fn new_builds_client_and_exposes_endpoint() {
    let endpoint = GatewayEndpoint::new("example.test", 4321);
    let client = GatewayClient::new(endpoint).expect("client should build");

    assert_eq!(client.endpoint().host, "example.test");
    assert_eq!(client.endpoint().port, 4321);
    assert!(client.endpoint().auth.is_none());
}

#[test]
fn endpoint_returns_bound_configuration() {
    let endpoint = GatewayEndpoint::new("127.0.0.1", 9090);
    let client = GatewayClient::new(endpoint).expect("client should build");

    assert_eq!(client.endpoint().base_url(), "http://127.0.0.1:9090");
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn chat_stream_stop_uses_request_scoped_route() {
    let server = crate::client::test_support::server(vec![(
        200,
        serde_json::json!({ "cancelled": true }),
    )]);

    let cancelled = server
        .client()
        .chat_stream_stop("desktop-request-1")
        .await
        .expect("chat stream stop");

    assert!(cancelled);
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/chat/stream/desktop-request-1/stop");
    assert_eq!(request.body, serde_json::json!({}));
    server.join();
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn stream_chat_blocking_delegates_to_stream_module() {
    let mut server =
        StreamServer::respond("200 OK", "data: {\"type\":\"chat.delta\",\"delta\":\"a\"}\r\n\r\n");
    let endpoint = server.endpoint();
    let body = GatewayChatStreamRequest::default();
    let mut events = Vec::new();

    GatewayClient::stream_chat_blocking(&endpoint, None, &body, |event| {
        events.push(event);
        true
    })
    .expect("blocking stream should succeed");
    let request = server.join();

    assert_eq!(events, vec![GatewayChatStreamEvent::Delta("a".to_string())]);
    assert!(request.starts_with("POST /v1/chat/stream HTTP/1.1"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn stream_chat_blocking_stops_when_callback_declines_more_events() {
    let mut server = StreamServer::respond(
        "200 OK",
        concat!(
            "data: {\"type\":\"chat.delta\",\"delta\":\"first\"}\r\n\r\n",
            "data: {\"type\":\"chat.delta\",\"delta\":\"second\"}\r\n\r\n",
        ),
    );
    let endpoint = server.endpoint();
    let body = GatewayChatStreamRequest::default();
    let mut events = Vec::new();

    GatewayClient::stream_chat_blocking(&endpoint, Some("/workspace"), &body, |event| {
        events.push(event);
        false
    })
    .expect("callback should stop the stream without an error");
    let request = server.join();

    assert_eq!(events, vec![GatewayChatStreamEvent::Delta("first".to_string())]);
    assert!(request.starts_with("POST /v1/chat/stream?directory="));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn stream_chat_blocking_with_idle_timeout_delegates_to_stream_module() {
    let mut server =
        StreamServer::respond("200 OK", "data: {\"type\":\"chat.delta\",\"delta\":\"b\"}\r\n\r\n");
    let endpoint = server.endpoint();
    let body = GatewayChatStreamRequest::default();
    let mut events = Vec::new();

    GatewayClient::stream_chat_blocking_with_idle_timeout(
        &endpoint,
        None,
        &body,
        Duration::from_secs(5),
        |event| {
            events.push(event);
            true
        },
    )
    .expect("blocking stream with idle timeout should succeed");
    server.join();

    assert_eq!(events, vec![GatewayChatStreamEvent::Delta("b".to_string())]);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn stream_chat_delegates_to_stream_module() {
    let mut server =
        StreamServer::respond("200 OK", "data: {\"type\":\"chat.delta\",\"delta\":\"c\"}\r\n\r\n");
    let endpoint = server.endpoint();
    let body = GatewayChatStreamRequest::default();
    let mut events = Vec::new();

    GatewayClient::stream_chat(&endpoint, None, &body, |event| {
        events.push(event);
        true
    })
    .await
    .expect("async stream should succeed");
    server.join();

    assert_eq!(events, vec![GatewayChatStreamEvent::Delta("c".to_string())]);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn stream_chat_with_idle_timeout_delegates_to_stream_module() {
    let mut server =
        StreamServer::respond("200 OK", "data: {\"type\":\"chat.delta\",\"delta\":\"d\"}\r\n\r\n");
    let endpoint = server.endpoint();
    let body = GatewayChatStreamRequest::default();
    let mut events = Vec::new();

    GatewayClient::stream_chat_with_idle_timeout(
        &endpoint,
        None,
        &body,
        Duration::from_secs(5),
        |event| {
            events.push(event);
            true
        },
    )
    .await
    .expect("async stream with idle timeout should succeed");
    server.join();

    assert_eq!(events, vec![GatewayChatStreamEvent::Delta("d".to_string())]);
}
