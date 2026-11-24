use std::ffi::OsString;
use std::io::{Cursor, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use clap::Parser;
use serde_json::{Value, json};

use super::run_with_io;
use crate::session_tests::{RecordedRequest, read_request};
use crate::test_cli::{Cli, Commands};

const SESSIONS: &str = "/v1/projects/demo/cli-workbench/sessions";
const SESSION_ID: &str = "session-a";
const REQUEST_ID: &str = "review-01";

#[tokio::test(flavor = "current_thread")]
async fn cli_workbench_command_lists_starts_and_reads_the_same_delivery() {
    let accepted = message("accepted", 1, "");
    let completed = message("completed", 3, "review complete");
    let session = json!({
        "id": SESSION_ID,
        "project_id": "demo",
        "directory": "/demo",
        "target": {"kind": "acp", "agent": "acp-claude"},
        "state": "ready",
        "capabilities": {"available": true, "terminal": false, "delivery": "structured"}
    });
    let (port, requests) = serve([
        Reply::json("200 OK", json!([session])),
        Reply::json("201 Created", session.clone()),
        Reply::json("200 OK", accepted.clone()),
        Reply::json("200 OK", accepted.clone()),
        Reply::json("200 OK", completed.clone()),
        Reply::json("200 OK", completed.clone()),
    ]);

    let (result, list) = run(port, &["list"], b"", None).await;
    result.unwrap();
    assert_eq!(list["result"], json!([session]));
    let request = next_request(&requests);
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, SESSIONS);
    assert!(request.headers.iter().any(|h| h.eq_ignore_ascii_case("x-user-id: fixture-user")));

    let (result, started) =
        run(port, &["start", "acp", "acp-claude", "--yes", "--directory", "/demo"], b"", None)
            .await;
    result.unwrap();
    assert_eq!(started["result"], session);
    let request = next_request(&requests);
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, SESSIONS);
    assert_eq!(request.body, json!({"target": session["target"], "directory": "/demo"}));

    let (result, sent) = run(
        port,
        &["send", SESSION_ID, "--request-id", REQUEST_ID, "--yes"],
        "请检查改动\n".as_bytes(),
        None,
    )
    .await;
    result.unwrap();
    assert_eq!(sent["result"], accepted);
    let request = next_request(&requests);
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, format!("{SESSIONS}/{SESSION_ID}/messages"));
    assert_eq!(request.body, json!({"request_id": REQUEST_ID, "text": "请检查改动\n"}));

    for command in ["wait", "read"] {
        let (result, output) =
            run(port, &[command, SESSION_ID, "--request-id", REQUEST_ID], b"", None).await;
        result.unwrap();
        assert_eq!(output, json!({"ok": true, "result": completed}));
    }
    let path = delivery_path();
    for expected in [path.clone(), format!("{path}/events?after_revision=1"), path] {
        let request = next_request(&requests);
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, expected);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn cli_workbench_command_reports_busy_and_requires_confirmation() {
    let (port, requests) = serve([Reply::json(
        "409 Conflict",
        json!({"code": "busy", "message": "workbench target is busy"}),
    )]);
    let args = ["send", SESSION_ID, "--request-id", REQUEST_ID];
    let (result, output) = run(port, &args, b"review", None).await;
    assert!(result.is_err());
    assert_eq!(output["error"]["code"], "invalid_request");
    assert!(output["error"]["message"].as_str().unwrap().contains("--yes"));
    assert!(matches!(requests.try_recv(), Err(mpsc::TryRecvError::Empty)));

    let args = [args.as_slice(), &["--yes"]].concat();
    let (result, output) = run(port, &args, b"review", None).await;
    assert!(result.is_err());
    assert_eq!(
        output,
        json!({
            "ok": false, "error": {"code": "busy", "message": "workbench target is busy"}
        })
    );
    let request = next_request(&requests);
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, format!("{SESSIONS}/{SESSION_ID}/messages"));
}

#[tokio::test(flavor = "current_thread")]
async fn cli_workbench_command_wait_timeout_preserves_id_for_status() {
    let accepted = message("accepted", 1, "");
    let submitted = message("submitted", 2, "partial");
    let (port, requests) = serve([
        Reply::json("200 OK", accepted.clone()),
        Reply::DisconnectAfter(Duration::from_millis(1250)),
        Reply::json("200 OK", submitted.clone()),
    ]);
    let (result, output) = run(
        port,
        &["wait", SESSION_ID, "--request-id", REQUEST_ID, "--timeout-secs", "1"],
        b"",
        None,
    )
    .await;
    assert!(result.is_err());
    assert_eq!(output["error"]["code"], "timed_out");
    assert_eq!(output["message"], accepted);
    let (result, output) =
        run(port, &["status", SESSION_ID, "--request-id", REQUEST_ID], b"", None).await;
    result.unwrap();
    assert_eq!(output["result"], submitted);
    let path = delivery_path();
    for expected in [path.clone(), format!("{path}/events?after_revision=1"), path] {
        let request = next_request(&requests);
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, expected);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn cli_workbench_command_wait_reports_user_actions() {
    let draft = message("pending_submission", 2, "");
    let mut approval = message("waiting_for_approval", 3, "");
    approval["approval_request_id"] = json!("approval-1");
    let (port, requests) =
        serve([Reply::json("200 OK", draft.clone()), Reply::json("200 OK", approval.clone())]);
    for expected in [draft, approval] {
        let (result, output) =
            run(port, &["wait", SESSION_ID, "--request-id", REQUEST_ID], b"", None).await;
        result.unwrap();
        assert_eq!(output["result"], expected);
        let request = next_request(&requests);
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, delivery_path());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn cli_workbench_command_child_connection_is_project_bound_and_fails_closed() {
    let (port, requests) = serve([Reply::json("200 OK", json!([]))]);
    let context = json!({"port": port, "project_id": "demo", "token": "fixture-token"});
    let child = Some(OsString::from(context.to_string()));

    let (result, output) =
        run(0, &["list", "--project", "another-project"], b"", child.clone()).await;
    assert!(result.is_err());
    assert_eq!(output["error"]["code"], "unauthorized");

    let (result, output) =
        run(0, &["list", "--project", "demo", "--user", "fixture-user"], b"", Some("{}".into()))
            .await;
    assert!(result.is_err());
    assert_eq!(output["error"]["message"], "invalid workbench child context");
    assert!(matches!(requests.try_recv(), Err(mpsc::TryRecvError::Empty)));

    let (result, output) = run(0, &["list"], b"", child).await;
    result.unwrap();
    assert_eq!(output["result"], json!([]));
    let request = next_request(&requests);
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, SESSIONS);
    assert!(
        request
            .headers
            .iter()
            .any(|h| h.eq_ignore_ascii_case("authorization: Bearer fixture-token"))
    );
}

#[tokio::test(flavor = "current_thread")]
async fn cli_workbench_command_external_connection_requires_explicit_user() {
    let cli = Cli::try_parse_from(["shiju", "workbench", "list", "--project", "demo"]).unwrap();
    let Commands::Workbench(args) = cli.command else { panic!("expected workbench command") };
    let mut output = Vec::new();
    assert!(run_with_io(args, &mut std::io::empty(), &mut output, None).await.is_err());
    let output: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(output["error"]["message"], "--user is required");
}

async fn run(
    port: u16,
    command: &[&str],
    input: &[u8],
    child: Option<OsString>,
) -> (anyhow::Result<()>, Value) {
    let port = port.to_string();
    let mut args = vec!["shiju", "workbench", "--port", port.as_str()];
    if child.is_none() {
        args.extend(["--user", "fixture-user", "--project", "demo"]);
    }
    args.extend_from_slice(command);
    let cli = Cli::try_parse_from(args).unwrap();
    let Commands::Workbench(args) = cli.command else { panic!("expected workbench command") };
    let mut output = Vec::new();
    let result = run_with_io(args, &mut Cursor::new(input), &mut output, child).await;
    (result, serde_json::from_slice(&output).unwrap())
}

fn delivery_path() -> String {
    format!("{SESSIONS}/{SESSION_ID}/messages/{REQUEST_ID}")
}

fn message(state: &str, revision: u64, output: &str) -> Value {
    json!({
        "id": {"request_id": REQUEST_ID, "sender_user_id": "fixture-user", "target_session_id": SESSION_ID},
        "turn_id": "turn-a",
        "state": state,
        "revision": revision,
        "approval_request_id": null,
        "approval_history": [],
        "assistant_message_id": if state == "completed" { Some("assistant-a") } else { None },
        "output": output,
        "output_truncated": false,
        "error": null
    })
}

fn next_request(requests: &Receiver<RecordedRequest>) -> RecordedRequest {
    requests.recv_timeout(Duration::from_secs(2)).expect("missing gateway request")
}

enum Reply {
    Json { status: &'static str, body: Value },
    DisconnectAfter(Duration),
}

impl Reply {
    fn json(status: &'static str, body: Value) -> Self {
        Self::Json { status, body }
    }
}

fn serve<const N: usize>(replies: [Reply; N]) -> (u16, Receiver<RecordedRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let (sent, received) = mpsc::channel();
    std::thread::spawn(move || {
        for reply in replies {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            sent.send(read_request(&mut stream)).unwrap();
            match reply {
                Reply::Json { status, body } => {
                    let body = body.to_string();
                    write!(
                        stream,
                        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len(),
                    ).unwrap();
                }
                Reply::DisconnectAfter(duration) => std::thread::sleep(duration),
            }
        }
    });
    (port, received)
}
