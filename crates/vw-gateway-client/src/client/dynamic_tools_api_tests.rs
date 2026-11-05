use super::{GatewayClient, test_support::server};
use crate::{
    endpoint::{GatewayAuth, GatewayEndpoint},
    stream::GatewayStreamAttemptResult,
};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
    time::Duration,
};
use vw_api_types::tools::{
    DynamicToolHostBinding, DynamicToolResponse, ReplaceDynamicToolsRequest,
};

fn binding() -> DynamicToolHostBinding {
    DynamicToolHostBinding { host_id: "host".into(), binding_token: "binding-secret".into() }
}
fn response() -> DynamicToolResponse {
    serde_json::from_value(json!({"success":true,"content":[{"type":"text","text":"done"}]}))
        .unwrap()
}
fn call_frame() -> String {
    format!(
        "event: dynamic_tool.call\r\ndata: {}\r\n\r\n",
        json!({
            "call_id":"call", "turn_id":"turn", "tool_id":"client.echo",
            "identity":{"name":"echo","namespace":null}, "arguments":{"text":"中文"}
        })
    )
}

struct WireReply {
    status: u16,
    content_type: &'static str,
    chunks: Vec<Vec<u8>>,
    watch_close: bool,
}
impl WireReply {
    fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            content_type: "application/json",
            chunks: vec![body.to_string().into_bytes()],
            watch_close: false,
        }
    }
    fn sse(chunks: Vec<Vec<u8>>, watch_close: bool) -> Self {
        Self { status: 200, content_type: "text/event-stream", chunks, watch_close }
    }
}
struct Loopback {
    client: GatewayClient,
    requests: mpsc::Receiver<String>,
    closed: mpsc::Receiver<()>,
    thread: thread::JoinHandle<()>,
}
impl Loopback {
    fn start(replies: Vec<WireReply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, requests) = mpsc::channel();
        let (close_tx, closed) = mpsc::channel();
        let thread = thread::spawn(move || {
            let mut workers = Vec::new();
            for reply in replies {
                let (mut socket, _) = listener.accept().unwrap();
                let tx = tx.clone();
                let close_tx = close_tx.clone();
                workers.push(thread::spawn(move || {
                socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                socket.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
                tx.send(read_request(&mut socket)).unwrap();
                write!(socket, "HTTP/1.1 {} Test\r\ncontent-type: {}\r\ntransfer-encoding: chunked\r\nconnection: close\r\n\r\n", reply.status, reply.content_type).unwrap();
                for chunk in reply.chunks {
                    write!(socket, "{:X}\r\n", chunk.len()).unwrap();
                    socket.write_all(&chunk).unwrap();
                    socket.write_all(b"\r\n").unwrap();
                    socket.flush().unwrap();
                }
                if reply.watch_close {
                    let mut byte = [0];
                    match socket.read(&mut byte) {
                        Ok(0) => {}
                        Err(error)
                            if matches!(
                                error.kind(),
                                std::io::ErrorKind::ConnectionReset
                                    | std::io::ErrorKind::ConnectionAborted
                            ) => {}
                        result => panic!("SSE connection not released: {result:?}"),
                    }
                    close_tx.send(()).unwrap();
                } else {
                    socket.write_all(b"0\r\n\r\n").unwrap();
                }
                }));
            }
            for worker in workers {
                worker.join().unwrap();
            }
        });
        let endpoint = GatewayEndpoint::new("127.0.0.1", port)
            .with_auth(GatewayAuth { skey: Some("gateway-key".into()) });
        Self { client: GatewayClient::new(endpoint).unwrap(), requests, closed, thread }
    }
    fn request(&self) -> String {
        self.requests.recv_timeout(Duration::from_secs(5)).unwrap()
    }
    fn join(self) {
        self.thread.join().unwrap();
    }
}
fn read_request(socket: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buf = [0; 1024];
    loop {
        let n = socket.read(&mut buf).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&buf[..n]);
        if let Some(end) = bytes.windows(4).position(|value| value == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..end]);
            let len = headers
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            if bytes.len() >= end + 4 + len {
                return String::from_utf8(bytes).unwrap();
            }
        }
    }
}
fn assert_context(request: &str, method_path: &str, bound: bool) {
    assert!(request.starts_with(method_path), "{request}");
    assert!(request.contains("authorization: Bearer gateway-key\r\n"));
    assert!(request.contains("x-platform-id:"));
    assert!(request.contains("x-company-id:"));
    assert_eq!(request.contains("x-vibewindow-tool-host: binding-secret\r\n"), bound);
    assert!(!request.lines().next().unwrap().contains("binding-secret"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dynamic_tools_client_host_roundtrip_completes() {
    let frame = call_frame();
    let split = frame.find('中').unwrap() + 1;
    let server = Loopback::start(vec![
        WireReply::json(200, json!({"host_id":"host", "binding_token":"binding-secret"})),
        WireReply::json(200, json!({"version":2})),
        WireReply::sse(
            vec![
                b":keepalive\r\n\r\n".to_vec(),
                frame.as_bytes()[..split].to_vec(),
                frame.as_bytes()[split..].to_vec(),
            ],
            true,
        ),
        WireReply::json(204, Value::Null),
        WireReply::json(403, json!({"message":"already responded"})),
    ]);
    let client = &server.client;
    let binding = client.register_dynamic_tool_host("session", Some("/workspace")).await.unwrap();
    assert_eq!(binding.host_id, "host");
    assert_context(
        &server.request(),
        "POST /v1/session/session/dynamic-tools/hosts?directory=%2Fworkspace ",
        false,
    );
    let replace: ReplaceDynamicToolsRequest = serde_json::from_value(json!({"tools":[{
        "type":"function", "identity":{"name":"echo","namespace":null},
        "source":"client", "exposure":"deferred", "id":"client.echo",
        "display_name":"Echo", "description":"Echo text", "input_schema":{"type":"object"}
    }],"persist":false}))
    .unwrap();
    assert_eq!(
        client
            .replace_dynamic_tools("session", Some("/workspace"), &binding, &replace)
            .await
            .unwrap()
            .version,
        2
    );
    let request = server.request();
    assert_context(&request, "PUT /v1/session/session/dynamic-tools?directory=%2Fworkspace ", true);
    assert_eq!(
        serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
        serde_json::to_value(&replace).unwrap()
    );
    let (call_tx, call_rx) = mpsc::channel();
    let stream_client = client.clone();
    let stream_binding = binding.clone();
    let stream = tokio::spawn(async move {
        stream_client
            .stream_dynamic_tool_calls("session", Some("/workspace"), &stream_binding, |call| {
                call_tx.send(call).unwrap();
                true
            })
            .await
    });
    let call =
        tokio::task::spawn_blocking(move || call_rx.recv_timeout(Duration::from_secs(5)).unwrap())
            .await
            .unwrap();
    assert_eq!(call.arguments["text"], "中文");
    assert_eq!(call.identity.name, "echo");
    assert_context(
        &server.request(),
        "GET /v1/session/session/dynamic-tools/calls?directory=%2Fworkspace ",
        true,
    );
    client
        .respond_dynamic_tool_call(
            "session",
            Some("/workspace"),
            &binding,
            &call.call_id,
            &response(),
        )
        .await
        .unwrap();
    let request = server.request();
    assert_context(
        &request,
        "POST /v1/session/session/dynamic-tools/calls/call/response?directory=%2Fworkspace ",
        true,
    );
    assert_eq!(
        serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
        serde_json::to_value(response()).unwrap()
    );
    assert!(
        client
            .respond_dynamic_tool_call("session", None, &binding, "call", &response())
            .await
            .unwrap_err()
            .contains("403")
    );
    server.request();
    stream.abort();
    assert!(stream.await.unwrap_err().is_cancelled());
    tokio::task::block_in_place(|| server.closed.recv_timeout(Duration::from_secs(5)).unwrap());
    server.join();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dynamic_tools_client_callback_stop_releases_host_connection() {
    let server = Loopback::start(vec![WireReply::sse(vec![call_frame().into_bytes()], true)]);
    assert_eq!(
        server
            .client
            .stream_dynamic_tool_calls("session", None, &binding(), |_| false)
            .await
            .unwrap(),
        GatewayStreamAttemptResult::CallerStopped
    );
    tokio::task::block_in_place(|| server.closed.recv_timeout(Duration::from_secs(5)).unwrap());
    server.join();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dynamic_tools_client_dropped_future_releases_host_connection() {
    let server = Loopback::start(vec![WireReply::sse(vec![b": ready\n\n".to_vec()], true)]);
    let (client, host) = (server.client.clone(), binding());
    let future = tokio::spawn(async move {
        client.stream_dynamic_tool_calls("session", None, &host, |_| true).await
    });
    // Receiving the recorded request proves the connection was established.
    tokio::task::block_in_place(|| {
        server.request();
    });
    future.abort();
    assert!(future.await.unwrap_err().is_cancelled());
    tokio::task::block_in_place(|| server.closed.recv_timeout(Duration::from_secs(5)).unwrap());
    server.join();
}

#[tokio::test]
async fn dynamic_tools_client_error_status_and_expired_binding_are_returned() {
    let server = server(vec![
        (401, json!({"message":"authentication required"})),
        (403, json!({"message":"expired binding"})),
        (400, json!({"message":"persistent tools unsupported"})),
        (403, json!({"message":"expired binding"})),
    ]);
    assert!(
        server
            .client()
            .register_dynamic_tool_host("session", None)
            .await
            .err()
            .unwrap()
            .contains("401")
    );
    assert!(
        server
            .client()
            .stream_dynamic_tool_calls("session", None, &binding(), |_| true)
            .await
            .unwrap_err()
            .contains("403")
    );
    assert!(
        server
            .client()
            .replace_dynamic_tools(
                "session",
                None,
                &binding(),
                &ReplaceDynamicToolsRequest { tools: vec![], persist: true }
            )
            .await
            .unwrap_err()
            .contains("400")
    );
    assert!(
        server
            .client()
            .respond_dynamic_tool_call("session", None, &binding(), "call", &response())
            .await
            .unwrap_err()
            .contains("403")
    );
    for _ in 0..4 {
        server.take_request();
    }
    server.join();
}

#[tokio::test]
async fn dynamic_tools_client_partial_frame_disconnect_does_not_deliver_call() {
    let frame = call_frame();
    let server = Loopback::start(vec![WireReply::sse(
        vec![frame.as_bytes()[..frame.len() - 4].to_vec()],
        false,
    )]);
    let mut count = 0;
    assert_eq!(
        server
            .client
            .stream_dynamic_tool_calls("session", None, &binding(), |_| {
                count += 1;
                true
            })
            .await
            .unwrap(),
        GatewayStreamAttemptResult::Incomplete
    );
    assert_eq!(count, 0);
    server.join();
}

#[tokio::test]
async fn dynamic_tools_client_malformed_stream_is_rejected() {
    for (content_type, body) in [
        ("application/json", "{}"),
        ("text/event-stream", "event: unexpected\ndata: {}\n\n"),
        ("text/event-stream", "event: dynamic_tool.call\ndata: {}\n\n"),
    ] {
        let server = Loopback::start(vec![WireReply {
            status: 200,
            content_type,
            chunks: vec![body.as_bytes().to_vec()],
            watch_close: false,
        }]);
        assert!(
            server
                .client
                .stream_dynamic_tool_calls("session", None, &binding(), |_| panic!(
                    "must not deliver malformed calls"
                ))
                .await
                .is_err()
        );
        server.join();
    }
}

#[tokio::test]
async fn dynamic_tools_client_invalid_ids_and_binding_fail_before_network() {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 1)).unwrap();
    for id in ["", " ", ".", ".."] {
        assert!(client.register_dynamic_tool_host(id, None).await.err().unwrap().contains("ID"));
        assert!(
            client
                .respond_dynamic_tool_call("session", None, &binding(), id, &response())
                .await
                .unwrap_err()
                .contains("ID")
        );
    }
    let mut host = binding();
    host.binding_token = "bad\r\nheader".into();
    assert!(
        client
            .stream_dynamic_tool_calls("session", None, &host, |_| true)
            .await
            .unwrap_err()
            .contains("header")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dynamic_tools_client_response_transport_failure_is_not_retried() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (done_tx, done_rx) = mpsc::channel();
    let worker = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let request = read_request(&mut socket);
        assert!(request.starts_with("POST /v1/session/session/dynamic-tools/calls/call/response "));
        drop(socket); // The response may have executed, but its acknowledgement was lost.
        done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        listener.set_nonblocking(true).unwrap();
        assert_eq!(listener.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
    });
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    assert!(
        client
            .respond_dynamic_tool_call("session", None, &binding(), "call", &response())
            .await
            .is_err()
    );
    done_tx.send(()).unwrap();
    worker.join().unwrap();
}

#[tokio::test]
async fn dynamic_tools_client_redirect_is_rejected_before_forwarding_binding() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let worker = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let request = read_request(&mut socket);
        assert!(request.contains("x-vibewindow-tool-host: binding-secret"));
        socket.write_all(b"HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:1/forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
    });
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port)).unwrap();
    let error = client
        .respond_dynamic_tool_call("session", None, &binding(), "call", &response())
        .await
        .unwrap_err();
    assert!(error.contains("307"), "redirect must be returned without following it");
    worker.join().unwrap();
}
