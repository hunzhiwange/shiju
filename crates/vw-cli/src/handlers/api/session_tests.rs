//! `session` 核心生命周期路由与确认测试。

use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, TryRecvError};

use clap::Parser;
use serde_json::{Value, json};

use super::run_with_io;
use crate::cli::Cli;

#[tokio::test(flavor = "current_thread")]
async fn api_session_commands_cover_core_lifecycle_routes() {
    let responses = vec![
        json!({"items":[{"id":"s/one"}]}),
        json!({"s/one":{"type":"idle"}}),
        json!({"id":"s/one","title":"One"}),
        json!([{"id":"child"}]),
        json!([{"id":"todo-1","content":"Ship","status":"pending","priority":"high"}]),
        json!({"files":[]}),
        json!({"id":"created"}),
        json!({"id":"s/one","title":"Renamed"}),
        json!(true),
        json!(true),
        json!({"id":"forked"}),
        json!({"id":"s/one","reset":true}),
        json!(true),
        json!({"title":"Generated"}),
    ];
    let (port, requests) = spawn_server(responses);
    let port = port.to_string();
    let common = [
        "--host", "127.0.0.1", "--port", port.as_str(), "--directory", "/repo path",
    ];
    let create = br#"{"parentID":"parent","title":"New"}"#;
    let update = br#"{"title":"Renamed"}"#;
    let todo = br#"{"todos":[{"id":"todo-1","content":"Ship","status":"pending","priority":"high"}]}"#;
    let fork = br#"{"messageID":"m/1"}"#;
    let reset = br#"{"messageID":"m/1","revertCode":true}"#;
    let summarize = br#"{"messageID":"m/1"}"#;
    let title = br#"{"content":"hello","preferred_model":"gpt"}"#;

    let dangerous = [
        Case::new(&["session", "create", "--input", "-"], create),
        Case::new(&["session", "update", "s/one", "--input", "-"], update),
        Case::new(&["session", "delete", "s/one"], b""),
        Case::new(&["session", "todo-put", "s/one", "--input", "-"], todo),
        Case::new(&["session", "fork", "s/one", "--input", "-"], fork),
        Case::new(&["session", "reset", "s/one", "--input", "-"], reset),
        Case::new(&["session", "summarize", "s/one", "--input", "-"], summarize),
        Case::new(&["session", "title", "s/one", "--input", "-"], title),
    ];
    for case in &dangerous {
        let error = run_command(&common, &case.command, &case.input)
            .await
            .expect_err("session mutation must require --yes");
        assert!(error.contains("--yes"));
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }

    run_command(&common, &["session", "list"], b"").await.expect("list");
    assert_request(&requests, "GET", "/v1/session?directory=%2Frepo+path", Value::Null);
    run_command(&common, &["session", "status"], b"").await.expect("status");
    assert_request(&requests, "GET", "/v1/session/status?directory=%2Frepo+path", Value::Null);
    run_command(&common, &["session", "detail", "s/one"], b"").await.expect("detail");
    assert_request(&requests, "GET", "/v1/session/s%2Fone?directory=%2Frepo+path", Value::Null);
    run_command(&common, &["session", "children", "s/one"], b"").await.expect("children");
    assert_request(&requests, "GET", "/v1/session/s%2Fone/children?directory=%2Frepo+path", Value::Null);
    run_command(&common, &["session", "todo-get", "s/one"], b"").await.expect("todo get");
    assert_request(&requests, "GET", "/v1/session/s%2Fone/todo?directory=%2Frepo+path", Value::Null);
    run_command(&common, &["session", "diff", "s/one", "--message-id", "m/1"], b"")
        .await
        .expect("diff");
    assert_request(&requests, "GET", "/v1/session/s%2Fone/diff?directory=%2Frepo+path&messageID=m%2F1", Value::Null);

    for case in dangerous {
        run_command_with_yes(&common, &case.command, &case.input).await.expect("mutation");
    }
    assert_request(&requests, "POST", "/v1/session?directory=%2Frepo+path", json!({"parentID":"parent","title":"New"}));
    assert_request(&requests, "PATCH", "/v1/session/s%2Fone?directory=%2Frepo+path", json!({"title":"Renamed"}));
    assert_request(&requests, "DELETE", "/v1/session/s%2Fone?directory=%2Frepo+path", Value::Null);
    assert_request(&requests, "PUT", "/v1/session/s%2Fone/todo?directory=%2Frepo+path", serde_json::from_slice(todo).unwrap());
    assert_request(&requests, "POST", "/v1/session/s%2Fone/fork?directory=%2Frepo+path", serde_json::from_slice(fork).unwrap());
    assert_request(&requests, "POST", "/v1/session/s%2Fone/reset?directory=%2Frepo+path", serde_json::from_slice(reset).unwrap());
    assert_request(&requests, "POST", "/v1/session/s%2Fone/summarize?directory=%2Frepo+path", serde_json::from_slice(summarize).unwrap());
    assert_request(&requests, "POST", "/v1/session/s%2Fone/title", serde_json::from_slice(title).unwrap());
}

struct Case { command: Vec<String>, input: Vec<u8> }
impl Case {
    fn new(command: &[&str], input: impl AsRef<[u8]>) -> Self {
        Self { command: command.iter().map(|v| (*v).to_string()).collect(), input: input.as_ref().to_vec() }
    }
}

pub(super) async fn run_command(common: &[&str], command: &[impl AsRef<str>], input: &[u8]) -> Result<Vec<u8>, String> {
    run_args(common, command, input, false).await
}
async fn run_command_with_yes(common: &[&str], command: &[impl AsRef<str>], input: &[u8]) -> Result<Vec<u8>, String> {
    run_args(common, command, input, true).await
}
async fn run_args(common: &[&str], command: &[impl AsRef<str>], input: &[u8], yes: bool) -> Result<Vec<u8>, String> {
    let (family, command) = command.split_first().ok_or("API command family is required")?;
    let mut args = vec!["shiju", family.as_ref()];
    args.extend(common.iter().copied());
    if yes { args.push("--yes"); }
    args.extend(command.iter().map(AsRef::as_ref));
    let cli = Cli::try_parse_from(args).map_err(|e| e.to_string())?;
    let api_args = cli.command.into_api_args().ok_or("expected API command")?;
    let mut output = Vec::new();
    run_with_io(api_args, &mut Cursor::new(input), &mut output).await.map_err(|e| e.to_string())?;
    Ok(output)
}

#[derive(Debug)]
pub(super) struct RecordedRequest { method: String, path: String, body: Value }
pub(super) fn assert_request(rx: &mpsc::Receiver<RecordedRequest>, method: &str, path: &str, body: Value) {
    let request = rx.recv().expect("session request");
    assert_eq!(request.method, method);
    assert_eq!(request.path, path);
    assert_eq!(request.body, body);
}
pub(super) fn spawn_server(responses: Vec<Value>) -> (u16, mpsc::Receiver<RecordedRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || for response in responses {
        let (mut stream, _) = listener.accept().expect("accept");
        tx.send(read_request(&mut stream)).expect("record");
        let body = response.to_string();
        write!(stream, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}", body.len(), body).expect("respond");
    });
    (port, rx)
}
fn read_request(stream: &mut std::net::TcpStream) -> RecordedRequest {
    let mut bytes = Vec::new();
    let mut chunk = [0; 1024];
    let header_end = loop {
        if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") { break pos; }
        let read = stream.read(&mut chunk).expect("read"); assert!(read > 0); bytes.extend_from_slice(&chunk[..read]);
    };
    let headers = String::from_utf8(bytes[..header_end].to_vec()).unwrap();
    let len = headers.lines().filter_map(|l| l.split_once(':')).find(|(n, _)| n.eq_ignore_ascii_case("content-length")).and_then(|(_, v)| v.trim().parse().ok()).unwrap_or(0);
    let mut body = bytes[header_end + 4..].to_vec();
    while body.len() < len { let read = stream.read(&mut chunk).unwrap(); body.extend_from_slice(&chunk[..read]); }
    body.truncate(len);
    let mut line = headers.lines().next().unwrap().split_whitespace();
    RecordedRequest {
        method: line.next().unwrap().to_string(), path: line.next().unwrap().to_string(),
        body: if body.is_empty() { Value::Null } else { serde_json::from_slice(&body).unwrap() },
    }
}
