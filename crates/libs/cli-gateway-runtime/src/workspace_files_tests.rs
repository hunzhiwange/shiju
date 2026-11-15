use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use vw_gateway_client::{GatewayClient, GatewayEndpoint};

use super::gateway::{GatewaySessionSeed, GatewayUiRuntime};
use super::workspace_files::{
    load_workspace_files_blocking, read_workspace_file_blocking, workspace_text_from_parts,
};

#[test]
fn workspace_file_browser_rejects_escape_and_reads_selected_text() {
    let (endpoint, requests, server) = workspace_files_fixture();
    let runtime = runtime(endpoint);

    assert!(load_workspace_files_blocking(&runtime, "../secret").is_err());
    assert!(read_workspace_file_blocking(&runtime, "/etc/passwd").is_err());

    let listing = load_workspace_files_blocking(&runtime, ".")
        .expect("workspace root listing should load from Gateway");
    assert_eq!(listing.path(), ".");
    assert!(listing.items().iter().any(|item| item.id == "directory:src"));
    assert!(listing.items().iter().any(|item| item.id == "file:README.md"));

    let text = read_workspace_file_blocking(&runtime, "README.md")
        .expect("selected text file should load from Gateway");
    let rendered = text.render_text();
    assert!(rendered.starts_with("README.md\n"));
    assert!(rendered.contains("workspace file"));
    assert!(rendered.contains("[内容已截断]"));
    assert!(rendered.chars().count() < 17_000, "file display must remain bounded");

    let requests =
        requests.recv_timeout(Duration::from_secs(3)).expect("record workspace file requests");
    assert_eq!(requests.len(), 2, "escape rejection must not make a request");
    assert_eq!(requests[0].path, "/v1/file?directory=%2Fworkspace&path=.");
    assert_eq!(requests[1].path, "/v1/file/read");
    assert!(requests[1].body.contains(r#"\"directory\":\"/workspace\""#));
    assert!(requests[1].body.contains(r#"\"path\":\"README.md\""#));
    server.join().expect("workspace files fixture");
}

#[test]
fn workspace_file_response_rejects_mismatched_paths_and_binary_text() {
    let root = std::path::Path::new("/workspace");
    assert!(
        workspace_text_from_parts(root, "README.md", "/other", "README.md", "text".to_string(),)
            .is_err()
    );
    assert!(
        workspace_text_from_parts(
            root,
            "README.md",
            "/workspace",
            "other.txt",
            "text".to_string(),
        )
        .is_err()
    );
    assert!(
        workspace_text_from_parts(
            root,
            "README.md",
            "/workspace",
            "README.md",
            "binary\0payload".to_string(),
        )
        .is_err()
    );
}

fn runtime(endpoint: GatewayEndpoint) -> GatewayUiRuntime {
    let client = GatewayClient::new(endpoint).expect("test gateway client");
    GatewayUiRuntime::new(client, GatewaySessionSeed::new(PathBuf::from("/workspace")))
}

#[derive(Debug)]
struct RecordedRequest {
    path: String,
    body: String,
}

fn workspace_files_fixture()
-> (GatewayEndpoint, mpsc::Receiver<Vec<RecordedRequest>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind workspace files fixture");
    let port = listener.local_addr().expect("workspace files fixture address").port();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();

        let (mut list_stream, _) = listener.accept().expect("accept workspace listing request");
        requests.push(read_request(&mut list_stream));
        write_json_response(
            &mut list_stream,
            r#"[{"path":"src","name":"src","type":"directory"},{"path":"README.md","name":"README.md","type":"file"}]"#,
        );

        let (mut read_stream, _) = listener.accept().expect("accept workspace read request");
        requests.push(read_request(&mut read_stream));
        let content = format!("workspace file\n{}", "x".repeat(20_000));
        let body = serde_json::json!({
            "root_directory": "/workspace",
            "path": "README.md",
            "content": content,
        })
        .to_string();
        write_json_response(&mut read_stream, body.as_str());
        sender.send(requests).expect("record workspace file requests");
    });
    (GatewayEndpoint::new("127.0.0.1", port), receiver, handle)
}

fn read_request(stream: &mut TcpStream) -> RecordedRequest {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    let header_end = loop {
        let read = stream.read(&mut buffer).expect("read workspace file request");
        assert!(read > 0, "workspace file request closed before headers");
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..header_end]);
    let mut lines = headers.lines();
    let path = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .expect("workspace file request path")
        .to_string();
    let content_length = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or_default();
    let body_start = header_end.saturating_add(4);
    while bytes.len().saturating_sub(body_start) < content_length {
        let read = stream.read(&mut buffer).expect("read workspace file request body");
        assert!(read > 0, "workspace file request closed before body");
        bytes.extend_from_slice(&buffer[..read]);
    }
    let body = String::from_utf8(bytes[body_start..body_start + content_length].to_vec())
        .expect("workspace file request body is UTF-8 JSON");
    RecordedRequest { path, body }
}

fn write_json_response(stream: &mut TcpStream, body: &str) {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("write workspace file response");
}
