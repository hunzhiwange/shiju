use std::path::PathBuf;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
};

use serde_json::{Value, json};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
use vw_shared::session::ui_types::{ChatSession, ChatSessionMeta};

use super::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::session_store::UiSessionCreateInfo;

fn runtime(session_id: Option<&str>) -> GatewayUiRuntime {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 42617)).unwrap();
    let mut seed = GatewaySessionSeed::new(PathBuf::from("/tmp/session-store-tests"));
    if let Some(session_id) = session_id {
        seed = seed.with_id(Some(session_id.to_string()));
    }
    GatewayUiRuntime::new(client, seed)
}

fn runtime_for_server(
    server: &TestServer,
    session_id: Option<&str>,
    directory: &str,
) -> GatewayUiRuntime {
    let client =
        GatewayClient::new(GatewayEndpoint::new("127.0.0.1", server.port)).expect("client");
    let mut seed = GatewaySessionSeed::new(PathBuf::from(directory));
    if let Some(session_id) = session_id {
        seed = seed.with_id(Some(session_id.to_string()));
    }
    GatewayUiRuntime::new(client, seed)
}

fn empty_session(id: &str) -> ChatSession {
    ChatSession {
        id: id.to_string(),
        title: "Title".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    }
}

fn preview_meta(id: &str) -> ChatSessionMeta {
    ChatSessionMeta {
        id: id.to_string(),
        title: "Title".to_string(),
        updated_ms: 2,
        message_count: 1,
        call_count: 0,
        last_content: Some("hello".to_string()),
    }
}

#[derive(Debug)]
struct RecordedRequest {
    method: String,
    path: String,
    body: Value,
}

struct TestServer {
    port: u16,
    requests: mpsc::Receiver<RecordedRequest>,
    handle: thread::JoinHandle<()>,
}

impl TestServer {
    fn take_request(&self) -> RecordedRequest {
        self.requests.recv().expect("recorded request")
    }

    fn join(self) {
        self.handle.join().expect("server thread");
    }
}

fn server(responses: Vec<(u16, Value)>) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let port = listener.local_addr().expect("local addr").port();
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().expect("accept request");
            let request = read_request(&mut stream);
            tx.send(request).expect("record request");
            let body = body.to_string();
            write_response(&mut stream, status, &body);
        }
    });
    TestServer { port, requests: rx, handle }
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
    let content_length = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
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

    RecordedRequest { method, path, body }
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn write_response(stream: &mut TcpStream, status: u16, body: &str) {
    let reason = match status {
        200 => "OK",
        201 => "Created",
        404 => "Not Found",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).expect("write response");
}

#[test]
fn session_create_info_deserializes_gateway_response_shape() {
    let info: UiSessionCreateInfo =
        serde_json::from_value(json!({"id": "s1", "title": "New"})).unwrap();

    assert_eq!(info.id, "s1");
    assert_eq!(info.title, "New");
}

#[tokio::test]
async fn async_session_store_methods_forward_session_directory_and_payloads() {
    let server = server(vec![
        (200, json!({"id": "created", "title": "Created"})),
        (200, serde_json::to_value(empty_session("bound")).unwrap()),
        (200, serde_json::to_value(empty_session("any")).unwrap()),
        (200, json!(true)),
        (200, json!([preview_meta("p1")])),
        (200, serde_json::to_value(preview_meta("preview")).unwrap()),
        (200, json!("/repo/session.json")),
        (200, json!("global-scope")),
        (200, json!(true)),
    ]);
    let runtime = runtime_for_server(&server, Some(" bound "), "/repo");

    assert_eq!(runtime.session_create(Some(" New ")).await.unwrap().id, "created");
    assert_eq!(runtime.session_ui_load(None).await.unwrap().unwrap().id, "bound");
    assert_eq!(runtime.session_ui_load_any(Some(" any ")).await.unwrap().unwrap().id, "any");
    runtime.session_ui_save(&empty_session("saved")).await.unwrap();
    assert_eq!(runtime.session_ui_previews().await.unwrap()[0].id, "p1");
    assert_eq!(
        runtime.session_preview_meta(Some(" preview ")).await.unwrap().unwrap().id,
        "preview"
    );
    assert_eq!(
        runtime.session_path(Some(" path ")).await.unwrap().unwrap(),
        PathBuf::from("/repo/session.json")
    );
    assert_eq!(runtime.session_scope_get().await.unwrap().as_deref(), Some("global-scope"));
    runtime.session_scope_put(Some(" scope ")).await.unwrap();

    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/session?directory=%2Frepo");
    assert_eq!(request.body["title"], "New");
    assert_eq!(server.take_request().path, "/v1/session/bound/ui?directory=%2Frepo");
    assert_eq!(server.take_request().path, "/v1/session/any/any");
    let request = server.take_request();
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, "/v1/session/saved/ui?directory=%2Frepo");
    assert_eq!(request.body["id"], "saved");
    assert_eq!(server.take_request().path, "/v1/session/ui-previews?directory=%2Frepo");
    assert_eq!(server.take_request().path, "/v1/session/preview/preview?directory=%2Frepo");
    assert_eq!(server.take_request().path, "/v1/session/path/path?directory=%2Frepo");
    assert_eq!(server.take_request().path, "/v1/session/scope");
    let request = server.take_request();
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, "/v1/session/scope");
    assert_eq!(request.body["scope"], "scope");
    server.join();
}

#[tokio::test]
async fn session_commands_use_gateway_routes_and_typed_payloads() {
    let server = server(vec![
        (200, json!({"id": "bound", "title": "Renamed"})),
        (200, json!({"id": "forked", "title": "Forked"})),
        (200, json!({"id": "bound", "title": "Renamed"})),
        (200, json!(true)),
    ]);
    let runtime = runtime_for_server(&server, Some("bound"), "/repo");

    assert_eq!(runtime.session_update_title(None, "Renamed").await.unwrap().title, "Renamed");
    assert_eq!(runtime.session_fork_from(None, Some("message-1")).await.unwrap().id, "forked");
    runtime.session_reset_to(None, "message-2", true).await.unwrap();
    assert!(runtime.session_summarize_from(None, "message-3").await.unwrap());

    let request = server.take_request();
    assert_eq!(request.method, "PATCH");
    assert_eq!(request.path, "/v1/session/bound?directory=%2Frepo");
    assert_eq!(request.body["title"], "Renamed");
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/session/bound/fork?directory=%2Frepo");
    assert_eq!(request.body["messageID"], "message-1");
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/session/bound/reset?directory=%2Frepo");
    assert_eq!(request.body["messageID"], "message-2");
    assert_eq!(request.body["revertCode"], true);
    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/session/bound/summarize?directory=%2Frepo");
    assert_eq!(request.body["messageID"], "message-3");
    server.join();
}

#[tokio::test]
async fn session_create_propagates_gateway_http_error() {
    let server = server(vec![(500, json!({"error": "creation denied"}))]);
    let runtime = runtime_for_server(&server, None, "/repo");

    let error = runtime
        .session_create(Some("Denied"))
        .await
        .expect_err("gateway status errors should be returned");

    assert!(error.contains("gateway request failed: 500"));
    assert!(error.contains("creation denied"));
    assert_eq!(server.take_request().path, "/v1/session?directory=%2Frepo");
    server.join();
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn blocking_session_store_methods_use_same_gateway_routes() {
    let server = server(vec![
        (200, json!({"id": "created", "title": "Created"})),
        (200, serde_json::to_value(empty_session("bound")).unwrap()),
        (200, serde_json::to_value(empty_session("any")).unwrap()),
        (200, json!(true)),
        (200, json!([preview_meta("p1")])),
        (200, serde_json::to_value(preview_meta("preview")).unwrap()),
        (200, json!("/repo/session.json")),
        (200, json!("global-scope")),
        (200, json!(true)),
    ]);
    let runtime = runtime_for_server(&server, Some("bound"), " ");

    assert_eq!(runtime.session_create_blocking(None).unwrap().title, "Created");
    assert_eq!(runtime.session_ui_load_blocking(None).unwrap().unwrap().id, "bound");
    assert_eq!(runtime.session_ui_load_any_blocking(Some("any")).unwrap().unwrap().id, "any");
    runtime.session_ui_save_blocking(&empty_session("saved")).unwrap();
    assert_eq!(runtime.session_ui_previews_blocking().unwrap()[0].id, "p1");
    assert_eq!(
        runtime.session_preview_meta_blocking(Some("preview")).unwrap().unwrap().id,
        "preview"
    );
    assert_eq!(
        runtime.session_path_blocking(Some("path")).unwrap().unwrap(),
        PathBuf::from("/repo/session.json")
    );
    assert_eq!(runtime.session_scope_get_blocking().unwrap().as_deref(), Some("global-scope"));
    runtime.session_scope_put_blocking(Some(" ")).unwrap();

    let request = server.take_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/v1/session");
    assert!(request.body.get("title").is_none());
    assert_eq!(server.take_request().path, "/v1/session/bound/ui");
    assert_eq!(server.take_request().path, "/v1/session/any/any");
    assert_eq!(server.take_request().path, "/v1/session/saved/ui");
    assert_eq!(server.take_request().path, "/v1/session/ui-previews");
    assert_eq!(server.take_request().path, "/v1/session/preview/preview");
    assert_eq!(server.take_request().path, "/v1/session/path/path");
    assert_eq!(server.take_request().path, "/v1/session/scope");
    let request = server.take_request();
    assert_eq!(request.method, "PUT");
    assert_eq!(request.path, "/v1/session/scope");
    assert!(request.body["scope"].is_null());
    server.join();
}

#[tokio::test]
async fn load_preview_and_path_require_explicit_or_bound_session_id() {
    let runtime = runtime(None);

    assert_eq!(
        runtime.session_ui_load(None).await.unwrap_err(),
        "gateway runtime session id is required"
    );
    assert_eq!(
        runtime.session_ui_load_any(Some("  ")).await.unwrap_err(),
        "gateway runtime session id is required"
    );
    assert_eq!(
        runtime.session_preview_meta(None).await.unwrap_err(),
        "gateway runtime session id is required"
    );
    assert_eq!(
        runtime.session_path(None).await.unwrap_err(),
        "gateway runtime session id is required"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn blocking_load_wrappers_return_session_id_errors_before_network() {
    let runtime = runtime(None);

    assert_eq!(
        runtime.session_ui_load_blocking(None).unwrap_err(),
        "gateway runtime session id is required"
    );
    assert_eq!(
        runtime.session_ui_load_any_blocking(Some(" ")).unwrap_err(),
        "gateway runtime session id is required"
    );
    assert_eq!(
        runtime.session_preview_meta_blocking(None).unwrap_err(),
        "gateway runtime session id is required"
    );
    assert_eq!(
        runtime.session_path_blocking(None).unwrap_err(),
        "gateway runtime session id is required"
    );
}

#[tokio::test]
async fn save_rejects_blank_session_id_before_network() {
    let runtime = runtime(Some("bound"));

    assert_eq!(
        runtime.session_ui_save(&empty_session("  ")).await.unwrap_err(),
        "session_ui save requires a non-empty session id"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn blocking_save_rejects_blank_session_id_before_network() {
    let runtime = runtime(Some("bound"));

    assert_eq!(
        runtime.session_ui_save_blocking(&empty_session("")).unwrap_err(),
        "session_ui save requires a non-empty session id"
    );
}
