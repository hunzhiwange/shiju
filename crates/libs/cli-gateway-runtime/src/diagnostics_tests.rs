use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread;

use vw_gateway_client::{GatewayClient, GatewayEndpoint};

use super::diagnostics::{load_gateway_diagnostics_blocking, sanitize_diagnostics_error};
use super::gateway::{GatewaySessionSeed, GatewayUiRuntime};

#[test]
fn gateway_diagnostics_use_doctor_get_and_only_render_bounded_fields() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind diagnostics fixture");
    let port = listener.local_addr().expect("diagnostics fixture address").port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept diagnostics request");
        let path = read_request_path(&mut stream);
        let long_message = "x".repeat(600);
        let body = serde_json::json!({
            "results": [
                {"severity":"ok", "category":"workspace", "message":"ready", "token":"do-not-render"},
                {"severity":"warn", "category":"config", "message":long_message},
                {"severity":"unknown", "category":"ignored", "message":"ignored"}
            ],
            "summary": {"ok": 99, "warnings": 99, "errors": 99},
            "authorization": "Bearer do-not-render"
        })
        .to_string();
        write_json_response(&mut stream, "200 OK", body.as_str());
        path
    });
    let runtime = runtime(GatewayEndpoint::new("127.0.0.1", port));

    let report = load_gateway_diagnostics_blocking(&runtime).expect("diagnostics should load");
    let rendered = report.render_text();

    assert_eq!(server.join().expect("join diagnostics fixture"), "/v1/doctor");
    assert!(rendered.contains("1 ok · 1 warnings · 0 errors"));
    assert!(rendered.contains("[ok] workspace: ready"));
    assert!(rendered.len() < 1_200, "diagnostics output must be bounded");
    assert!(!rendered.contains("do-not-render"));
    assert!(!rendered.contains("ignored"));
}

#[test]
fn diagnostics_errors_are_redacted_and_truncated() {
    let error = format!("authorization=Bearer-secret token=abc {}", "detail ".repeat(200));
    let sanitized = sanitize_diagnostics_error(&error);

    assert!(sanitized.contains("[redacted]"));
    assert!(!sanitized.contains("Bearer-secret"));
    assert!(!sanitized.contains("token=abc"));
    assert!(sanitized.chars().count() <= 320);
}

fn runtime(endpoint: GatewayEndpoint) -> GatewayUiRuntime {
    let client = GatewayClient::new(endpoint).expect("test gateway client");
    GatewayUiRuntime::new(client, GatewaySessionSeed::new(PathBuf::from("/workspace")))
}

fn read_request_path(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream.read(&mut buffer).expect("read diagnostics request");
        assert!(read > 0, "diagnostics request closed before headers");
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&bytes)
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .expect("diagnostics request path")
        .to_string()
}

fn write_json_response(stream: &mut TcpStream, status: &str, body: &str) {
    write!(
        stream,
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("write diagnostics response");
}
