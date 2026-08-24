//! `interactive` 路由、输入校验与确认测试。

use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, TryRecvError};

use clap::Parser;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

use super::run_with_io;
use crate::cli::Cli;

#[tokio::test(flavor = "current_thread")]
async fn api_interactive_commands_cover_permission_question_and_pty() {
    let responses = vec![
        (200, json!([])),
        (200, json!(true)),
        (200, json!([])),
        (200, json!(true)),
        (200, json!(true)),
        (200, json!([])),
        (200, pty_info("pty-created", "running")),
        (200, pty_info("pty /one", "running")),
        (200, pty_info("pty /one", "exited")),
        (200, json!(true)),
    ];
    let (port, requests) = spawn_http_server(responses);
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
            &["interactive", "permission", "reply", "permission /one", "once"],
            b"",
            "permission /one",
        ),
        Case::new(
            &["interactive", "question", "reply", "question /one", "--input", "-"],
            br#"[["A"]]"#,
            "question /one",
        ),
        Case::new(
            &["interactive", "question", "reject", "question /one"],
            b"",
            "question /one",
        ),
        Case::new(
            &["interactive", "pty", "create", "--input", "-"],
            br#"{"command":"bash","cwd":"/tmp/project root"}"#,
            "create PTY",
        ),
        Case::new(
            &["interactive", "pty", "update", "pty /one", "--input", "-"],
            br#"{"title":"renamed","size":{"rows":32,"cols":120}}"#,
            "pty /one",
        ),
        Case::new(
            &["interactive", "pty", "delete", "pty /one"],
            b"",
            "pty /one",
        ),
        Case::new(
            &["interactive", "pty", "connect", "pty /one", "--cursor", "42"],
            b"",
            "pty /one",
        ),
    ];
    for case in &dangerous {
        let error = run_command(&common, &case.command, &case.input)
            .await
            .expect_err("interactive side effect must require --yes");
        assert!(error.contains("--yes"), "unexpected confirmation error: {error}");
        assert!(error.contains(case.target), "confirmation omitted target: {error}");
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }

    let invalid = run_command(
        &common,
        &["--yes", "interactive", "question", "reply", "question /one", "--input", "-"],
        br#"[[]]"#,
    )
    .await
    .expect_err("empty answer must fail");
    assert!(invalid.contains("answer"));
    assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));

    let unknown = run_command(
        &common,
        &[
            "--yes",
            "interactive",
            "permission",
            "reply",
            "permission /one",
            "sometimes",
        ],
        b"",
    )
    .await
    .expect_err("unknown permission reply must fail");
    assert!(unknown.contains("invalid value"));
    assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));

    let empty_id = run_command(
        &common,
        &["interactive", "question", "reject", "   "],
        b"",
    )
    .await
    .expect_err("blank request id must fail");
    assert!(empty_id.contains("must not be empty"));
    assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));

    let cases = [
        RequestCase::new(
            &["interactive", "permission", "list"],
            b"",
            "GET",
            "/v1/permission?directory=%2Ftmp%2Fproject+root",
            Value::Null,
        ),
        RequestCase::new(
            &["--yes", "interactive", "permission", "reply", "permission /one", "always", "--message", "trusted"],
            b"",
            "POST",
            "/v1/permission/permission%20%2Fone/reply?directory=%2Ftmp%2Fproject+root",
            json!({"reply":"always","message":"trusted"}),
        ),
        RequestCase::new(
            &["interactive", "question", "list"],
            b"",
            "GET",
            "/v1/question",
            Value::Null,
        ),
        RequestCase::new(
            &["--yes", "interactive", "question", "reply", "question /one", "--input", "-"],
            br#"[["A"],["custom","B"]]"#,
            "POST",
            "/v1/question/question%20%2Fone/reply",
            json!({"answers":[["A"],["custom","B"]]}),
        ),
        RequestCase::new(
            &["--yes", "interactive", "question", "reject", "question /one"],
            b"",
            "POST",
            "/v1/question/question%20%2Fone/reject",
            json!({}),
        ),
        RequestCase::new(
            &["interactive", "pty", "list"],
            b"",
            "GET",
            "/v1/pty?directory=%2Ftmp%2Fproject+root",
            Value::Null,
        ),
        RequestCase::new(
            &["--yes", "interactive", "pty", "create", "--input", "-"],
            dangerous[3].input.as_slice(),
            "POST",
            "/v1/pty?directory=%2Ftmp%2Fproject+root",
            json!({"command":"bash","cwd":"/tmp/project root"}),
        ),
        RequestCase::new(
            &["interactive", "pty", "detail", "pty /one"],
            b"",
            "GET",
            "/v1/pty/pty%20%2Fone?directory=%2Ftmp%2Fproject+root",
            Value::Null,
        ),
        RequestCase::new(
            &["--yes", "interactive", "pty", "update", "pty /one", "--input", "-"],
            dangerous[4].input.as_slice(),
            "PUT",
            "/v1/pty/pty%20%2Fone?directory=%2Ftmp%2Fproject+root",
            json!({"title":"renamed","size":{"rows":32,"cols":120}}),
        ),
        RequestCase::new(
            &["--yes", "interactive", "pty", "delete", "pty /one"],
            b"",
            "DELETE",
            "/v1/pty/pty%20%2Fone?directory=%2Ftmp%2Fproject+root",
            Value::Null,
        ),
    ];

    for case in cases {
        run_command(&common, &case.command, &case.input)
            .await
            .unwrap_or_else(|error| panic!("interactive command failed: {error}"));
        assert_request(&requests.recv().expect("interactive request"), &case);
    }

    let (ws_port, handshake) = spawn_websocket_server();
    let ws_port = ws_port.to_string();
    let ws_common = [
        "--host",
        "127.0.0.1",
        "--port",
        ws_port.as_str(),
        "--platform",
        "platform-a",
        "--company",
        "company-b",
        "--directory",
        "/tmp/project root",
    ];
    let output = run_command(
        &ws_common,
        &[
            "--yes",
            "interactive",
            "pty",
            "connect",
            "pty /one",
            "--cursor",
            "42",
        ],
        b"",
    )
    .await
    .expect("PTY connect command");
    let output: Value = serde_json::from_slice(&output).expect("PTY connect JSON output");
    assert_eq!(output["status"], "closed");
    assert_eq!(output["events"][0]["type"], "closed");
    let handshake = handshake.recv().expect("PTY connect handshake");
    assert_eq!(
        handshake.path,
        "/v1/pty/pty%20%2Fone/connect?directory=%2Ftmp%2Fproject+root&cursor=42"
    );
    assert_eq!(handshake.platform.as_deref(), Some("platform-a"));
    assert_eq!(handshake.company.as_deref(), Some("company-b"));
}

fn pty_info(id: &str, status: &str) -> Value {
    json!({
        "id":id,
        "title":"terminal",
        "command":"bash",
        "args":[],
        "cwd":"/tmp/project root",
        "status":status,
        "pid":123
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

fn assert_request(request: &RecordedRequest, case: &RequestCase) {
    assert_eq!(request.method, case.method);
    assert_eq!(request.path, case.path);
    assert_eq!(request.body, case.body);
    let headers = request.headers.to_ascii_lowercase();
    assert!(headers.contains("\r\nx-platform-id: platform-a\r\n"));
    assert!(headers.contains("\r\nx-company-id: company-b\r\n"));
}

fn spawn_http_server(responses: Vec<(u16, Value)>) -> (u16, mpsc::Receiver<RecordedRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind interactive server");
    let port = listener.local_addr().expect("interactive server address").port();
    let (request_tx, request_rx) = mpsc::channel();
    std::thread::spawn(move || {
        for (status, response) in responses {
            let (mut stream, _) = listener.accept().expect("accept interactive request");
            request_tx.send(read_request(&mut stream)).expect("send interactive request");
            let response = response.to_string();
            write!(
                stream,
                "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}",
                response.len()
            )
            .expect("write interactive response");
        }
    });
    (port, request_rx)
}

#[derive(Debug)]
struct HandshakeRequest {
    path: String,
    platform: Option<String>,
    company: Option<String>,
}

fn spawn_websocket_server() -> (u16, mpsc::Receiver<HandshakeRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind PTY websocket server");
    let port = listener.local_addr().expect("PTY websocket address").port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept PTY websocket");
        let callback = move |request: &Request, response: Response| {
            tx.send(HandshakeRequest {
                path: request.uri().to_string(),
                platform: header(request, "x-platform-id"),
                company: header(request, "x-company-id"),
            })
            .expect("record PTY handshake");
            Ok(response)
        };
        let mut socket = tokio_tungstenite::tungstenite::accept_hdr(stream, callback)
            .expect("accept PTY websocket handshake");
        socket.close(None).expect("close PTY websocket");
    });
    (port, rx)
}

fn header(request: &Request, name: &str) -> Option<String> {
    request.headers().get(name).and_then(|value| value.to_str().ok()).map(str::to_string)
}

fn read_request(stream: &mut std::net::TcpStream) -> RecordedRequest {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 1024];
    let header_end = loop {
        if let Some(position) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break position;
        }
        let read = stream.read(&mut chunk).expect("read interactive request");
        assert!(read > 0, "connection closed before request headers");
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
        let read = stream.read(&mut chunk).expect("read interactive request body");
        assert!(read > 0, "connection closed before request body");
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
