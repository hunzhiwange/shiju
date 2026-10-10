//! `project` 项目与 worktree 生命周期测试。

use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, TryRecvError};

use clap::Parser;
use serde_json::{Value, json};

use super::run_with_io;
use crate::cli::Cli;

#[tokio::test(flavor = "current_thread")]
async fn api_project_commands_cover_project_and_worktree_lifecycle() {
    let responses = vec![
        (200, json!({"items":[]})),
        (200, json!({"project":project_json()})),
        (200, json!({"project":project_json()})),
        (200, json!({"project":project_json()})),
        (200, json!({"items":[{"path":"src/main.rs","patch":"@@"}]})),
        (200, json!({"items":[]})),
        (201, json!({"worktree":worktree_json()})),
        (200, json!({"worktree":worktree_json()})),
        (200, json!({"ok":true})),
        (200, json!({"ok":true,"message":"reset"})),
    ];
    let (port, requests) = spawn_server(responses);
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
        "/tmp/project root",
    ];

    let dangerous = [
        Case::new(
            &["project", "update", "project /one", "--input", "-"],
            br#"{"name":"renamed"}"#,
            "project /one",
        ),
        Case::new(
            &["project", "worktree", "create", "project /one", "--input", "-"],
            br#"{"name":"feature","branch":"feature /one","checkout":true}"#,
            "project /one",
        ),
        Case::new(
            &["project", "worktree", "delete", "project /one", "worktree /one", "--input", "-"],
            br#"{"force":true}"#,
            "worktree /one",
        ),
        Case::new(
            &["project", "worktree", "reset", "project /one", "worktree /one", "--input", "-"],
            br#"{"mode":"hard","target_ref":"origin/main"}"#,
            "worktree /one",
        ),
    ];
    for case in &dangerous {
        let error = run_command(&common, &case.command, &case.input)
            .await
            .expect_err("project mutation must require --yes");
        assert!(error.contains("--yes"));
        assert!(error.contains(case.target), "confirmation omitted target: {error}");
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }

    let cases = [
        RequestCase::new(
            &[
                "project",
                "list",
                "--cursor",
                "next /one",
                "--limit",
                "7",
                "--query",
                "demo /one",
                "--status",
                "indexing",
            ],
            b"",
            "GET",
            "/v1/projects?cursor=next+%2Fone&limit=7&query=demo+%2Fone&status=indexing",
            Value::Null,
        ),
        RequestCase::new(
            &["project", "resolve", "--input", "-"],
            br#"{"directory":"/tmp/project root","create_if_missing":false}"#,
            "POST",
            "/v1/projects/resolve",
            json!({"directory":"/tmp/project root","create_if_missing":false}),
        ),
        RequestCase::new(
            &["project", "detail", "project /one"],
            b"",
            "GET",
            "/v1/projects/project%20%2Fone",
            Value::Null,
        ),
        RequestCase::new(
            &["--yes", "project", "update", "project /one", "--input", "-"],
            dangerous[0].input.as_slice(),
            "PATCH",
            "/v1/projects/project%20%2Fone",
            json!({"name":"renamed"}),
        ),
        RequestCase::new(
            &["project", "change-records", "project /one"],
            b"",
            "GET",
            "/v1/projects/project%20%2Fone/change-records?directory=%2Ftmp%2Fproject+root",
            Value::Null,
        ),
        RequestCase::new(
            &["project", "worktree", "list", "project /one"],
            b"",
            "GET",
            "/v1/projects/project%20%2Fone/worktrees",
            Value::Null,
        ),
        RequestCase::new(
            &["--yes", "project", "worktree", "create", "project /one", "--input", "-"],
            dangerous[1].input.as_slice(),
            "POST",
            "/v1/projects/project%20%2Fone/worktrees",
            json!({"name":"feature","branch":"feature /one","checkout":true}),
        ),
        RequestCase::new(
            &["project", "worktree", "detail", "project /one", "worktree /one"],
            b"",
            "GET",
            "/v1/projects/project%20%2Fone/worktrees/worktree%20%2Fone",
            Value::Null,
        ),
        RequestCase::new(
            &[
                "--yes",
                "project",
                "worktree",
                "delete",
                "project /one",
                "worktree /one",
                "--input",
                "-",
            ],
            dangerous[2].input.as_slice(),
            "DELETE",
            "/v1/projects/project%20%2Fone/worktrees/worktree%20%2Fone",
            json!({"force":true}),
        ),
        RequestCase::new(
            &[
                "--yes",
                "project",
                "worktree",
                "reset",
                "project /one",
                "worktree /one",
                "--input",
                "-",
            ],
            dangerous[3].input.as_slice(),
            "POST",
            "/v1/projects/project%20%2Fone/worktrees/worktree%20%2Fone/reset",
            json!({"mode":"hard","target_ref":"origin/main"}),
        ),
    ];
    for case in cases {
        run_command(&common, &case.command, &case.input)
            .await
            .unwrap_or_else(|error| panic!("project command failed: {error}"));
        assert_request(&requests.recv().expect("project request"), &case);
    }
}

fn project_json() -> Value {
    json!({
        "id":"project /one","name":"demo","directory":"/tmp/project root",
        "display_path":"~/project","status":"ready","created_at_ms":1,"updated_at_ms":2,
        "git":{"is_repo":true,"has_uncommitted_changes":false}
    })
}

fn worktree_json() -> Value {
    json!({
        "id":"worktree /one","project_id":"project /one","name":"feature",
        "branch":"feature /one","directory":"/tmp/worktree","status":"ready",
        "created_at_ms":1,"updated_at_ms":2
    })
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
    let (yes, command) = if command.first().is_some_and(|part| part.as_ref() == "--yes") {
        (Some("--yes"), &command[1..])
    } else {
        (None, command)
    };
    let (family, command) = command.split_first().ok_or("API command family is required")?;
    let args = std::iter::once("shiju")
        .chain(std::iter::once(family.as_ref()))
        .chain(common.iter().copied())
        .chain(yes)
        .chain(command.iter().map(AsRef::as_ref))
        .collect::<Vec<_>>();
    let cli = Cli::try_parse_from(args).map_err(|error| error.to_string())?;
    let api_args = cli.command.into_api_args().ok_or("expected API command")?;
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

fn spawn_server(responses: Vec<(u16, Value)>) -> (u16, mpsc::Receiver<RecordedRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind project server");
    let port = listener.local_addr().expect("project server address").port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for (status, response) in responses {
            let (mut stream, _) = listener.accept().expect("accept project request");
            tx.send(read_request(&mut stream)).expect("record project request");
            let response = response.to_string();
            write!(stream, "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}", response.len())
            .expect("write project response");
        }
    });
    (port, rx)
}

fn read_request(stream: &mut std::net::TcpStream) -> RecordedRequest {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 1024];
    let header_end = loop {
        if let Some(position) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break position;
        }
        let read = stream.read(&mut chunk).expect("read project request");
        assert!(read > 0);
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
        let read = stream.read(&mut chunk).expect("read project body");
        assert!(read > 0);
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);
    let mut request_line = headers.lines().next().expect("request line").split_whitespace();
    let method = request_line.next().expect("method").to_string();
    let path = request_line.next().expect("path").to_string();
    let body = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body).expect("JSON body")
    };
    RecordedRequest { method, path, headers, body }
}
