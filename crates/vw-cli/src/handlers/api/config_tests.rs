//! `config` 命令的远程作用域、输入与确认测试。

use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, TryRecvError};

use clap::Parser;
use serde_json::{Value, json};

use super::{ApiCommands, run_with_io};
use crate::cli::Cli;

#[tokio::test]
async fn api_config_commands_preserve_scope_input_and_confirmation() {
    let responses = vec![
        (200, json!({"scope": "directory"})),
        (200, json!({"patched": true})),
        (200, json!({"scope": "global"})),
        (200, Value::Null),
        (200, json!({"agent": "codex"})),
        (200, json!({"format": "toml", "content": "model = 'o3'"})),
        (200, json!({"status": "ok"})),
        (200, json!({"providers": [], "default": {}})),
        (422, json!({"error": "invalid config"})),
    ];
    let (port, requests) = spawn_config_server(responses);
    let port = port.to_string();
    let common = [
        "--host",
        "127.0.0.1",
        "--port",
        port.as_str(),
        "--platform",
        "platform-a",
        "--company",
        "company-b",
        "--directory",
        "/tmp/workspace",
    ];

    let refused = parse_api(&common, &["config", "patch", "--input", "-"]);
    assert!(matches!(refused.command, ApiCommands::Config { .. }));
    let error = run_with_io(
        refused,
        &mut Cursor::new(br#"{"model":"o3"}"#),
        &mut Vec::new(),
    )
    .await
    .expect_err("config writes must require confirmation");
    let error = error.to_string();
    assert!(error.contains("--yes"));
    assert!(error.contains("/tmp/workspace"));
    assert!(error.contains("platform-a/company-b"));
    assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));

    run_ok(&common, &["config", "get"], b"").await;
    assert_request(
        &requests.recv().expect("directory get request"),
        "GET",
        "/v1/config?directory=%2Ftmp%2Fworkspace",
        Value::Null,
    );

    run_ok_with_input(
        &common,
        &["--yes", "config", "patch", "--input", "-"],
        br#"{"model":"o3","nested":{"enabled":true}}"#,
    )
    .await;
    assert_request(
        &requests.recv().expect("directory patch request"),
        "PATCH",
        "/v1/config?directory=%2Ftmp%2Fworkspace",
        json!({"model": "o3", "nested": {"enabled": true}}),
    );

    run_ok(&common, &["config", "global", "get"], b"").await;
    assert_request(
        &requests.recv().expect("global get request"),
        "GET",
        "/v1/global/config",
        Value::Null,
    );

    let mut patch_file = tempfile::NamedTempFile::new().expect("create global patch file");
    patch_file.write_all(br#"{"default_model":"o4-mini"}"#).expect("write global patch");
    run_ok(
        &common,
        &[
            "--yes",
            "config",
            "global",
            "patch",
            "--input",
            patch_file.path().to_str().expect("utf-8 patch path"),
        ],
        b"",
    )
    .await;
    assert_request(
        &requests.recv().expect("global patch request"),
        "PATCH",
        "/v1/global/config",
        json!({"default_model": "o4-mini"}),
    );

    run_ok(&common, &["config", "acp"], b"").await;
    assert_request(
        &requests.recv().expect("ACP config request"),
        "GET",
        "/v1/global/config/acp",
        Value::Null,
    );

    run_ok(&common, &["config", "dashboard", "get"], b"").await;
    assert_request(
        &requests.recv().expect("dashboard get request"),
        "GET",
        "/v1/dashboard/config",
        Value::Null,
    );

    run_ok_with_input(
        &common,
        &["--yes", "config", "dashboard", "put", "--input", "-"],
        br#"{"default_model":"o3","gateway":{"auth_enabled":true}}"#,
    )
    .await;
    assert_request(
        &requests.recv().expect("dashboard put request"),
        "PUT",
        "/v1/dashboard/config",
        json!({"default_model": "o3", "gateway": {"auth_enabled": true}}),
    );

    run_ok(&common, &["config", "providers"], b"").await;
    assert_request(
        &requests.recv().expect("provider catalog request"),
        "GET",
        "/v1/config/providers?directory=%2Ftmp%2Fworkspace",
        Value::Null,
    );

    let invalid = parse_api(
        &common,
        &["--yes", "config", "patch", "--input", "-"],
    );
    let error = run_with_io(
        invalid,
        &mut Cursor::new(br#"{"invalid":true}"#),
        &mut Vec::new(),
    )
    .await
    .expect_err("gateway validation failure should propagate")
    .to_string();
    assert!(error.contains("422"));
    assert!(error.contains("invalid config"));
    assert_request(
        &requests.recv().expect("invalid patch request"),
        "PATCH",
        "/v1/config?directory=%2Ftmp%2Fworkspace",
        json!({"invalid": true}),
    );
}

async fn run_ok(common: &[&str], command: &[&str], input: &[u8]) {
    let args = parse_api(common, command);
    run_with_io(args, &mut Cursor::new(input), &mut Vec::new())
        .await
        .unwrap_or_else(|error| panic!("config command should succeed: {error}"));
}

async fn run_ok_with_input(common: &[&str], command: &[&str], input: &[u8]) {
    run_ok(common, command, input).await;
}

fn parse_api(common: &[&str], command: &[&str]) -> super::ApiArgs {
    let (yes, command) = if command.first() == Some(&"--yes") {
        (Some("--yes"), &command[1..])
    } else {
        (None, command)
    };
    let (family, command) = command.split_first().expect("API command family");
    let args = std::iter::once("shiju")
        .chain(std::iter::once(*family))
        .chain(common.iter().copied())
        .chain(yes)
        .chain(command.iter().copied())
        .collect::<Vec<_>>();
    let cli = Cli::try_parse_from(args).expect("remote config command should parse");
    cli.command.into_api_args().expect("expected API command")
}

#[derive(Debug)]
struct RecordedRequest {
    method: String,
    path: String,
    headers: String,
    body: Value,
}

fn assert_request(request: &RecordedRequest, method: &str, path: &str, body: Value) {
    assert_eq!(request.method, method);
    assert_eq!(request.path, path);
    assert_eq!(request.body, body);
    let headers = request.headers.to_ascii_lowercase();
    assert!(headers.contains("\r\nx-platform-id: platform-a\r\n"));
    assert!(headers.contains("\r\nx-company-id: company-b\r\n"));
}

fn spawn_config_server(
    responses: Vec<(u16, Value)>,
) -> (u16, mpsc::Receiver<RecordedRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind config test server");
    let port = listener.local_addr().expect("config server address").port();
    let (request_tx, request_rx) = mpsc::channel();

    std::thread::spawn(move || {
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().expect("accept config request");
            let request = read_request(&mut stream);
            request_tx.send(request).expect("send config request");
            let body = body.to_string();
            let reason = if status == 200 { "OK" } else { "Unprocessable Entity" };
            write!(
                stream,
                "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            )
            .expect("write config response");
        }
    });

    (port, request_rx)
}

fn read_request(stream: &mut std::net::TcpStream) -> RecordedRequest {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 1024];
    let header_end = loop {
        if let Some(position) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break position;
        }
        let read = stream.read(&mut chunk).expect("read config request");
        assert!(read > 0, "connection closed before config request headers");
        bytes.extend_from_slice(&chunk[..read]);
    };
    let headers = String::from_utf8(bytes[..header_end].to_vec()).expect("UTF-8 headers");
    let content_length = headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = bytes[(header_end + 4)..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk).expect("read config request body");
        assert!(read > 0, "connection closed before config request body");
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);
    let mut request_line = headers.lines().next().expect("request line").split_whitespace();
    let method = request_line.next().expect("request method").to_string();
    let path = request_line.next().expect("request path").to_string();
    let body = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body).expect("JSON request body")
    };
    RecordedRequest { method, path, headers, body }
}
