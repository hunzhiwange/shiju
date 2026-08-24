//! `workflow` CRUD、恢复与 SSE JSONL 输出测试。

use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, TryRecvError};

use clap::Parser;
use serde_json::{Value, json};

use super::run_with_io;
use crate::cli::Cli;

#[tokio::test(flavor = "current_thread")]
async fn api_workflow_commands_cover_application_crud_resume_and_stream() {
    let record = json!({
        "uuid":"wf/one","name":"Deploy","description":"Ship","workflow_yaml":"steps: []",
        "created_at_ms":1,"updated_at_ms":2
    });
    let responses = vec![
        Response::json(json!([{"uuid":"wf/one","name":"Deploy","description":"Ship","created_at_ms":1,"updated_at_ms":2}])),
        Response::json(record.clone()),
        Response::json(json!({"run_id":"run-1","status":"succeeded"})),
        Response::json(record.clone()),
        Response::json(record),
        Response::json(json!({"uuid":"wf/one","deleted":true})),
        Response::sse([
            sse(json!({"event":"workflow_started","task_id":"run-1","workflow_run_id":"run-1","data":{}})),
            sse(json!({"event":"workflow_finished","task_id":"run-1","workflow_run_id":"run-1","data":{"status":"succeeded"}})),
        ].join("")),
        Response::sse([
            sse(json!({"event":"text_chunk","task_id":"run-2","workflow_run_id":"run-2","data":{"text":"partial"}})),
            sse(json!({"event":"error","task_id":"run-2","status":422,"code":"bad_node","message":"node failed"})),
        ].join("")),
    ];
    let (port, requests) = spawn_server(responses);
    let port = port.to_string();
    let common = ["--host", "127.0.0.1", "--port", port.as_str()];
    let upsert = br#"{"name":"Deploy","description":"Ship","workflow_yaml":"steps: []"}"#;
    let resume = br#"{"run_id":"run-1","form_token":"token-1"}"#;
    let default_chat = br#"{"query":"run inline","application_workflow":"steps: []","response_mode":"streaming","user":"user-1"}"#;
    let uuid_chat = br#"{"query":"run saved","response_mode":"streaming","user":"user-1"}"#;

    let dangerous = [
        Case::new(&["workflow", "resume", "--input", "-"], resume),
        Case::new(&["workflow", "create", "--input", "-"], upsert),
        Case::new(&["workflow", "update", "wf/one", "--input", "-"], upsert),
        Case::new(&["workflow", "delete", "wf/one"], b""),
        Case::new(&["workflow", "chat", "--input", "-"], default_chat),
        Case::new(&["workflow", "chat-application", "wf/one", "--input", "-"], uuid_chat),
    ];
    for case in &dangerous {
        let error = run_command(&common, &case.command, &case.input)
            .await
            .expect_err("workflow mutation must require --yes");
        assert!(error.contains("--yes"));
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }

    run_command(&common, &["workflow", "list"], b"").await.expect("list");
    assert_request(&requests, "GET", "/v1/workflow/applications", Value::Null);
    run_command(&common, &["workflow", "detail", "wf/one"], b"").await.expect("detail");
    assert_request(&requests, "GET", "/v1/workflow/applications/wf%2Fone", Value::Null);
    run_command(&common, &["--yes", "workflow", "resume", "--input", "-"], resume)
        .await
        .expect("resume");
    assert_request(
        &requests,
        "POST",
        "/v1/workflow/resume",
        json!({"run_id":"run-1","form_token":"token-1","form_values":{}}),
    );
    run_command(&common, &["--yes", "workflow", "create", "--input", "-"], upsert)
        .await
        .expect("create");
    assert_request(
        &requests,
        "POST",
        "/v1/workflow/applications",
        json!({"name":"Deploy","description":"Ship","workflow_yaml":"steps: []"}),
    );
    run_command(
        &common,
        &["--yes", "workflow", "update", "wf/one", "--input", "-"],
        upsert,
    )
    .await
    .expect("update");
    assert_request(
        &requests,
        "PUT",
        "/v1/workflow/applications/wf%2Fone",
        json!({"name":"Deploy","description":"Ship","workflow_yaml":"steps: []"}),
    );
    run_command(&common, &["--yes", "workflow", "delete", "wf/one"], b"")
        .await
        .expect("delete");
    assert_request(
        &requests,
        "DELETE",
        "/v1/workflow/applications/wf%2Fone",
        json!({}),
    );

    let output = run_command(
        &common,
        &["--yes", "workflow", "chat", "--input", "-"],
        default_chat,
    )
    .await
    .expect("default workflow stream");
    let events = jsonl(&output);
    assert_eq!(events.len(), 2);
    assert!(events[0].get("WorkflowStarted").is_some());
    assert!(events[1].get("WorkflowFinished").is_some());
    assert_request(
        &requests,
        "POST",
        "/v1/workflow/applications/chat-messages",
        json!({"query":"run inline","application_workflow":"steps: []","inputs":{},"response_mode":"streaming","user":"user-1"}),
    );

    let (error, output) = run_command_with_output(
        &common,
        &["--yes", "workflow", "chat-application", "wf/one", "--input", "-"],
        uuid_chat,
    )
    .await;
    assert!(error.expect("stream error").contains("bad_node: node failed"));
    let events = jsonl(&output);
    assert_eq!(events.len(), 2, "stream error must not discard prior or terminal event");
    assert!(events[0].get("TextChunk").is_some());
    assert!(events[1].get("Error").is_some());
    assert_request(
        &requests,
        "POST",
        "/v1/workflow/applications/wf%2Fone/chat-messages",
        json!({"query":"run saved","inputs":{},"response_mode":"streaming","user":"user-1"}),
    );
}

struct Case {
    command: Vec<String>,
    input: Vec<u8>,
}

impl Case {
    fn new(command: &[&str], input: impl AsRef<[u8]>) -> Self {
        Self {
            command: command.iter().map(|value| (*value).to_string()).collect(),
            input: input.as_ref().to_vec(),
        }
    }
}

async fn run_command<S: AsRef<str>>(
    common: &[&str],
    command: &[S],
    input: &[u8],
) -> Result<Vec<u8>, String> {
    let (error, output) = run_command_with_output(common, command, input).await;
    error.map_or(Ok(output), Err)
}

async fn run_command_with_output<S: AsRef<str>>(
    common: &[&str],
    command: &[S],
    input: &[u8],
) -> (Option<String>, Vec<u8>) {
    let (yes, command) = if command.first().is_some_and(|part| part.as_ref() == "--yes") {
        (Some("--yes"), &command[1..])
    } else {
        (None, command)
    };
    let (family, command) = command.split_first().expect("API command family");
    let args = std::iter::once("shiju")
        .chain(std::iter::once(family.as_ref()))
        .chain(common.iter().copied())
        .chain(yes)
        .chain(command.iter().map(AsRef::as_ref))
        .collect::<Vec<_>>();
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error) => return (Some(error.to_string()), Vec::new()),
    };
    let Some(api_args) = cli.command.into_api_args() else {
        return (Some("expected API command".into()), Vec::new());
    };
    let mut output = Vec::new();
    let error = run_with_io(api_args, &mut Cursor::new(input), &mut output)
        .await
        .err()
        .map(|error| error.to_string());
    (error, output)
}

fn jsonl(output: &[u8]) -> Vec<Value> {
    String::from_utf8(output.to_vec())
        .expect("UTF-8 output")
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSONL event"))
        .collect()
}

#[derive(Debug)]
struct RecordedRequest {
    method: String,
    path: String,
    body: Value,
}

fn assert_request(
    requests: &mpsc::Receiver<RecordedRequest>,
    method: &str,
    path: &str,
    body: Value,
) {
    let request = requests.recv().expect("workflow request");
    assert_eq!(request.method, method);
    assert_eq!(request.path, path);
    assert_eq!(request.body, body);
}

struct Response {
    content_type: &'static str,
    body: String,
}

impl Response {
    fn json(value: Value) -> Self {
        Self { content_type: "application/json", body: value.to_string() }
    }

    fn sse(body: String) -> Self {
        Self { content_type: "text/event-stream", body }
    }
}

fn spawn_server(responses: Vec<Response>) -> (u16, mpsc::Receiver<RecordedRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind workflow server");
    let port = listener.local_addr().expect("address").port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().expect("accept");
            tx.send(read_request(&mut stream)).expect("record");
            write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                response.content_type,
                response.body.len(),
                response.body
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
    RecordedRequest { method, path, body }
}

fn sse(payload: Value) -> String {
    format!("data: {payload}\n\n")
}
