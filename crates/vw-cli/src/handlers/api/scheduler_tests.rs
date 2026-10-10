//! `scheduler` 路由、项目作用域与确认测试。

use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, TryRecvError};

use clap::Parser;
use serde_json::{Value, json};

use super::run_with_io;
use crate::cli::Cli;

#[tokio::test(flavor = "current_thread")]
async fn api_scheduler_commands_cover_project_operations_and_scope_checks() {
    let responses = scheduler_responses();
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
    ];
    let project = "project/one";
    let create = br#"{"task_id":"task/a","title":"Task A","kind":"finite","spec_uri":"spec.md","meta_uri":"meta.json","created_at_ms":1}"#;
    let command = br#"{"task_id":"task/a","command":"start"}"#;
    let bulk = br#"{"commands":[]}"#;
    let metadata = br#"{"patch":{"title":"Renamed"}}"#;
    let import = br#"{"tasks":[]}"#;
    let wake = br#"{"now_ms":20}"#;
    let drain = br#"{"now_ms":21,"limit":10}"#;

    let dangerous = [
        Case::new(&["scheduler", "create", project, "--input", "-"], create, "", Value::Null),
        Case::new(&["scheduler", "command", project, "--input", "-"], command, "", Value::Null),
        Case::new(&["scheduler", "bulk", project, "--input", "-"], bulk, "", Value::Null),
        Case::new(
            &["scheduler", "metadata", project, "task/a", "--input", "-"],
            metadata,
            "",
            Value::Null,
        ),
        Case::new(&["scheduler", "import", project, "--input", "-"], import, "", Value::Null),
        Case::new(&["scheduler", "wake", project, "--input", "-"], wake, "", Value::Null),
        Case::new(&["scheduler", "drain", project, "--input", "-"], drain, "", Value::Null),
    ];
    for case in &dangerous {
        let error = run_command(&common, &case.command, &case.input)
            .await
            .expect_err("scheduler mutation must require --yes");
        assert!(error.contains("--yes"), "unexpected confirmation error: {error}");
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }

    let base = "/v1/agent-scheduler/projects/project%2Fone";
    let cases = [
        Case::new(
            &["scheduler", "workflow", project],
            b"",
            &format!("{base}/workflow"),
            Value::Null,
        ),
        Case::new(&["scheduler", "tasks", project], b"", &format!("{base}/tasks"), Value::Null),
        Case::new(
            &["--yes", "scheduler", "create", project, "--input", "-"],
            create,
            &format!("{base}/tasks"),
            json!({"task_id":"task/a","title":"Task A","kind":"finite","spec_uri":"spec.md","meta_uri":"meta.json","metadata":null,"created_at_ms":1}),
        ),
        Case::new(
            &["--yes", "scheduler", "command", project, "--input", "-"],
            command,
            &format!("{base}/commands"),
            json_body(command),
        ),
        Case::new(
            &["--yes", "scheduler", "bulk", project, "--input", "-"],
            bulk,
            &format!("{base}/commands/bulk"),
            json!({"commands":[],"stop_on_error":false}),
        ),
        Case::new(
            &["--yes", "scheduler", "metadata", project, "task/a", "--input", "-"],
            metadata,
            &format!("{base}/tasks/task%2Fa/metadata"),
            json_body(metadata),
        ),
        Case::new(
            &["--yes", "scheduler", "import", project, "--input", "-"],
            import,
            &format!("{base}/legacy-import"),
            json!({"tasks":[],"dry_run":false}),
        ),
        Case::new(
            &["scheduler", "logs", project, "task/a"],
            b"",
            &format!("{base}/tasks/task%2Fa/logs"),
            Value::Null,
        ),
        Case::new(
            &["scheduler", "artifacts", project, "task/a"],
            b"",
            &format!("{base}/tasks/task%2Fa/artifacts"),
            Value::Null,
        ),
        Case::new(
            &["scheduler", "events", project, "--after-cursor", "6"],
            b"",
            &format!("{base}/events?after_cursor=6"),
            Value::Null,
        ),
        Case::new(
            &["--yes", "scheduler", "wake", project, "--input", "-"],
            wake,
            &format!("{base}/wake"),
            json_body(wake),
        ),
        Case::new(
            &["--yes", "scheduler", "drain", project, "--input", "-"],
            drain,
            &format!("{base}/outbox/drain"),
            json_body(drain),
        ),
    ];
    for case in cases {
        run_command(&common, &case.command, &case.input)
            .await
            .unwrap_or_else(|error| panic!("scheduler command failed: {error}"));
        let request = requests.recv().expect("scheduler request");
        assert_eq!(request.path, case.path);
        assert_eq!(request.body, case.body);
        let expected_method = if case.body.is_null() {
            "GET"
        } else if case.path.ends_with("/metadata") {
            "PATCH"
        } else {
            "POST"
        };
        assert_eq!(request.method, expected_method);
        assert!(request.headers.contains("\r\nx-platform-id: platform-a\r\n"));
        assert!(request.headers.contains("\r\nx-company-id: company-b\r\n"));
    }
}

fn scheduler_responses() -> Vec<Value> {
    let task = json!({
        "project_id":"project/one", "task_id":"task/a", "priority":0, "order_no":0,
        "model":"auto", "prompt":"prompt", "retry_count":0, "updated_at_ms":1
    });
    vec![
        json!({"project_id":"project/one","workflow":null}),
        json!({"project_id":"project/one","tasks":[]}),
        json!({"task_id":"task/a","task":{"id":"task/a","title":"Task A","kind":"finite","status":"queued","created_at_ms":1,"updated_at_ms":1}}),
        json!({"accepted":true}),
        json!({"accepted":0,"rejected":0,"results":[]}),
        json!({"task":task}),
        json!({"imported":0,"skipped":0}),
        json!({"task_id":"task/a","logs":[]}),
        json!({"task_id":"task/a","artifacts":[]}),
        json!({"cursor":6,"events":[]}),
        json!({"dispatched":false}),
        json!({"claimed":0,"messages":[]}),
    ]
}

struct Case {
    command: Vec<String>,
    input: Vec<u8>,
    path: String,
    body: Value,
}

impl Case {
    fn new(command: &[&str], input: impl AsRef<[u8]>, path: &str, body: Value) -> Self {
        Self {
            command: command.iter().map(|value| (*value).to_string()).collect(),
            input: input.as_ref().to_vec(),
            path: path.to_string(),
            body,
        }
    }
}

fn json_body(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("test JSON")
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

fn spawn_server(responses: Vec<Value>) -> (u16, mpsc::Receiver<RecordedRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind scheduler server");
    let port = listener.local_addr().expect("address").port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().expect("accept");
            tx.send(read_request(&mut stream)).expect("record");
            let response = response.to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}",
                response.len()
            )
            .expect("respond");
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
        let read = stream.read(&mut chunk).expect("read request");
        assert!(read > 0);
        bytes.extend_from_slice(&chunk[..read]);
    };
    let headers = String::from_utf8(bytes[..header_end].to_vec()).expect("headers");
    let content_length = headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = bytes[(header_end + 4)..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk).expect("read body");
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
        serde_json::from_slice(&body).expect("request JSON")
    };
    RecordedRequest { method, path, headers: format!("\r\n{headers}\r\n"), body }
}
