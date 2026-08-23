use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

use clap::Parser;
use serde_json::json;

use super::context::ApiContext;
use super::input::read_json_input;
use super::output::{write_json, write_jsonl_item};
use super::{ApiCommands, ApiOutputFormat, ProviderCommands, ensure_confirmed, run_with_writer};
use crate::cli::{Cli, Commands};

#[tokio::test]
async fn api_cli_context_input_output_and_confirmation_are_stable() {
    let cli = Cli::try_parse_from(["shiju", "api", "provider", "list"])
        .expect("api provider list should parse");
    let Commands::Api { api_args } = cli.command else {
        panic!("expected api command");
    };
    assert_eq!(api_args.host, "127.0.0.1");
    assert_eq!(api_args.port, 14_682);
    assert_eq!(api_args.platform, "local");
    assert_eq!(api_args.company, "local");
    assert_eq!(api_args.output, ApiOutputFormat::Json);
    assert!(!api_args.yes);
    assert!(matches!(api_args.command, ApiCommands::Provider { command: ProviderCommands::List }));

    for (flag, value) in [("--platform", " "), ("--company", "\t")] {
        let error = Cli::try_parse_from(["shiju", "api", flag, value, "provider", "list"])
            .expect_err("blank tenant ids must be rejected");
        assert!(error.to_string().contains("must not be empty"));
    }

    let env_cli = Cli::try_parse_from([
        "shiju",
        "api",
        "--skey-env",
        "TEST_GATEWAY_SKEY",
        "provider",
        "list",
    ])
    .expect("environment auth source should parse");
    let Commands::Api { api_args: env_args } = env_cli.command else {
        panic!("expected api command");
    };
    let env_context = ApiContext::from_args_with_env(&env_args, |name| {
        (name == "TEST_GATEWAY_SKEY").then(|| "env-secret".into())
    })
    .expect("environment auth source should resolve");
    assert_eq!(
        env_context.client().endpoint().auth.as_ref().and_then(|auth| auth.skey.as_deref()),
        Some("env-secret")
    );

    let empty_skey_file = tempfile::NamedTempFile::new().expect("create empty skey file");
    let empty_skey_cli = Cli::try_parse_from([
        "shiju",
        "api",
        "--skey-file",
        empty_skey_file.path().to_str().expect("utf-8 temp path"),
        "provider",
        "list",
    ])
    .expect("file auth source should parse");
    let Commands::Api { api_args: empty_skey_args } = empty_skey_cli.command else {
        panic!("expected api command");
    };
    let error = ApiContext::from_args(&empty_skey_args).expect_err("empty skey file must fail");
    assert!(error.to_string().contains("skey file is empty"));

    let stdin_value = read_json_input("-", &mut Cursor::new(br#"{"source":"stdin"}"#))
        .expect("stdin JSON should parse");
    assert_eq!(stdin_value, json!({"source": "stdin"}));

    let mut json_file = tempfile::NamedTempFile::new().expect("create JSON file");
    json_file.write_all(br#"{"source":"file"}"#).expect("write JSON file");
    let file_value = read_json_input(
        json_file.path().to_str().expect("utf-8 temp path"),
        &mut Cursor::new(Vec::<u8>::new()),
    )
    .expect("file JSON should parse");
    assert_eq!(file_value, json!({"source": "file"}));
    let error =
        read_json_input("-", &mut Cursor::new(b"not-json")).expect_err("non-JSON input must fail");
    assert!(error.to_string().contains("invalid JSON"));

    let mut json_output = Vec::new();
    write_json(&mut json_output, &json!({"ok": true}), ApiOutputFormat::Json)
        .expect("JSON output should succeed");
    assert_eq!(
        String::from_utf8(json_output).expect("utf-8 JSON output"),
        "{\n  \"ok\": true\n}\n"
    );
    let mut jsonl_output = Vec::new();
    write_jsonl_item(&mut jsonl_output, &json!({"event": "done"}))
        .expect("JSONL output should succeed");
    assert_eq!(
        String::from_utf8(jsonl_output).expect("utf-8 JSONL output"),
        "{\"event\":\"done\"}\n"
    );

    let error = ensure_confirmed(false, "delete remote data")
        .expect_err("dangerous operations must default to refusal");
    assert!(error.to_string().contains("--yes"));
    ensure_confirmed(true, "delete remote data").expect("--yes should confirm operation");

    let mut skey_file = tempfile::NamedTempFile::new().expect("create skey file");
    skey_file.write_all(b"file-secret\n").expect("write skey file");
    let (port, request_rx) = spawn_provider_server();
    let request_cli = Cli::try_parse_from([
        "shiju",
        "api",
        "--host",
        "127.0.0.1",
        "--port",
        &port.to_string(),
        "--platform",
        "platform-a",
        "--company",
        "company-b",
        "--skey-file",
        skey_file.path().to_str().expect("utf-8 temp path"),
        "--directory",
        "/tmp/workspace",
        "provider",
        "list",
    ])
    .expect("fully configured api command should parse");
    let Commands::Api { api_args: request_args } = request_cli.command else {
        panic!("expected api command");
    };
    let mut output = Vec::new();
    run_with_writer(request_args, &mut output).await.expect("provider list request should succeed");

    let request = request_rx.recv().expect("server should capture request").to_ascii_lowercase();
    assert!(request.starts_with("get /v1/provider?directory=%2ftmp%2fworkspace http/1.1\r\n"));
    assert!(request.contains("\r\nauthorization: bearer file-secret\r\n"));
    assert!(request.contains("\r\nx-platform-id: platform-a\r\n"));
    assert!(request.contains("\r\nx-company-id: company-b\r\n"));

    let output = String::from_utf8(output).expect("utf-8 command output");
    assert_eq!(output, "{\n  \"all\": [],\n  \"default\": {},\n  \"connected\": []\n}\n");
    assert!(!output.contains("file-secret"));
}

fn spawn_provider_server() -> (u16, mpsc::Receiver<String>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test server");
    let port = listener.local_addr().expect("read test server address").port();
    let (request_tx, request_rx) = mpsc::channel();

    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept provider request");
        let mut request = Vec::new();
        let mut chunk = [0_u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream.read(&mut chunk).expect("read provider request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
        }
        request_tx
            .send(String::from_utf8(request).expect("request should be UTF-8"))
            .expect("send captured request");

        let body = br#"{"all":[],"default":{},"connected":[]}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        )
        .expect("write response headers");
        stream.write_all(body).expect("write response body");
    });

    (port, request_rx)
}
