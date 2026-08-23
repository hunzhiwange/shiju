//! `api workspace` 文件、搜索与 Git 路由及确认测试。

use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, TryRecvError};

use clap::Parser;
use serde_json::{Value, json};

use super::run_with_io;
use crate::cli::{Cli, Commands};

#[tokio::test(flavor = "current_thread")]
async fn api_workspace_commands_preserve_paths_and_require_confirmation() {
    let responses = vec![
        (
            200,
            json!([{"path":"src/main.rs","lineNumber":7,"line":"needle/value","start":0,"end":6}]),
        ),
        (200, json!(["src/main.rs"])),
        (
            200,
            json!([{"name":"main.rs","path":"src/main.rs","absolute":"/repo/src/main.rs","type":"file","ignored":false}]),
        ),
        (200, json!({"type":"text","content":"fn main() {}","encoding":"utf-8"})),
        (200, json!([{"path":"src/main.rs","added":2,"removed":1,"status":"modified"}])),
        (
            200,
            json!({"root_directory":"/tmp/workspace root","path":"notes /todo.md","content":"hello"}),
        ),
        (
            200,
            json!({"ok":true,"root_directory":"/tmp/workspace root","path":"notes /todo.md","bytes_written":5}),
        ),
        (200, json!({"ok":true})),
        (200, json!({"ok":true})),
        (200, json!({"ok":true})),
        (200, json!({"ok":true,"created":true,"commit":{"sha":"abc123","message":"ship it"}})),
        (200, json!({"success":true,"code":0,"stdout":"clean","stderr":""})),
        (
            200,
            json!({"ok":true,"source_branch":"feature /one","target_branch":"main","workspace":"/repo","already_merged":false,"message":"merged"}),
        ),
        (403, json!({"message":"workspace denied"})),
    ];
    let (port, requests) = spawn_workspace_server(responses);
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
        "/tmp/workspace root",
    ];

    let dangerous = [
        Case::new(
            &["workspace", "write", "--input", "-"],
            br#"{"path":"notes /todo.md","content":"hello","create_if_missing":true}"#,
            "notes /todo.md",
        ),
        Case::new(
            &["workspace", "move", "--input", "-"],
            br#"{"project_id":"project /one","from_path":"old /name","to_path":"new /name","overwrite":true}"#,
            "old /name",
        ),
        Case::new(
            &["workspace", "copy", "--input", "-"],
            br#"{"project_id":"project /one","from_path":"src /one","to_path":"dst /one","overwrite":false}"#,
            "dst /one",
        ),
        Case::new(
            &["workspace", "delete", "--input", "-"],
            br#"{"project_id":"project /one","path":"trash /one","recursive":true}"#,
            "trash /one",
        ),
        Case::new(
            &["workspace", "git", "commit", "--input", "-"],
            br#"{"project_id":"project /one","message":"ship it","stage_all":true}"#,
            "project /one",
        ),
        Case::new(
            &["workspace", "git", "command", "--input", "-"],
            br#"{"directory":"/repo with spaces","args":["status","--short"],"timeout_secs":5}"#,
            "/repo with spaces",
        ),
        Case::new(
            &["workspace", "git", "merge", "--input", "-"],
            br#"{"project_id":"project /one","source_branch":"feature /one","target_branch":"main"}"#,
            "feature /one",
        ),
    ];

    for case in &dangerous {
        let error = run_command(&common, &case.command, &case.input)
            .await
            .expect_err("workspace side effect must require --yes");
        assert!(error.contains("--yes"), "unexpected confirmation error: {error}");
        assert!(error.contains(case.target), "confirmation omitted target: {error}");
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }

    let cases = [
        RequestCase::new(
            &["workspace", "find-text", "needle/value"],
            b"",
            "GET",
            "/v1/find?directory=%2Ftmp%2Fworkspace+root&pattern=needle%2Fvalue",
            Value::Null,
        ),
        RequestCase::new(
            &[
                "workspace",
                "find-file",
                "main /rs",
                "--dirs",
                "false",
                "--type",
                "file",
                "--limit",
                "7",
            ],
            b"",
            "GET",
            "/v1/find/file?directory=%2Ftmp%2Fworkspace+root&query=main+%2Frs&dirs=false&type=file&limit=7",
            Value::Null,
        ),
        RequestCase::new(
            &["workspace", "list", "src /nested"],
            b"",
            "GET",
            "/v1/file?directory=%2Ftmp%2Fworkspace+root&path=src+%2Fnested",
            Value::Null,
        ),
        RequestCase::new(
            &["workspace", "content", "src /main.rs"],
            b"",
            "GET",
            "/v1/file/content?directory=%2Ftmp%2Fworkspace+root&path=src+%2Fmain.rs",
            Value::Null,
        ),
        RequestCase::new(
            &["workspace", "status"],
            b"",
            "GET",
            "/v1/file/status?directory=%2Ftmp%2Fworkspace+root",
            Value::Null,
        ),
        RequestCase::new(
            &["workspace", "read", "--input", "-"],
            br#"{"agent_key":"agent /one","path":"notes /todo.md"}"#,
            "POST",
            "/v1/file/read",
            json!({"directory":"/tmp/workspace root","agent_key":"agent /one","path":"notes /todo.md"}),
        ),
        RequestCase::new(
            &["--yes", "workspace", "write", "--input", "-"],
            dangerous[0].input.as_slice(),
            "POST",
            "/v1/file/write",
            json!({"directory":"/tmp/workspace root","path":"notes /todo.md","content":"hello","create_if_missing":true}),
        ),
        RequestCase::new(
            &["--yes", "workspace", "move", "--input", "-"],
            dangerous[1].input.as_slice(),
            "POST",
            "/v1/files/move",
            json!({"project_id":"project /one","from_path":"old /name","to_path":"new /name","overwrite":true}),
        ),
        RequestCase::new(
            &["--yes", "workspace", "copy", "--input", "-"],
            dangerous[2].input.as_slice(),
            "POST",
            "/v1/files/copy",
            json!({"project_id":"project /one","from_path":"src /one","to_path":"dst /one","overwrite":false}),
        ),
        RequestCase::new(
            &["--yes", "workspace", "delete", "--input", "-"],
            dangerous[3].input.as_slice(),
            "POST",
            "/v1/files/delete",
            json!({"project_id":"project /one","path":"trash /one","recursive":true}),
        ),
        RequestCase::new(
            &["--yes", "workspace", "git", "commit", "--input", "-"],
            dangerous[4].input.as_slice(),
            "POST",
            "/v1/git/commit",
            json!({"project_id":"project /one","message":"ship it","stage_all":true,"selected_files":[],"selected_hunks":[],"selected_lines":[],"selected_old_lines":[]}),
        ),
        RequestCase::new(
            &["--yes", "workspace", "git", "command", "--input", "-"],
            dangerous[5].input.as_slice(),
            "POST",
            "/v1/git/command",
            json!({"directory":"/repo with spaces","args":["status","--short"],"timeout_secs":5}),
        ),
        RequestCase::new(
            &["--yes", "workspace", "git", "merge", "--input", "-"],
            dangerous[6].input.as_slice(),
            "POST",
            "/v1/git/merge",
            json!({"project_id":"project /one","source_branch":"feature /one","target_branch":"main"}),
        ),
    ];

    for case in cases {
        run_command(&common, &case.command, &case.input)
            .await
            .unwrap_or_else(|error| panic!("workspace command failed: {error}"));
        assert_request(&requests.recv().expect("workspace request"), &case);
    }

    let error = run_command(&common, &["workspace", "content", "denied /file"], b"")
        .await
        .expect_err("Gateway rejection must propagate");
    assert!(error.contains("403"));
    assert!(error.contains("workspace denied"));
    let denied = requests.recv().expect("denied request");
    assert_eq!(
        denied.path,
        "/v1/file/content?directory=%2Ftmp%2Fworkspace+root&path=denied+%2Ffile"
    );
}

struct Case {
    command: Vec<String>,
    input: Vec<u8>,
    target: &'static str,
}

impl Case {
    fn new(command: &[&str], input: impl AsRef<[u8]>, target: &'static str) -> Self {
        Self {
            command: command.iter().map(|value| (*value).to_string()).collect(),
            input: input.as_ref().to_vec(),
            target,
        }
    }
}

struct RequestCase {
    command: Vec<String>,
    input: Vec<u8>,
    method: &'static str,
    path: &'static str,
    body: Value,
}

impl RequestCase {
    fn new(
        command: &[&str],
        input: impl AsRef<[u8]>,
        method: &'static str,
        path: &'static str,
        body: Value,
    ) -> Self {
        Self {
            command: command.iter().map(|value| (*value).to_string()).collect(),
            input: input.as_ref().to_vec(),
            method,
            path,
            body,
        }
    }
}

async fn run_command<S: AsRef<str>>(
    common: &[&str],
    command: &[S],
    input: &[u8],
) -> Result<Vec<u8>, String> {
    let args = std::iter::once("shiju")
        .chain(std::iter::once("api"))
        .chain(common.iter().copied())
        .chain(command.iter().map(AsRef::as_ref))
        .collect::<Vec<_>>();
    let cli = Cli::try_parse_from(args).map_err(|error| error.to_string())?;
    let Commands::Api { api_args } = cli.command else {
        return Err("expected api command".to_string());
    };
    let mut output = Vec::new();
    run_with_io(api_args, &mut Cursor::new(input), &mut output)
        .await
        .map_err(|error| error.to_string())?;
    Ok(output)
}

#[derive(Debug)]
struct RecordedRequest {
    method: String,
    path: String,
    headers: String,
    body: Value,
}

fn assert_request(request: &RecordedRequest, case: &RequestCase) {
    assert_eq!(request.method, case.method);
    assert_eq!(request.path, case.path);
    assert_eq!(request.body, case.body);
    let headers = request.headers.to_ascii_lowercase();
    assert!(headers.contains("\r\nx-platform-id: platform-a\r\n"));
    assert!(headers.contains("\r\nx-company-id: company-b\r\n"));
}

fn spawn_workspace_server(responses: Vec<(u16, Value)>) -> (u16, mpsc::Receiver<RecordedRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind workspace test server");
    let port = listener.local_addr().expect("workspace server address").port();
    let (request_tx, request_rx) = mpsc::channel();
    std::thread::spawn(move || {
        for (status, response) in responses {
            let (mut stream, _) = listener.accept().expect("accept workspace request");
            request_tx.send(read_request(&mut stream)).expect("send workspace request");
            let response = response.to_string();
            write!(
                stream,
                "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}",
                response.len()
            )
            .expect("write workspace response");
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
        let read = stream.read(&mut chunk).expect("read workspace request");
        assert!(read > 0, "connection closed before workspace request headers");
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
        let read = stream.read(&mut chunk).expect("read workspace request body");
        assert!(read > 0, "connection closed before workspace request body");
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
