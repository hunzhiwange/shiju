//! 网关客户端测试使用的最小 HTTP 固件。
//!
//! 本模块在本地监听一次性请求，记录方法、路径、请求头和 JSON 正文，以便测试线协议而不依赖外部服务。

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;

use serde_json::Value;

use super::GatewayClient;
use crate::endpoint::GatewayEndpoint;
use vw_api_types::id::TenantScope;

#[derive(Debug)]
pub struct RecordedRequest {
    pub method: String,
    pub path: String,
    pub headers: BTreeMap<String, String>,
    pub body: Value,
}

impl RecordedRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_ascii_lowercase()).map(String::as_str)
    }
}

pub struct TestServer {
    client: GatewayClient,
    base_url: String,
    requests: mpsc::Receiver<RecordedRequest>,
    handle: thread::JoinHandle<()>,
}

impl TestServer {
    pub fn client(&self) -> &GatewayClient {
        &self.client
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn take_request(&self) -> RecordedRequest {
        self.requests.recv().expect("recorded request")
    }

    pub fn join(self) {
        self.handle.join().expect("server thread");
    }
}

pub fn server(responses: Vec<(u16, Value)>) -> TestServer {
    server_raw(responses.into_iter().map(|(status, body)| (status, body.to_string())).collect())
}

pub fn server_with_tenant_scope(
    responses: Vec<(u16, Value)>,
    tenant_scope: TenantScope,
) -> TestServer {
    server_raw_with_tenant_scope(
        responses.into_iter().map(|(status, body)| (status, body.to_string())).collect(),
        tenant_scope,
    )
}

pub fn server_raw(responses: Vec<(u16, String)>) -> TestServer {
    server_raw_with_tenant_scope(responses, TenantScope::default())
}

pub fn server_raw_with_tenant_scope(
    responses: Vec<(u16, String)>,
    tenant_scope: TenantScope,
) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let port = listener.local_addr().expect("local addr").port();
    let (tx, rx) = mpsc::channel();
    let base_url = format!("http://127.0.0.1:{port}");
    let handle = thread::spawn(move || {
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().expect("accept request");
            let request = read_request(&mut stream);
            tx.send(request).expect("record request");
            write_response(&mut stream, status, &body);
        }
    });
    let endpoint = GatewayEndpoint::new("127.0.0.1", port).with_tenant_scope(tenant_scope);
    let client = GatewayClient::new(endpoint).expect("client");
    TestServer { client, base_url, requests: rx, handle }
}

fn read_request(stream: &mut TcpStream) -> RecordedRequest {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    let mut header_end = None;
    while header_end.is_none() {
        let read = stream.read(&mut buffer).expect("read request");
        assert!(read > 0, "connection closed before headers");
        bytes.extend_from_slice(&buffer[..read]);
        header_end = find_header_end(&bytes);
    }

    let header_end = header_end.expect("headers");
    let headers = String::from_utf8_lossy(&bytes[..header_end]).to_string();
    let mut lines = headers.lines();
    let request_line = lines.next().expect("request line");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().expect("method").to_string();
    let path = parts.next().expect("path").to_string();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect::<BTreeMap<_, _>>();
    let content_length = headers
        .get("content-length")
        .map(String::as_str)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);

    let mut body_bytes = bytes[(header_end + 4)..].to_vec();
    while body_bytes.len() < content_length {
        let read = stream.read(&mut buffer).expect("read body");
        assert!(read > 0, "connection closed before body");
        body_bytes.extend_from_slice(&buffer[..read]);
    }
    body_bytes.truncate(content_length);
    let body = if body_bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body_bytes).expect("json body")
    };

    RecordedRequest { method, path, headers, body }
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn write_response(stream: &mut TcpStream, status: u16, body: &str) {
    let reason = match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    let response = if status == 204 {
        format!(
            "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
        )
    } else {
        format!(
            "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        )
    };
    stream.write_all(response.as_bytes()).expect("write response");
}
