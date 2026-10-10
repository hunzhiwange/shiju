//! `knowledge` 资源与检索路由测试。

use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, TryRecvError};

use clap::Parser;
use serde_json::{Value, json};

use super::run_with_io;
use crate::cli::Cli;

#[tokio::test(flavor = "current_thread")]
async fn api_knowledge_commands_cover_dataset_document_and_retrieval_routes() {
    let responses = vec![
        json!({"full_text":true,"vector":false,"hybrid":false,"rerank":false,"notes":{}}),
        json!({"provider_id":"local","route_hint":"knowledge","model":"embed","dimensions":3,"api_key_configured":true,"vector_available":true,"hybrid_available":true,"retrieval_modes":["full_text"],"notes":{}}),
        json!([]),
        json!({"ok":true,"provider_id":"local","model":"embed","configured_dimensions":3,"observed_dimensions":3,"latency_ms":1,"message":"ok"}),
        json!([]),
        dataset(),
        dataset(),
        dataset(),
        json!([]),
        document(),
        json!({"document":document(),"filename":"guide.md","source_bytes":3,"extracted_text_bytes":3,"warnings":[]}),
        json!({"document":document(),"job":job(),"filename":"guide.md","source_bytes":3,"warnings":[]}),
        job(),
        json!([]),
        document(),
        document(),
        json!([]),
        document(),
        json!({"chunks":[]}),
        json!({"chunks":[]}),
    ];
    let (port, requests) = spawn_server(responses);
    let port = port.to_string();
    let common = ["--host", "127.0.0.1", "--port", port.as_str()];

    for command in [
        vec!["knowledge", "embedding", "test", "--input", "-"],
        vec!["knowledge", "dataset", "delete", "dataset /one"],
        vec!["knowledge", "document", "import", "dataset /one", "--input", "-"],
        vec!["knowledge", "document", "import-async", "dataset /one", "--input", "-"],
        vec!["knowledge", "document", "batch", "--input", "-"],
        vec!["knowledge", "document", "delete", "doc /one"],
        vec!["knowledge", "document", "reindex", "doc /one", "--input", "-"],
        vec!["knowledge", "retrieval-test", "--input", "-"],
    ] {
        let error =
            run_command(&common, &command, b"").await.expect_err("dangerous knowledge operation");
        assert!(error.contains("--yes"), "unexpected error: {error}");
        assert!(matches!(requests.try_recv(), Err(TryRecvError::Empty)));
    }

    let cases = [
        Case::new(&["knowledge", "status"], b"", "GET", "/v1/knowledge/status", Value::Null),
        Case::new(
            &["knowledge", "embedding", "status"],
            b"",
            "GET",
            "/v1/knowledge/embedding/status",
            Value::Null,
        ),
        Case::new(
            &["knowledge", "embedding", "presets"],
            b"",
            "GET",
            "/v1/knowledge/embedding/presets",
            Value::Null,
        ),
        Case::new(
            &["--yes", "knowledge", "embedding", "test", "--input", "-"],
            br#"{"sample_text":"hello"}"#,
            "POST",
            "/v1/knowledge/embedding/test",
            json!({"sample_text":"hello"}),
        ),
        Case::new(
            &["knowledge", "dataset", "list"],
            b"",
            "GET",
            "/v1/knowledge/datasets",
            Value::Null,
        ),
        Case::new(
            &["knowledge", "dataset", "create", "--input", "-"],
            br#"{"name":"Docs"}"#,
            "POST",
            "/v1/knowledge/datasets",
            json!({"name":"Docs","description":"","chunking_mode":"general","indexing_mode":"economy","retrieval_mode":"full_text","keyword_count":10,"top_k":10,"score_threshold_enabled":false,"score_threshold":0.15,"rerank_enabled":false}),
        ),
        Case::new(
            &["knowledge", "dataset", "detail", "dataset /one"],
            b"",
            "GET",
            "/v1/knowledge/datasets/dataset%20%2Fone",
            Value::Null,
        ),
        Case::new(
            &["--yes", "knowledge", "dataset", "delete", "dataset /one"],
            b"",
            "DELETE",
            "/v1/knowledge/datasets/dataset%20%2Fone",
            json!({}),
        ),
        Case::new(
            &["knowledge", "document", "list", "dataset /one", "--status", "archived"],
            b"",
            "GET",
            "/v1/knowledge/datasets/dataset%20%2Fone/documents?status=archived",
            Value::Null,
        ),
        Case::new(
            &["knowledge", "document", "create", "dataset /one", "--input", "-"],
            br#"{"name":"Readme","content":"hello","metadata":{},"enabled":true}"#,
            "POST",
            "/v1/knowledge/datasets/dataset%20%2Fone/documents",
            json!({"name":"Readme","content":"hello","metadata":{},"enabled":true}),
        ),
        Case::new(
            &["--yes", "knowledge", "document", "import", "dataset /one", "--input", "-"],
            import_input(),
            "POST",
            "/v1/knowledge/datasets/dataset%20%2Fone/documents/import",
            import_body(),
        ),
        Case::new(
            &["--yes", "knowledge", "document", "import-async", "dataset /one", "--input", "-"],
            import_input(),
            "POST",
            "/v1/knowledge/datasets/dataset%20%2Fone/documents/import-async",
            import_body(),
        ),
        Case::new(
            &["knowledge", "job", "job /one"],
            b"",
            "GET",
            "/v1/knowledge/index-jobs/job%20%2Fone",
            Value::Null,
        ),
        Case::new(
            &["--yes", "knowledge", "document", "batch", "--input", "-"],
            br#"{"document_ids":["doc /one"],"action":"archive"}"#,
            "POST",
            "/v1/knowledge/documents/batch",
            json!({"document_ids":["doc /one"],"action":"archive"}),
        ),
        Case::new(
            &["knowledge", "document", "update", "doc /one", "--input", "-"],
            br#"{"name":"renamed"}"#,
            "PATCH",
            "/v1/knowledge/documents/doc%20%2Fone",
            json!({"name":"renamed"}),
        ),
        Case::new(
            &["--yes", "knowledge", "document", "delete", "doc /one"],
            b"",
            "DELETE",
            "/v1/knowledge/documents/doc%20%2Fone",
            json!({}),
        ),
        Case::new(
            &["knowledge", "document", "chunks", "doc /one"],
            b"",
            "GET",
            "/v1/knowledge/documents/doc%20%2Fone/chunks",
            Value::Null,
        ),
        Case::new(
            &["--yes", "knowledge", "document", "reindex", "doc /one", "--input", "-"],
            br#"{"force":true}"#,
            "POST",
            "/v1/knowledge/documents/doc%20%2Fone/reindex",
            json!({"force":true}),
        ),
        Case::new(
            &["knowledge", "retrieve", "--input", "-"],
            retrieve_input(),
            "POST",
            "/v1/knowledge/retrieve",
            retrieve_body(),
        ),
        Case::new(
            &["--yes", "knowledge", "retrieval-test", "--input", "-"],
            retrieve_input(),
            "POST",
            "/v1/knowledge/retrieval-test",
            retrieve_body(),
        ),
    ];
    for case in cases {
        run_command(&common, &case.command, &case.input)
            .await
            .unwrap_or_else(|error| panic!("knowledge command failed: {error}"));
        assert_request(&requests.recv().expect("knowledge request"), &case);
    }
}

fn dataset() -> Value {
    json!({"id":"dataset /one","name":"Docs","description":"","document_count":1,"chunk_count":0,"created_at_ms":1,"updated_at_ms":2})
}

fn document() -> Value {
    json!({"id":"doc /one","dataset_id":"dataset /one","name":"Readme","metadata":{},"enabled":true,"chunk_count":0,"created_at_ms":1,"updated_at_ms":2})
}

fn job() -> Value {
    json!({"id":"job /one","dataset_id":"dataset /one","document_id":"doc /one","status":"queued","created_at_ms":1,"updated_at_ms":1})
}

fn import_input() -> &'static [u8] {
    br#"{"filename":"guide.md","content_base64":"YWJj","metadata":{},"enabled":true}"#
}

fn import_body() -> Value {
    json!({"filename":"guide.md","content_base64":"YWJj","metadata":{},"enabled":true})
}

fn retrieve_input() -> &'static [u8] {
    br#"{"query":"hello","dataset_ids":["dataset /one"],"top_k":5,"include_context":true}"#
}

fn retrieve_body() -> Value {
    json!({"query":"hello","dataset_ids":["dataset /one"],"top_k":5,"include_context":true})
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

fn assert_request(request: &RecordedRequest, case: &Case) {
    assert_eq!(request.method, case.method);
    assert_eq!(request.path, case.path);
    assert_eq!(request.body, case.body);
    let headers = request.headers.to_ascii_lowercase();
    assert!(headers.contains("\r\nx-platform-id: local\r\n"));
    assert!(headers.contains("\r\nx-company-id: local\r\n"));
}

fn spawn_server(responses: Vec<Value>) -> (u16, mpsc::Receiver<RecordedRequest>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind knowledge server");
    let port = listener.local_addr().expect("knowledge address").port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().expect("accept knowledge request");
            tx.send(read_request(&mut stream)).expect("record knowledge request");
            let response = response.to_string();
            write!(stream, "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response}", response.len()).expect("write response");
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
    let length = headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = bytes[(header_end + 4)..].to_vec();
    while body.len() < length {
        let read = stream.read(&mut chunk).expect("body");
        assert!(read > 0);
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(length);
    let mut line = headers.lines().next().expect("request line").split_whitespace();
    let method = line.next().expect("method").to_string();
    let path = line.next().expect("path").to_string();
    let body =
        if body.is_empty() { Value::Null } else { serde_json::from_slice(&body).expect("JSON") };
    RecordedRequest { method, path, headers, body }
}
