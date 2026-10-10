//! `operations` 运维接口路由、确认与敏感输入测试。

use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, TryRecvError};
use std::sync::{Arc, Mutex};

use clap::Parser;
use serde_json::{Value, json};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::{Interest, Subscriber};
use tracing::{Event, Metadata};

use super::run_with_io;
use crate::cli::Cli;

const CREDENTIAL_SENTINEL: &str = "operations-credential-sentinel-74bd";

#[tokio::test(flavor = "current_thread")]
async fn api_operations_commands_cover_cron_integrations_memory_and_doctor() {
    for args in [
        vec!["shiju", "cron", "list"],
        vec!["shiju", "integrations", "list"],
        vec!["shiju", "memory", "list"],
        vec!["shiju", "doctor"],
    ] {
        Cli::try_parse_from(args).expect("existing local command must keep parsing");
    }

    let responses = vec![
        (200, json!({"jobs": []})),
        (200, json!({"runs": []})),
        (200, json!({"status": "ok", "job": null})),
        (200, json!({"status": "ok", "job": null})),
        (204, Value::Null),
        (200, json!({"integrations": []})),
        (200, json!({"revision": "r1", "integrations": []})),
        (200, json!({"status": "ok", "revision": "r2"})),
        (200, json!({"entries": []})),
        (200, json!({"status": "ok"})),
        (200, json!({"status": "ok", "deleted": true})),
        (200, json!({"results": [], "summary": {"ok": 0, "warnings": 0, "errors": 0}})),
        (200, json!({"results": [], "summary": {"ok": 0, "warnings": 0, "errors": 0}})),
    ];
    let (port, requests) = spawn_operations_server(responses);
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
        "/tmp/operations",
    ];
    let captured = CapturedEvents::default();
    let _guard = tracing::subscriber::set_default(captured.clone());

    for command in [
        vec!["operations", "cron", "delete", "job /one"],
        vec!["operations", "integrations", "credentials", "slack /team", "--input", "-"],
        vec!["operations", "memory", "delete", "memory /one"],
    ] {
        let error = run_command(&common, &command, br#"{"fields":{"token":"secret"}}"#)
            .await
            .expect_err("dangerous operation must require --yes");
        assert!(error.contains("--yes"));
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }

    let cases = [
        Case::new(&["operations", "cron", "list"], b"", "GET", "/v1/cron", Value::Null),
        Case::new(
            &["operations", "cron", "runs", "job /one"],
            b"",
            "GET",
            "/v1/cron/job%20%2Fone/runs",
            Value::Null,
        ),
        Case::new(
            &["operations", "cron", "add", "--input", "-"],
            br#"{"name":"nightly","job_type":"command","schedule_kind":"cron","schedule":"0 0 * * *","command":"date"}"#,
            "POST",
            "/v1/cron",
            json!({
                "name": "nightly",
                "job_type": "command",
                "schedule_kind": "cron",
                "schedule": "0 0 * * *",
                "command": "date"
            }),
        ),
        Case::new(
            &["operations", "cron", "update", "job /one", "--input", "-"],
            br#"{"enabled":false}"#,
            "PATCH",
            "/v1/cron/job%20%2Fone",
            json!({"enabled": false}),
        ),
        Case::new(
            &["--yes", "operations", "cron", "delete", "job /one"],
            b"",
            "DELETE",
            "/v1/cron/job%20%2Fone",
            Value::Null,
        ),
        Case::new(
            &["operations", "integrations", "list"],
            b"",
            "GET",
            "/v1/integrations",
            Value::Null,
        ),
        Case::new(
            &["operations", "integrations", "settings"],
            b"",
            "GET",
            "/v1/integrations/settings",
            Value::Null,
        ),
        Case::new(
            &[
                "--yes",
                "operations",
                "integrations",
                "credentials",
                "slack /team",
                "--input",
                "-",
            ],
            format!("{{\"revision\":\"r1\",\"fields\":{{\"token\":\"{CREDENTIAL_SENTINEL}\"}}}}")
                .into_bytes(),
            "PUT",
            "/v1/integrations/slack%20%2Fteam/credentials",
            json!({"revision": "r1", "fields": {"token": CREDENTIAL_SENTINEL}}),
        ),
        Case::new(
            &["operations", "memory", "query", "--query", "find this"],
            b"",
            "GET",
            "/v1/memory?query=find+this",
            Value::Null,
        ),
        Case::new(
            &["operations", "memory", "store", "--input", "-"],
            br#"{"key":"memory-one","content":"remember this","category":"core"}"#,
            "POST",
            "/v1/memory",
            json!({"key": "memory-one", "content": "remember this", "category": "core"}),
        ),
        Case::new(
            &["--yes", "operations", "memory", "delete", "memory /one"],
            b"",
            "DELETE",
            "/v1/memory/memory%20%2Fone",
            Value::Null,
        ),
        Case::new(
            &["operations", "doctor", "get"],
            b"",
            "GET",
            "/v1/doctor",
            Value::Null,
        ),
        Case::new(
            &["operations", "doctor", "post"],
            b"",
            "POST",
            "/v1/doctor",
            json!({}),
        ),
    ];

    let mut user_surfaces = Vec::new();
    for case in cases {
        let output = run_command(&common, &case.command, &case.input)
            .await
            .unwrap_or_else(|error| panic!("operations command failed: {error}"));
        user_surfaces.extend(output);
        assert_request(
            &requests.recv().expect("operations request"),
            case.method,
            case.path,
            case.body,
        );
    }

    let surfaces = String::from_utf8(user_surfaces).expect("UTF-8 output");
    assert!(!surfaces.contains(CREDENTIAL_SENTINEL));
    assert!(!captured.joined().contains(CREDENTIAL_SENTINEL));
}

struct Case {
    command: Vec<String>,
    input: Vec<u8>,
    method: &'static str,
    path: &'static str,
    body: Value,
}

impl Case {
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

fn assert_request(request: &RecordedRequest, method: &str, path: &str, body: Value) {
    assert_eq!(request.method, method);
    assert_eq!(request.path, path);
    assert_eq!(request.body, body);
    let headers = request.headers.to_ascii_lowercase();
    assert!(headers.contains("\r\nx-platform-id: platform-a\r\n"));
    assert!(headers.contains("\r\nx-company-id: company-b\r\n"));
}

fn spawn_operations_server(responses: Vec<(u16, Value)>) -> (u16, mpsc::Receiver<RecordedRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind operations test server");
    let port = listener.local_addr().expect("operations server address").port();
    let (request_tx, request_rx) = mpsc::channel();
    std::thread::spawn(move || {
        for (status, response) in responses {
            let (mut stream, _) = listener.accept().expect("accept operations request");
            request_tx.send(read_request(&mut stream)).expect("send operations request");
            let response = response.to_string();
            write!(
                stream,
                "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}",
                response.len()
            )
            .expect("write operations response");
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
        let read = stream.read(&mut chunk).expect("read operations request");
        assert!(read > 0, "connection closed before operations request headers");
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
        let read = stream.read(&mut chunk).expect("read operations request body");
        assert!(read > 0, "connection closed before operations request body");
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

#[derive(Clone, Default)]
struct CapturedEvents(Arc<Mutex<Vec<String>>>);

impl CapturedEvents {
    fn joined(&self) -> String {
        self.0.lock().expect("captured events").join("\n")
    }
}

impl Subscriber for CapturedEvents {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, attrs: &Attributes<'_>) -> Id {
        let mut visitor = EventVisitor::default();
        attrs.record(&mut visitor);
        self.0.lock().expect("captured events").extend(visitor.values);
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, values: &Record<'_>) {
        let mut visitor = EventVisitor::default();
        values.record(&mut visitor);
        self.0.lock().expect("captured events").extend(visitor.values);
    }

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);
        self.0.lock().expect("captured events").extend(visitor.values);
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}

    fn register_callsite(&self, _metadata: &'static Metadata<'static>) -> Interest {
        Interest::always()
    }

    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(tracing::metadata::LevelFilter::TRACE)
    }

    fn clone_span(&self, id: &Id) -> Id {
        id.clone()
    }

    fn try_close(&self, _id: Id) -> bool {
        true
    }
}

#[derive(Default)]
struct EventVisitor {
    values: Vec<String>,
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.values.push(format!("{}={value:?}", field.name()));
    }
}
