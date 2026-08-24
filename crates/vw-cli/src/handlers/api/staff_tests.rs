//! `staff` 层级路由、确认与内部 IPC 排除测试。

use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, TryRecvError};

use clap::Parser;
use serde_json::{Value, json};

use super::run_with_io;
use crate::cli::Cli;

#[tokio::test(flavor = "current_thread")]
async fn api_staff_commands_cover_capability_orchestration_and_projection_routes() {
    let (port, requests) = spawn_server(staff_responses());
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
    let workflow = "workflow/one";
    let capabilities = br#"{"capabilities":[]}"#;
    let project_orchestration =
        br#"{"workflow":{"id":"flow","label":"Flow","statuses":[]}}"#;
    let workflow_orchestration = br#"{"workflow_id":"workflow/one"}"#;
    let command = br#"{"command":{"type":"assign_task","command_id":"cmd-1","task_id":"task/a","member_id":"staff-a"}}"#;
    let plan = br#"{}"#;

    let dangerous = [
        Case::new(
            &["staff", "project-capabilities-save", project, "--input", "-"],
            capabilities,
            "",
            Value::Null,
        ),
        Case::new(
            &["staff", "project-orchestration-save", project, "--input", "-"],
            project_orchestration,
            "",
            Value::Null,
        ),
        Case::new(
            &[
                "staff",
                "workflow-capabilities-save",
                project,
                workflow,
                "--input",
                "-",
            ],
            capabilities,
            "",
            Value::Null,
        ),
        Case::new(
            &[
                "staff",
                "workflow-orchestration-save",
                project,
                workflow,
                "--input",
                "-",
            ],
            workflow_orchestration,
            "",
            Value::Null,
        ),
        Case::new(
            &["staff", "command", project, "--input", "-"],
            command,
            "",
            Value::Null,
        ),
        Case::new(
            &["staff", "auto-plan", project, "--input", "-"],
            plan,
            "",
            Value::Null,
        ),
    ];
    for case in &dangerous {
        let error = run_command(&common, &case.command, &case.input)
            .await
            .expect_err("staff mutation must require --yes");
        assert!(error.contains("--yes"));
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }

    let ipc = run_command(&common, &["staff", "ipc"], b"")
        .await
        .expect_err("internal staff IPC must be excluded");
    assert!(ipc.contains("internal staff process protocol"));
    assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));

    let base = "/v1/staff/projects/project%2Fone";
    let project_state = json!({
        "capabilities": [],
        "stage_assignments": [],
        "stage_workflow_bindings": [],
        "stage_actions": [],
        "member_task_assignments": []
    });
    let workflow_state = json!({
        "capabilities": [],
        "stage_assignments": [],
        "stage_workflow_bindings": [],
        "stage_actions": []
    });
    let cases = [
        Case::new(&["staff", "stage-capabilities"], b"", "/v1/staff/stage-capabilities", Value::Null),
        Case::new(&["staff", "project-capabilities", project], b"", &format!("{base}/capabilities"), Value::Null),
        Case::new(&["--yes", "staff", "project-capabilities-save", project, "--input", "-"], capabilities, &format!("{base}/capabilities"), json!({"capabilities":[]})),
        Case::new(&["staff", "project-orchestration", project], b"", &format!("{base}/orchestration"), Value::Null),
        Case::new(&["--yes", "staff", "project-orchestration-save", project, "--input", "-"], project_orchestration, &format!("{base}/orchestration"), json!({"workflow":{"id":"flow","label":"Flow","statuses":[]},"project_state":project_state})),
        Case::new(&["staff", "workflow-capabilities", project, workflow], b"", &format!("{base}/workflows/workflow%2Fone/capabilities"), Value::Null),
        Case::new(&["--yes", "staff", "workflow-capabilities-save", project, workflow, "--input", "-"], capabilities, &format!("{base}/workflows/workflow%2Fone/capabilities"), json!({"capabilities":[]})),
        Case::new(&["staff", "workflow-orchestration", project, workflow], b"", &format!("{base}/workflows/workflow%2Fone/orchestration"), Value::Null),
        Case::new(&["--yes", "staff", "workflow-orchestration-save", project, workflow, "--input", "-"], workflow_orchestration, &format!("{base}/workflows/workflow%2Fone/orchestration"), json!({"workflow_id":"workflow/one","orchestration":workflow_state})),
        Case::new(&["staff", "projection", project], b"", &format!("{base}/projection"), Value::Null),
        Case::new(&["--yes", "staff", "command", project, "--input", "-"], command, &format!("{base}/commands"), json!({"command":{"type":"assign_task","command_id":"cmd-1","task_id":"task/a","member_id":"staff-a"},"dry_run":false})),
        Case::new(&["--yes", "staff", "auto-plan", project, "--input", "-"], plan, &format!("{base}/auto-schedule-plan"), json!({"running_task_ids":[]})),
    ];
    for case in cases {
        run_command(&common, &case.command, &case.input)
            .await
            .unwrap_or_else(|error| panic!("staff command failed: {error}"));
        let request = requests.recv().expect("staff request");
        assert_eq!(request.path, case.path);
        assert_eq!(request.body, case.body);
        let expected_method = if case.body.is_null() {
            "GET"
        } else if case.path.ends_with("/capabilities")
            || case.path.ends_with("/orchestration")
        {
            "PUT"
        } else {
            "POST"
        };
        assert_eq!(request.method, expected_method);
        assert!(request.headers.contains("\r\nx-platform-id: platform-a\r\n"));
        assert!(request.headers.contains("\r\nx-company-id: company-b\r\n"));
    }
}

fn staff_responses() -> Vec<Value> {
    let project_orchestration = json!({
        "project_id":"project/one",
        "workflow":{"id":"flow","label":"Flow","statuses":[]},
        "project_state":{}
    });
    let workflow_orchestration = json!({
        "workflow_id":"workflow/one",
        "orchestration":{}
    });
    vec![
        json!({"capabilities":[]}),
        json!({"capabilities":[]}),
        json!({"capabilities":[]}),
        project_orchestration.clone(),
        project_orchestration,
        json!({"capabilities":[]}),
        json!({"capabilities":[]}),
        workflow_orchestration.clone(),
        workflow_orchestration,
        json!({"project_id":"project/one","projection":{"lanes":[],"member_loads":[],"unassigned":[],"total_task_count":0,"blocked_task_count":0}}),
        json!({"accepted":true,"message":"validated"}),
        json!({"project_id":"project/one","plan":{"commands":[],"command_contexts":[],"skipped":[]}}),
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

async fn run_command<S: AsRef<str>>(
    common: &[&str],
    command: &[S],
    input: &[u8],
) -> Result<Vec<u8>, String> {
    let (yes, command) = if command.first().is_some_and(|part| part.as_ref() == "--yes") { (Some("--yes"), &command[1..]) } else { (None, command) };
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
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind staff server");
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
