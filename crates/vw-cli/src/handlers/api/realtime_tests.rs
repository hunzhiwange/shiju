//! `realtime` 推理、SSE、stop 与 WebSocket 协议测试。

use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, TryRecvError};

use clap::Parser;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

use super::run_with_io;
use crate::cli::Cli;

#[tokio::test(flavor = "current_thread")]
async fn api_realtime_commands_cover_inference_sse_stop_and_websocket_protocols() {
    let chat_sse = [
        "id: 1\ndata: {\"type\":\"chat.delta\",\"delta\":\"hi\"}\n\n",
        "id: 2\ndata: {\"type\":\"chat.done\",\"finish_reason\":\"stop\"}\n\n",
    ]
    .join("");
    let plan_sse = "id: 3\ndata: {\"type\":\"finished\",\"task_id\":\"task-1\",\"plan_dir\":\"/plan\",\"status\":\"completed\",\"output\":\"done\"}\n\n";
    let responses = vec![
        json!({"response":"agent reply"}).to_string(),
        json!({"id":"chat-1","choices":[]}).to_string(),
        sse(json!({"type":"gateway"})),
        sse(json!({"type":"global"})),
        sse(json!({"type":"instance"})),
        sse(json!({"type":"scheduler"})),
        chat_sse,
        json!({"cancelled":true}).to_string(),
        plan_sse.to_string(),
        json!({"cancelled":false}).to_string(),
    ];
    let (port, requests) = spawn_http_server(responses);
    let port = port.to_string();
    let common = ["--host", "127.0.0.1", "--port", port.as_str()];
    let agent = br#"{"message":"hello"}"#;
    let openai = br#"{"model":"auto","messages":[{"role":"user","content":"hello"}]}"#;
    let chat = br#"{"messages":[{"role":"user","content":"hello"}]}"#;
    let plan = br#"{"query":"make plan","root":"/workspace"}"#;

    let dangerous = [
        Case::new(&["realtime", "agent", "--input", "-"], agent),
        Case::new(&["realtime", "openai", "--input", "-"], openai),
        Case::new(&["realtime", "chat-stream", "--request-id", "chat/one", "--input", "-"], chat),
        Case::new(&["realtime", "chat-stop", "chat/one"], b""),
        Case::new(&["realtime", "plan-stream", "--request-id", "plan/one", "--input", "-"], plan),
        Case::new(&["realtime", "plan-stop", "plan/one"], b""),
        Case::new(
            &["realtime", "websocket", "--input", "-"],
            br#"[{"type":"message","content":"hello"}]"#,
        ),
    ];
    for case in &dangerous {
        let error = run_command(&common, &case.command, &case.input)
            .await
            .expect_err("realtime mutation must require --yes");
        assert!(error.contains("--yes"));
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }

    run_command(&common, &["--yes", "realtime", "agent", "--input", "-"], agent)
        .await
        .expect("agent");
    run_command(&common, &["--yes", "realtime", "openai", "--input", "-"], openai)
        .await
        .expect("openai");
    for command in [
        vec!["realtime", "gateway-events", "--limit", "1"],
        vec!["realtime", "global-events", "--limit", "1"],
        vec!["realtime", "instance-events", "--limit", "1"],
        vec!["realtime", "scheduler-events", "project/one", "--after-cursor", "6", "--limit", "1"],
    ] {
        let output = run_command(&common, &command, b"").await.expect("event stream");
        let lines = jsonl(&output);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1]["status"], "caller_stopped");
    }
    let chat_output = run_command(
        &common,
        &["--yes", "realtime", "chat-stream", "--request-id", "chat/one", "--input", "-"],
        chat,
    )
    .await
    .expect("chat stream");
    assert_eq!(jsonl(&chat_output).last().unwrap()["status"], "terminated");
    run_command(&common, &["--yes", "realtime", "chat-stop", "chat/one"], b"")
        .await
        .expect("chat stop");
    let plan_output = run_command(
        &common,
        &["--yes", "realtime", "plan-stream", "--request-id", "plan/one", "--input", "-"],
        plan,
    )
    .await
    .expect("plan stream");
    assert_eq!(jsonl(&plan_output).last().unwrap()["status"], "terminated");
    run_command(&common, &["--yes", "realtime", "plan-stop", "plan/one"], b"")
        .await
        .expect("plan stop");

    let recorded = (0..10).map(|_| requests.recv().expect("request")).collect::<Vec<_>>();
    assert_eq!(recorded[0].path, "/v1/agent");
    assert_eq!(recorded[1].path, "/v1/chat/completions");
    assert_eq!(recorded[2].path, "/v1/events");
    assert_eq!(recorded[3].path, "/v1/global/event");
    assert_eq!(recorded[4].path, "/v1/event");
    assert_eq!(
        recorded[5].path,
        "/v1/agent-scheduler/projects/project%2Fone/events/stream?after_cursor=6"
    );
    assert_eq!(recorded[6].path, "/v1/chat/stream");
    assert_eq!(recorded[6].request_id.as_deref(), Some("chat/one"));
    assert_eq!(recorded[7].path, "/v1/chat/stream/chat%2Fone/stop");
    assert_eq!(recorded[8].path, "/v1/task-plan-run/stream");
    assert_eq!(recorded[8].request_id.as_deref(), Some("plan/one"));
    assert_eq!(recorded[9].path, "/v1/task-plan-run/stream/plan%2Fone/stop");
    assert!(recorded.iter().all(|request| request.platform.as_deref() == Some("local")));

    let (ws_port, handshake) = spawn_websocket_server();
    let ws_port = ws_port.to_string();
    let ws_common = ["--host", "127.0.0.1", "--port", ws_port.as_str()];
    let ws_output = run_command(
        &ws_common,
        &["--yes", "realtime", "websocket", "--input", "-"],
        br#"[{"type":"message","content":"hello"}]"#,
    )
    .await
    .expect("WebSocket chat");
    let events = jsonl(&ws_output);
    assert_eq!(events[0]["data"]["type"], "done");
    assert_eq!(events[1]["type"], "closed");
    let handshake = handshake.recv().expect("WebSocket handshake");
    assert_eq!(handshake.0, "/v1/ws/chat");
    assert!(handshake.1.contains("vibewindow.v1"));
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

fn jsonl(output: &[u8]) -> Vec<Value> {
    String::from_utf8(output.to_vec())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[derive(Debug)]
struct RecordedRequest {
    path: String,
    request_id: Option<String>,
    platform: Option<String>,
}

fn spawn_http_server(responses: Vec<String>) -> (u16, mpsc::Receiver<RecordedRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind realtime server");
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for body in responses {
            let (mut stream, _) = listener.accept().unwrap();
            tx.send(read_request(&mut stream)).unwrap();
            write!(stream, "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
    });
    (port, rx)
}

fn read_request(stream: &mut std::net::TcpStream) -> RecordedRequest {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        if bytes.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        let n = stream.read(&mut chunk).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&chunk[..n]);
    }
    let headers = String::from_utf8(bytes).unwrap();
    let first = headers.lines().next().unwrap();
    let path = first.split_whitespace().nth(1).unwrap().to_string();
    let header = |name: &str| {
        headers
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.trim().to_string())
    };
    RecordedRequest {
        path,
        request_id: header("x-vw-stream-request-id"),
        platform: header("x-platform-id"),
    }
}

fn spawn_websocket_server() -> (u16, mpsc::Receiver<(String, String)>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let callback = move |request: &Request, mut response: Response| {
            tx.send((
                request.uri().to_string(),
                request
                    .headers()
                    .get("sec-websocket-protocol")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
            ))
            .unwrap();
            response
                .headers_mut()
                .insert("sec-websocket-protocol", "vibewindow.v1".parse().unwrap());
            Ok(response)
        };
        let mut socket = tokio_tungstenite::tungstenite::accept_hdr(stream, callback).unwrap();
        assert!(matches!(socket.read().unwrap(), Message::Text(_)));
        socket
            .send(Message::Text(json!({"type":"done","full_response":"ok"}).to_string().into()))
            .unwrap();
        socket.close(None).unwrap();
    });
    (port, rx)
}

fn sse(value: Value) -> String {
    format!("id: 1\nevent: update\ndata: {value}\n\n")
}
