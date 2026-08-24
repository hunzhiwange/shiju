//! `provider` 与 `control` 的路由、敏感输入和危险操作测试。

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

use super::input::read_json_input_with_env;
use super::run_with_io;
use crate::cli::Cli;

const PROVIDER_ID: &str = "openai /合作";
const ENCODED_PROVIDER_ID: &str = "openai%20%2F%E5%90%88%E4%BD%9C";
const CREDENTIAL_SENTINEL: &str = "credential-sentinel-5f8c";
const OAUTH_CODE_SENTINEL: &str = "oauth-code-sentinel-9a21";

#[tokio::test(flavor = "current_thread")]
async fn api_control_commands_protect_credentials_and_destructive_actions() {
    let responses = vec![
        (200, json!({"all": [], "default": {}, "connected": []})),
        (200, json!({"openai": [{"type": "api", "label": "API Key"}]})),
        (
            200,
            json!({
                "url": "https://example.test/oauth",
                "method": "browser",
                "instructions": "open the URL"
            }),
        ),
        (200, json!(true)),
        (200, json!(true)),
        (200, json!(true)),
        (200, json!([])),
        (200, json!({"plugin_id": "plugin-a", "hook_hash": "hash-a", "trusted": true})),
        (200, json!({"plugin_id": "plugin-a", "hook_hash": "hash-a", "trusted": false})),
        (200, json!([])),
        (200, json!(true)),
        (200, json!(true)),
    ];
    let (port, requests) = spawn_control_server(responses);
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
        "/tmp/instance one",
    ];
    let captured = CapturedEvents::default();
    let _guard = tracing::subscriber::set_default(captured.clone());
    let mut user_surfaces = Vec::new();
    let env_input = read_json_input_with_env(
        "env:CONTROL_CREDENTIAL_JSON",
        &mut Cursor::new(Vec::<u8>::new()),
        |name| {
            (name == "CONTROL_CREDENTIAL_JSON")
                .then(|| format!("{{\"type\":\"api\",\"key\":\"{CREDENTIAL_SENTINEL}\"}}").into())
        },
    )
    .expect("credential JSON should be readable from an environment source");
    assert_eq!(env_input["key"], CREDENTIAL_SENTINEL);

    for command in [
        vec!["control", "auth", "remove", PROVIDER_ID],
        vec!["control", "hooks", "trust", "--plugin-id", "plugin-a", "--hook-hash", "hash-a"],
        vec!["control", "hooks", "revoke", "--plugin-id", "plugin-a", "--hook-hash", "hash-a"],
        vec!["control", "global", "dispose"],
        vec!["control", "instance", "dispose"],
    ] {
        let error = run_command(&common, &command, b"")
            .await
            .expect_err("dangerous operation must require --yes");
        assert!(error.contains("--yes"));
        user_surfaces.extend_from_slice(error.as_bytes());
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }
    let global_error = run_command(&common, &["control", "global", "dispose"], b"")
        .await
        .expect_err("global dispose must identify the target");
    assert!(global_error.contains("platform-a/company-b"));
    let instance_error = run_command(&common, &["control", "instance", "dispose"], b"")
        .await
        .expect_err("instance dispose must identify the target");
    assert!(instance_error.contains("platform-a/company-b"));
    assert!(instance_error.contains("/tmp/instance one"));
    assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    let callback_error = run_command(
        &common,
        &["provider", "oauth", "callback", PROVIDER_ID, "--method", "1", "--input", "-"],
        br#"{"code":"  "}"#,
    )
    .await
    .expect_err("blank OAuth callback code must be rejected");
    assert!(callback_error.contains("must not be empty"));
    assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));

    user_surfaces.extend(run_command(&common, &["provider", "list"], b"").await.unwrap());
    assert_request(
        &requests.recv().expect("provider list"),
        "GET",
        "/v1/provider?directory=%2Ftmp%2Finstance+one",
        Value::Null,
    );

    user_surfaces.extend(run_command(&common, &["provider", "auth"], b"").await.unwrap());
    assert_request(
        &requests.recv().expect("provider auth methods"),
        "GET",
        "/v1/provider/auth",
        Value::Null,
    );

    user_surfaces.extend(
        run_command(
            &common,
            &["provider", "oauth", "authorize", PROVIDER_ID, "--method", "1"],
            b"",
        )
        .await
        .unwrap(),
    );
    assert_request(
        &requests.recv().expect("provider OAuth authorize"),
        "POST",
        &format!("/v1/provider/{ENCODED_PROVIDER_ID}/oauth/authorize"),
        json!({"method": 1}),
    );

    let mut callback_file = tempfile::NamedTempFile::new().expect("create OAuth callback file");
    write!(callback_file, "{{\"code\":\"{OAUTH_CODE_SENTINEL}\"}}").expect("write OAuth callback");
    user_surfaces.extend(
        run_command(
            &common,
            &[
                "provider",
                "oauth",
                "callback",
                PROVIDER_ID,
                "--method",
                "1",
                "--input",
                callback_file.path().to_str().expect("UTF-8 callback path"),
            ],
            b"",
        )
        .await
        .unwrap(),
    );
    assert_request(
        &requests.recv().expect("provider OAuth callback"),
        "POST",
        &format!("/v1/provider/{ENCODED_PROVIDER_ID}/oauth/callback"),
        json!({"method": 1, "code": OAUTH_CODE_SENTINEL}),
    );

    user_surfaces.extend(
        run_command(
            &common,
            &["control", "auth", "set", PROVIDER_ID, "--input", "-"],
            format!("{{\"type\":\"api\",\"key\":\"{CREDENTIAL_SENTINEL}\"}}").as_bytes(),
        )
        .await
        .unwrap(),
    );
    assert_request(
        &requests.recv().expect("auth set"),
        "PUT",
        &format!("/v1/auth/{ENCODED_PROVIDER_ID}"),
        json!({"type": "api", "key": CREDENTIAL_SENTINEL}),
    );

    user_surfaces.extend(
        run_command(&common, &["--yes", "control", "auth", "remove", PROVIDER_ID], b"")
            .await
            .unwrap(),
    );
    assert_request(
        &requests.recv().expect("auth remove"),
        "DELETE",
        &format!("/v1/auth/{ENCODED_PROVIDER_ID}"),
        Value::Null,
    );

    user_surfaces.extend(run_command(&common, &["control", "hooks", "list"], b"").await.unwrap());
    assert_request(&requests.recv().expect("hooks list"), "GET", "/v1/hooks", Value::Null);

    for (action, trusted) in [("trust", true), ("revoke", false)] {
        user_surfaces.extend(
            run_command(
                &common,
                &[
                    "--yes",
                    "control",
                    "hooks",
                    action,
                    "--plugin-id",
                    "plugin-a",
                    "--hook-hash",
                    "hash-a",
                ],
                b"",
            )
            .await
            .unwrap(),
        );
        assert_request(
            &requests.recv().expect("hook mutation"),
            "POST",
            &format!("/v1/hooks/{action}"),
            json!({"plugin_id": "plugin-a", "hook_hash": "hash-a"}),
        );
        assert!(String::from_utf8_lossy(&user_surfaces).contains(&trusted.to_string()));
    }

    user_surfaces.extend(run_command(&common, &["control", "hooks", "audit"], b"").await.unwrap());
    assert_request(&requests.recv().expect("hook audit"), "GET", "/v1/hooks/audit", Value::Null);

    user_surfaces.extend(
        run_command(&common, &["--yes", "control", "global", "dispose"], b"").await.unwrap(),
    );
    assert_request(
        &requests.recv().expect("global dispose"),
        "POST",
        "/v1/global/dispose",
        json!({}),
    );

    user_surfaces.extend(
        run_command(&common, &["--yes", "control", "instance", "dispose"], b"").await.unwrap(),
    );
    assert_request(
        &requests.recv().expect("instance dispose"),
        "POST",
        "/v1/instance/dispose?directory=%2Ftmp%2Finstance+one",
        json!({}),
    );

    let surfaces = String::from_utf8(user_surfaces).expect("UTF-8 user surfaces");
    let traces = captured.joined();
    for secret in [CREDENTIAL_SENTINEL, OAUTH_CODE_SENTINEL] {
        assert!(!surfaces.contains(secret), "user output exposed {secret}");
        assert!(!traces.contains(secret), "tracing exposed {secret}");
    }
}

async fn run_command(common: &[&str], command: &[&str], input: &[u8]) -> Result<Vec<u8>, String> {
    let (yes, command) = if command.first() == Some(&"--yes") { (Some("--yes"), &command[1..]) } else { (None, command) };
    let (family, command) = command.split_first().ok_or("API command family is required")?;
    let args = std::iter::once("shiju")
        .chain(std::iter::once(*family))
        .chain(common.iter().copied())
        .chain(yes)
        .chain(command.iter().copied())
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

fn spawn_control_server(responses: Vec<(u16, Value)>) -> (u16, mpsc::Receiver<RecordedRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind control test server");
    let port = listener.local_addr().expect("control server address").port();
    let (request_tx, request_rx) = mpsc::channel();
    std::thread::spawn(move || {
        for (status, response) in responses {
            let (mut stream, _) = listener.accept().expect("accept control request");
            request_tx.send(read_request(&mut stream)).expect("send control request");
            let response = response.to_string();
            write!(
                stream,
                "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}",
                response.len()
            )
            .expect("write control response");
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
        let read = stream.read(&mut chunk).expect("read control request");
        assert!(read > 0, "connection closed before control request headers");
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
        let read = stream.read(&mut chunk).expect("read control request body");
        assert!(read > 0, "connection closed before control request body");
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

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}
    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);
        self.0.lock().expect("captured events").push(visitor.fields.join(" "));
    }

    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}

    fn register_callsite(&self, _metadata: &'static Metadata<'static>) -> Interest {
        Interest::always()
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
    fields: Vec<String>,
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields.push(format!("{}={value:?}", field.name()));
    }
}
