use super::*;
use std::io::Cursor;

fn framed_message(value: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(value).unwrap();
    let mut message = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    message.extend(body);
    message
}

fn service_with_transport() -> (SharedLspService, mpsc::Receiver<Vec<u8>>) {
    let (writer, receiver) = mpsc::channel();
    let service = SharedLspService {
        child: Mutex::new(None),
        writer,
        request_id: AtomicU64::new(1),
        pending: Mutex::new(HashMap::new()),
        documents: Mutex::new(HashMap::new()),
    };
    (service, receiver)
}

fn receive_message(receiver: &mpsc::Receiver<Vec<u8>>) -> Value {
    let bytes = receiver.try_recv().unwrap();
    let mut reader = BufReader::new(Cursor::new(bytes));
    read_lsp_message(&mut reader).unwrap()
}

#[tokio::test]
async fn open_returns_none_for_an_unsupported_file_type() {
    let session = LspBackendSession::open(
        Path::new("/workspace"),
        Path::new("/workspace/notes.unsupported"),
        "content",
    )
    .await
    .unwrap();

    assert!(session.is_none());
}

#[test]
fn session_accessors_return_the_session_metadata() {
    let (service, _receiver) = service_with_transport();
    let session = LspBackendSession {
        service: Arc::new(service),
        server_key: "test-server".to_string(),
        language_id: "test-language".to_string(),
        uri: "file:///workspace/main.test".to_string(),
    };

    assert_eq!(session.server_key(), "test-server");
    assert_eq!(session.language_id(), "test-language");
    assert_eq!(session.uri(), "file:///workspace/main.test");
}

#[tokio::test]
async fn session_request_forwards_to_the_shared_service() {
    let (service, receiver) = service_with_transport();
    let service = Arc::new(service);
    let session = LspBackendSession {
        service: service.clone(),
        server_key: "test-server".to_string(),
        language_id: "test-language".to_string(),
        uri: "file:///workspace/main.test".to_string(),
    };
    let request = tokio::spawn(async move {
        session.request("textDocument/hover", json!({"position": {"line": 1}})).await
    });
    while service.pending.lock().is_empty() {
        tokio::task::yield_now().await;
    }

    service.handle_message(json!({"id": 1, "result": {"contents": "symbol"}}));

    assert_eq!(request.await.unwrap().unwrap(), json!({"contents": "symbol"}));
    assert_eq!(
        receive_message(&receiver),
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/hover",
            "params": {"position": {"line": 1}}
        })
    );
}

#[test]
fn send_notification_writes_a_framed_json_rpc_notification() {
    let (service, receiver) = service_with_transport();

    service.send_notification("workspace/didChange", json!({"value": 3})).unwrap();

    assert_eq!(
        receive_message(&receiver),
        json!({
            "jsonrpc": "2.0",
            "method": "workspace/didChange",
            "params": {"value": 3}
        })
    );
}

#[test]
fn send_raw_reports_a_closed_transport() {
    let (service, receiver) = service_with_transport();
    drop(receiver);

    let error = service.send_raw(&json!({"value": true})).unwrap_err();

    assert_eq!(error.to_string(), "LSP transport is closed");
}

#[tokio::test]
async fn sync_document_opens_and_records_a_new_document() {
    let (service, receiver) = service_with_transport();
    let uri = "file:///workspace/main.rs";

    service.sync_document(uri, "rust", "fn main() {}").await.unwrap();

    assert_eq!(
        receive_message(&receiver),
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "rust",
                    "version": 1,
                    "text": "fn main() {}"
                }
            }
        })
    );
    let state = service.documents.lock().get(uri).cloned().unwrap();
    assert_eq!(state.version, 1);
    assert_eq!(state.text, "fn main() {}");
}

#[tokio::test]
async fn sync_document_skips_an_unchanged_document() {
    let (service, receiver) = service_with_transport();
    let uri = "file:///workspace/main.rs";

    service.sync_document(uri, "rust", "same").await.unwrap();
    let _ = receive_message(&receiver);
    service.sync_document(uri, "rust", "same").await.unwrap();

    assert!(matches!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty)));
    assert_eq!(service.documents.lock().get(uri).unwrap().version, 1);
}

#[tokio::test]
async fn sync_document_changes_text_and_increments_the_version() {
    let (service, receiver) = service_with_transport();
    let uri = "file:///workspace/main.rs";

    service.sync_document(uri, "rust", "before").await.unwrap();
    let _ = receive_message(&receiver);
    service.sync_document(uri, "rust", "after").await.unwrap();

    assert_eq!(
        receive_message(&receiver),
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {"uri": uri, "version": 2},
                "contentChanges": [{"text": "after"}]
            }
        })
    );
    let state = service.documents.lock().get(uri).cloned().unwrap();
    assert_eq!(state.version, 2);
    assert_eq!(state.text, "after");
}

#[tokio::test]
async fn send_request_reports_a_closed_transport_and_cleans_pending_state() {
    let (service, receiver) = service_with_transport();
    drop(receiver);

    let error =
        service.send_request("test/closed", json!({}), Duration::from_secs(1)).await.unwrap_err();

    assert_eq!(error.to_string(), "LSP transport is closed");
    assert!(service.pending.lock().is_empty());
}

#[tokio::test]
async fn send_request_times_out_and_cleans_pending_state() {
    let (service, receiver) = service_with_transport();

    let error = service
        .send_request("test/timeout", json!({"value": 1}), Duration::ZERO)
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "LSP request timed out for test/timeout");
    assert!(service.pending.lock().is_empty());
    assert_eq!(
        receive_message(&receiver),
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "test/timeout",
            "params": {"value": 1}
        })
    );
}

#[tokio::test]
async fn send_request_returns_the_matching_response() {
    let (service, receiver) = service_with_transport();
    let service = Arc::new(service);
    let request_service = service.clone();
    let request = tokio::spawn(async move {
        request_service
            .send_request("test/success", json!({"value": 2}), Duration::from_secs(1))
            .await
    });
    while service.pending.lock().is_empty() {
        tokio::task::yield_now().await;
    }

    service.handle_message(json!({"id": 1, "result": {"accepted": true}}));

    assert_eq!(request.await.unwrap().unwrap(), json!({"accepted": true}));
    assert_eq!(
        receive_message(&receiver),
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "test/success",
            "params": {"value": 2}
        })
    );
}

#[tokio::test]
async fn send_request_reports_when_the_response_channel_closes() {
    let (service, _receiver) = service_with_transport();
    let service = Arc::new(service);
    let request_service = service.clone();
    let request = tokio::spawn(async move {
        request_service
            .send_request("test/closed-response", json!({}), Duration::from_secs(1))
            .await
    });
    while service.pending.lock().is_empty() {
        tokio::task::yield_now().await;
    }

    service.pending.lock().remove(&1);

    let error = request.await.unwrap().unwrap_err();
    assert_eq!(error.to_string(), "LSP response channel closed for test/closed-response");
}

#[test]
fn handle_message_delivers_a_successful_response_to_the_pending_request() {
    let (service, _receiver) = service_with_transport();
    let (sender, mut receiver) = oneshot::channel();
    service.pending.lock().insert(41, sender);

    service.handle_message(json!({"id": 41, "result": {"value": true}}));

    assert_eq!(receiver.try_recv().unwrap().unwrap(), json!({"value": true}));
    assert!(service.pending.lock().is_empty());
}

#[test]
fn handle_message_defaults_a_missing_result_to_null() {
    let (service, _receiver) = service_with_transport();
    let (sender, mut receiver) = oneshot::channel();
    service.pending.lock().insert(42, sender);

    service.handle_message(json!({"id": 42}));

    assert_eq!(receiver.try_recv().unwrap().unwrap(), Value::Null);
}

#[test]
fn handle_message_delivers_a_formatted_protocol_error() {
    let (service, _receiver) = service_with_transport();
    let (sender, mut receiver) = oneshot::channel();
    service.pending.lock().insert(43, sender);

    service.handle_message(json!({
        "id": 43,
        "error": {"code": -32601, "message": "Method not found"}
    }));

    let error = receiver.try_recv().unwrap().unwrap_err();
    assert_eq!(error.to_string(), "Method not found (code -32601)");
}

#[test]
fn handle_message_ignores_notifications_and_unknown_response_ids() {
    let (service, _receiver) = service_with_transport();
    let (sender, _response) = oneshot::channel();
    service.pending.lock().insert(44, sender);

    service.handle_message(json!({"method": "window/logMessage", "params": {}}));
    service.handle_message(json!({"id": 99, "result": true}));

    assert!(service.pending.lock().contains_key(&44));
}

#[test]
fn handle_message_replies_with_null_to_an_unsupported_server_request() {
    let (service, receiver) = service_with_transport();

    service.handle_message(json!({
        "jsonrpc": "2.0",
        "id": 45,
        "method": "window/showMessageRequest",
        "params": {"message": "choose"}
    }));

    assert_eq!(receive_message(&receiver), json!({"jsonrpc": "2.0", "id": 45, "result": null}));

    service.handle_message(json!({
        "id": "opaque",
        "method": "window/showMessageRequest"
    }));
    assert!(matches!(receiver.try_recv(), Err(mpsc::TryRecvError::Empty)));
}

#[test]
fn read_lsp_message_parses_a_framed_json_response() {
    let expected = json!({"jsonrpc": "2.0", "id": 7, "result": {"name": "symbol"}});
    let input = framed_message(&expected);
    let mut reader = BufReader::new(Cursor::new(input));

    assert_eq!(read_lsp_message(&mut reader), Some(expected));
}

#[test]
fn read_lsp_message_consumes_consecutive_frames() {
    let first = json!({"jsonrpc": "2.0", "id": 1, "result": null});
    let second = json!({"jsonrpc": "2.0", "id": 2, "error": {"code": -1}});
    let mut input = framed_message(&first);
    input.extend(framed_message(&second));
    let mut reader = BufReader::new(Cursor::new(input));

    assert_eq!(read_lsp_message(&mut reader), Some(first));
    assert_eq!(read_lsp_message(&mut reader), Some(second));
    assert_eq!(read_lsp_message(&mut reader), None);
}

#[test]
fn read_lsp_message_accepts_other_headers_around_content_length() {
    let body = br#"{"result":true}"#;
    let input = format!(
        "Content-Type: application/vscode-jsonrpc; charset=utf-8\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    let mut bytes = input.into_bytes();
    bytes.extend(body);
    let mut reader = BufReader::new(Cursor::new(bytes));

    assert_eq!(read_lsp_message(&mut reader), Some(json!({"result": true})));
}

#[test]
fn read_lsp_message_rejects_missing_or_invalid_content_length() {
    let inputs: [&[u8]; 2] =
        [b"Content-Type: application/json\r\n\r\n{}", b"Content-Length: invalid\r\n\r\n{}"];

    for input in inputs {
        let mut reader = BufReader::new(Cursor::new(input));
        assert_eq!(read_lsp_message(&mut reader), None);
    }
}

#[test]
fn read_lsp_message_rejects_truncated_or_invalid_json_bodies() {
    let inputs: [&[u8]; 2] = [b"Content-Length: 5\r\n\r\n{}", b"Content-Length: 4\r\n\r\nnope"];

    for input in inputs {
        let mut reader = BufReader::new(Cursor::new(input));
        assert_eq!(read_lsp_message(&mut reader), None);
    }
}

#[test]
fn read_lsp_message_returns_none_when_headers_end_at_eof() {
    let mut reader = BufReader::new(Cursor::new(b"Content-Length: 2\r\n"));

    assert_eq!(read_lsp_message(&mut reader), None);
}

#[test]
fn format_lsp_error_uses_available_code_and_message_fields() {
    assert_eq!(
        format_lsp_error(&json!({"code": -32601, "message": "Method not found"})),
        "Method not found (code -32601)"
    );
    assert_eq!(format_lsp_error(&json!({"message": "Unavailable"})), "Unavailable");
    assert_eq!(format_lsp_error(&json!({"code": 9})), "unknown LSP error (code 9)");
    assert_eq!(format_lsp_error(&json!({})), "unknown LSP error");
}

#[test]
fn resolve_root_dir_keeps_files_inside_the_workspace_at_workspace_root() {
    let workspace = Path::new("/vibe-window/workspace");
    let file = Path::new("/vibe-window/workspace/src/main.rs");

    assert_eq!(resolve_root_dir(workspace, file), workspace);
}

#[test]
fn resolve_root_dir_uses_parent_for_files_outside_the_workspace() {
    let workspace = Path::new("/vibe-window/workspace");
    let file = Path::new("/vibe-window/external/src/main.rs");

    assert_eq!(resolve_root_dir(workspace, file), Path::new("/vibe-window/external/src"));
}

#[test]
fn resolve_root_dir_does_not_treat_a_prefix_sibling_as_inside_workspace() {
    let workspace = Path::new("/vibe-window/workspace");
    let file = Path::new("/vibe-window/workspace-copy/main.rs");

    assert_eq!(resolve_root_dir(workspace, file), Path::new("/vibe-window/workspace-copy"));
}

#[test]
fn resolve_root_dir_uses_workspace_when_file_has_no_parent() {
    let workspace = Path::new("/vibe-window/workspace");

    assert_eq!(resolve_root_dir(workspace, Path::new("/")), workspace);
}

#[test]
fn path_to_file_uri_encodes_spaces_in_the_path() {
    assert_eq!(
        path_to_file_uri(Path::new("/vibe-window/my workspace/main.rs")),
        "file:///vibe-window/my%20workspace/main.rs"
    );
}

#[test]
fn path_to_file_uri_preserves_non_space_characters() {
    assert_eq!(
        path_to_file_uri(Path::new("/vibe-window/src/main.rs")),
        "file:///vibe-window/src/main.rs"
    );
}
