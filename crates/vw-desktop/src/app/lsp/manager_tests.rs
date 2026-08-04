#[test]
fn manager_tests_module_is_wired() {
    assert!(module_path!().ends_with("manager_tests"));
}

use iced_code_editor::{LspClient, LspDocument, LspEvent, LspPosition, LspRange, LspTextChange};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex, mpsc};

type TestClientParts = (
    super::SharedLspClient,
    mpsc::Receiver<Vec<u8>>,
    Arc<Mutex<HashMap<String, super::DocumentState>>>,
    Arc<Mutex<HashMap<u64, super::LspRequestKind>>>,
);

fn test_client() -> TestClientParts {
    let (writer, receiver) = mpsc::channel();
    let documents = Arc::new(Mutex::new(HashMap::new()));
    let pending_requests = Arc::new(Mutex::new(HashMap::new()));
    let service = Arc::new(super::SharedLspService {
        server_key: "test-server".to_string(),
        writer,
        documents: documents.clone(),
        request_id: AtomicU64::new(1),
        pending_requests: pending_requests.clone(),
    });

    (super::SharedLspClient { service }, receiver, documents, pending_requests)
}

fn receive_json(receiver: &mpsc::Receiver<Vec<u8>>) -> serde_json::Value {
    let frame = receiver.try_recv().unwrap();
    let separator = frame.windows(4).position(|window| window == b"\r\n\r\n").unwrap();
    let header = std::str::from_utf8(&frame[..separator]).unwrap();
    let body = &frame[separator + 4..];
    assert_eq!(header, format!("Content-Length: {}", body.len()));
    serde_json::from_slice(body).unwrap()
}

#[test]
fn text_model_tracks_empty_text_and_utf16_columns() {
    let model = super::TextModel::from_text("");
    assert_eq!(model.lines, vec!["".to_string()]);

    let model = super::TextModel::from_text("a😀b\nsecond");
    assert_eq!(model.to_utf16_position(LspPosition { line: 0, character: 3 }).character, 4);
    assert_eq!(model.to_utf16_position(LspPosition { line: 8, character: 3 }).character, 3);
}

#[test]
fn text_model_applies_single_and_multiline_changes() {
    let mut model = super::TextModel::from_text("hello\nworld");
    model.apply_change(&LspTextChange {
        range: LspRange {
            start: LspPosition { line: 0, character: 1 },
            end: LspPosition { line: 0, character: 4 },
        },
        text: "i".to_string(),
    });
    assert_eq!(model.lines, vec!["hio".to_string(), "world".to_string()]);

    model.apply_change(&LspTextChange {
        range: LspRange {
            start: LspPosition { line: 0, character: 2 },
            end: LspPosition { line: 1, character: 3 },
        },
        text: "!\nnew".to_string(),
    });
    assert_eq!(model.lines, vec!["hi!".to_string(), "newld".to_string()]);
}

#[test]
fn text_model_ignores_changes_outside_existing_lines() {
    let mut model = super::TextModel::from_text("first\nsecond");
    let original = model.lines.clone();

    model.apply_change(&LspTextChange {
        range: LspRange {
            start: LspPosition { line: 2, character: 0 },
            end: LspPosition { line: 2, character: 0 },
        },
        text: "ignored".to_string(),
    });

    assert_eq!(model.lines, original);
}

#[test]
fn text_model_clamps_character_offsets_to_line_end() {
    let mut model = super::TextModel::from_text("abc");

    model.apply_change(&LspTextChange {
        range: LspRange {
            start: LspPosition { line: 0, character: 99 },
            end: LspPosition { line: 0, character: 100 },
        },
        text: "!".to_string(),
    });

    assert_eq!(model.lines, vec!["abc!".to_string()]);
}

#[test]
fn char_to_byte_index_clamps_past_end_and_handles_multibyte() {
    assert_eq!(super::char_to_byte_index("a😀b", 2), 5);
    assert_eq!(super::char_to_byte_index("a😀b", 99), "a😀b".len());
}

#[test]
fn parse_hover_text_supports_strings_arrays_and_marked_strings() {
    assert_eq!(super::parse_hover_text(&json!({"contents": "plain"})), Some("plain".to_string()));
    assert_eq!(
        super::parse_hover_text(&json!({"contents": [{"value": "one"}, "two"]})),
        Some("one\ntwo".to_string())
    );
    assert_eq!(super::parse_hover_text(&json!({"contents": []})), None);
    assert_eq!(super::parse_hover_text(&json!({})), None);
    assert_eq!(super::parse_hover_text(&json!({"contents": 7})), None);
}

#[test]
fn parse_completion_items_accepts_array_or_list_shape() {
    assert_eq!(
        super::parse_completion_items(&json!([{"label": "alpha"}, {"label": 2}])),
        vec!["alpha".to_string()]
    );
    assert_eq!(
        super::parse_completion_items(&json!({"items": [{"label": "beta"}]})),
        vec!["beta".to_string()]
    );
    assert!(super::parse_completion_items(&json!({"items": "invalid"})).is_empty());
}

#[test]
fn parse_definition_location_accepts_location_and_link_shapes() {
    let location = json!({
        "uri": "file:///a.rs",
        "range": {"start": {"line": 1, "character": 2}, "end": {"line": 3, "character": 4}}
    });
    let (uri, range) = super::parse_definition_location(&location).unwrap();
    assert_eq!(uri, "file:///a.rs");
    assert_eq!(range.start.line, 1);
    assert_eq!(range.end.character, 4);

    let link = json!([{
        "targetUri": "file:///b.rs",
        "targetSelectionRange": {
            "start": {"line": 5, "character": 6},
            "end": {"line": 7, "character": 8}
        }
    }]);
    let (uri, range) = super::parse_definition_location(&link).unwrap();
    assert_eq!(uri, "file:///b.rs");
    assert_eq!(range.start.character, 6);
}

#[test]
fn parse_definition_link_falls_back_to_target_range() {
    let link = json!([{
        "targetUri": "file:///fallback.rs",
        "targetRange": {
            "start": {"line": 9, "character": 10},
            "end": {"line": 11, "character": 12}
        }
    }]);

    let (uri, range) = super::parse_definition_location(&link).unwrap();

    assert_eq!(uri, "file:///fallback.rs");
    assert_eq!(range.start.line, 9);
    assert_eq!(range.start.character, 10);
    assert_eq!(range.end.line, 11);
    assert_eq!(range.end.character, 12);
}

#[test]
fn parse_definition_location_rejects_empty_or_incomplete_shapes() {
    assert!(super::parse_definition_location(&json!([])).is_none());
    assert!(super::parse_definition_location(&json!(null)).is_none());
    assert!(
        super::parse_definition_location(&json!({
            "uri": "file:///missing-range.rs"
        }))
        .is_none()
    );
    assert!(
        super::parse_definition_location(&json!([{
            "targetUri": "file:///missing-selection.rs"
        }]))
        .is_none()
    );
    assert!(
        super::parse_definition_location(&json!({
            "uri": "file:///bad-range.rs",
            "range": {
                "start": {"line": "0", "character": 1},
                "end": {"line": 0, "character": 2}
            }
        }))
        .is_none()
    );
}

#[test]
fn handle_server_request_acknowledges_progress_create_only() {
    let (tx, rx) = mpsc::channel();
    super::handle_server_request(7, "window/workDoneProgress/create", &tx);
    let payload = String::from_utf8(rx.try_recv().unwrap()).unwrap();
    assert!(payload.starts_with("Content-Length:"));
    assert!(payload.contains("\"id\":7"));

    super::handle_server_request(8, "workspace/configuration", &tx);
    assert!(rx.try_recv().is_err());
}

#[test]
fn handle_client_response_emits_matching_events_and_clears_pending() {
    let pending = Arc::new(Mutex::new(HashMap::from([
        (1, super::LspRequestKind::Hover),
        (2, super::LspRequestKind::Completion),
        (3, super::LspRequestKind::Definition),
    ])));
    let (tx, rx) = mpsc::channel();

    super::handle_client_response(1, &json!({"result": {"contents": "hover"}}), &pending, &tx);
    super::handle_client_response(2, &json!({"result": [{"label": "item"}]}), &pending, &tx);
    super::handle_client_response(
        3,
        &json!({"result": {"uri": "file:///main.rs", "range": {
            "start": {"line": 0, "character": 1},
            "end": {"line": 0, "character": 2}
        }}}),
        &pending,
        &tx,
    );

    assert!(matches!(rx.try_recv().unwrap(), LspEvent::Hover { text } if text == "hover"));
    assert!(
        matches!(rx.try_recv().unwrap(), LspEvent::Completion { items } if items == vec!["item"])
    );
    assert!(
        matches!(rx.try_recv().unwrap(), LspEvent::Definition { uri, .. } if uri == "file:///main.rs")
    );
    assert!(pending.lock().unwrap().is_empty());
}

#[test]
fn handle_client_response_suppresses_unknown_and_empty_results() {
    let pending = Arc::new(Mutex::new(HashMap::from([
        (1, super::LspRequestKind::Completion),
        (2, super::LspRequestKind::Definition),
    ])));
    let (tx, rx) = mpsc::channel();

    super::handle_client_response(99, &json!({"result": {"contents": "ignored"}}), &pending, &tx);
    super::handle_client_response(1, &json!({"result": []}), &pending, &tx);
    super::handle_client_response(2, &json!({"result": null}), &pending, &tx);

    assert!(rx.try_recv().is_err());
    assert!(pending.lock().unwrap().is_empty());
}

#[test]
fn handle_client_response_emits_empty_hover_when_result_is_missing() {
    let pending = Arc::new(Mutex::new(HashMap::from([(1, super::LspRequestKind::Hover)])));
    let (tx, rx) = mpsc::channel();

    super::handle_client_response(1, &json!({"error": {"code": -1}}), &pending, &tx);

    assert!(matches!(rx.try_recv().unwrap(), LspEvent::Hover { text } if text.is_empty()));
}

#[test]
fn handle_server_notification_emits_progress_for_numeric_tokens() {
    let (tx, rx) = mpsc::channel();

    super::handle_server_notification(
        "$/progress",
        &json!({"token": 9, "value": {
            "kind": "end",
            "title": "index",
            "message": "done",
            "percentage": 100
        }}),
        &tx,
        "rust-analyzer",
    );

    assert!(matches!(
        rx.try_recv().unwrap(),
        LspEvent::Progress { token, server_key, title, message, percentage, done }
            if token == "9"
                && server_key == "rust-analyzer"
                && title == "index"
                && message.as_deref() == Some("done")
                && percentage == Some(100)
                && done
    ));

    super::handle_server_notification("other", &json!({"token": "x", "value": {}}), &tx, "server");
    assert!(rx.try_recv().is_err());
}

#[test]
fn handle_server_notification_accepts_string_token_and_optional_fields() {
    let (tx, rx) = mpsc::channel();

    super::handle_server_notification(
        "$/progress",
        &json!({"token": "build", "value": {"kind": "report"}}),
        &tx,
        "server",
    );

    assert!(matches!(
        rx.try_recv().unwrap(),
        LspEvent::Progress { token, title, message, percentage, done, .. }
            if token == "build"
                && title.is_empty()
                && message.is_none()
                && percentage.is_none()
                && !done
    ));
}

#[test]
fn handle_server_notification_rejects_missing_token_or_value() {
    let (tx, rx) = mpsc::channel();

    super::handle_server_notification("$/progress", &json!({"value": {}}), &tx, "server");
    super::handle_server_notification(
        "$/progress",
        &json!({"token": true, "value": {}}),
        &tx,
        "server",
    );
    super::handle_server_notification("$/progress", &json!({"token": "x"}), &tx, "server");

    assert!(rx.try_recv().is_err());
}

#[test]
fn shared_service_frames_messages_and_allocates_monotonic_ids() {
    let (client, receiver, _, _) = test_client();

    assert_eq!(client.service.next_id(), 1);
    assert_eq!(client.service.next_id(), 2);
    client.service.send_message(&json!({"jsonrpc": "2.0", "method": "test"}));

    assert_eq!(receive_json(&receiver)["method"], "test");
}

#[test]
fn shared_client_tracks_open_save_and_close_lifecycle() {
    let (mut client, receiver, documents, _) = test_client();
    let document = LspDocument {
        uri: "file:///main.rs".to_string(),
        language_id: "rust".to_string(),
        version: 4,
    };

    client.did_open(&document, "fn main() {}");
    assert!(documents.lock().unwrap().contains_key(&document.uri));
    let opened = receive_json(&receiver);
    assert_eq!(opened["method"], "textDocument/didOpen");
    assert_eq!(opened["params"]["textDocument"]["version"], 4);

    client.did_save(&document, "fn main() { println!(); }");
    let saved = receive_json(&receiver);
    assert_eq!(saved["method"], "textDocument/didSave");
    assert_eq!(saved["params"]["text"], "fn main() { println!(); }");

    client.did_close(&document);
    assert!(!documents.lock().unwrap().contains_key(&document.uri));
    assert_eq!(receive_json(&receiver)["method"], "textDocument/didClose");
}

#[test]
fn shared_client_converts_changes_to_utf16_and_updates_document() {
    let (mut client, receiver, documents, _) = test_client();
    let mut document = LspDocument::new("file:///unicode.rs", "rust");
    client.did_open(&document, "a😀b");
    let _ = receive_json(&receiver);
    document.version = 1;

    client.did_change(
        &document,
        &[LspTextChange {
            range: LspRange {
                start: LspPosition { line: 0, character: 1 },
                end: LspPosition { line: 0, character: 2 },
            },
            text: "x".to_string(),
        }],
    );

    let changed = receive_json(&receiver);
    assert_eq!(changed["method"], "textDocument/didChange");
    assert_eq!(changed["params"]["contentChanges"][0]["range"]["start"]["character"], 1);
    assert_eq!(changed["params"]["contentChanges"][0]["range"]["end"]["character"], 3);
    assert_eq!(documents.lock().unwrap()[&document.uri].text.lines, vec!["axb".to_string()]);
}

#[test]
fn shared_service_applies_multiple_changes_in_order_and_skips_unknown_uri() {
    let (client, _receiver, documents, _) = test_client();
    let uri = "file:///ordered.rs";
    documents.lock().unwrap().insert(
        uri.to_string(),
        super::DocumentState { text: super::TextModel::from_text("alpha\nbeta") },
    );

    assert!(client.service.apply_change_and_convert("file:///missing.rs", &[]).is_empty());

    let content_changes = client.service.apply_change_and_convert(
        uri,
        &[
            LspTextChange {
                range: LspRange {
                    start: LspPosition { line: 0, character: 0 },
                    end: LspPosition { line: 0, character: 5 },
                },
                text: "one".to_string(),
            },
            LspTextChange {
                range: LspRange {
                    start: LspPosition { line: 1, character: 0 },
                    end: LspPosition { line: 1, character: 4 },
                },
                text: "two".to_string(),
            },
        ],
    );

    assert_eq!(content_changes.len(), 2);
    assert_eq!(
        documents.lock().unwrap()[uri].text.lines,
        vec!["one".to_string(), "two".to_string()]
    );
}

#[test]
fn shared_client_ignores_changes_and_requests_for_unknown_documents() {
    let (mut client, receiver, _, pending) = test_client();
    let document = LspDocument::new("file:///missing.rs", "rust");
    let change = LspTextChange {
        range: LspRange {
            start: LspPosition { line: 0, character: 0 },
            end: LspPosition { line: 0, character: 0 },
        },
        text: "ignored".to_string(),
    };
    let position = LspPosition { line: 0, character: 0 };

    client.did_change(&document, &[change]);
    client.request_hover(&document, position);
    client.request_completion(&document, position);
    client.request_definition(&document, position);

    assert!(receiver.try_recv().is_err());
    assert!(pending.lock().unwrap().is_empty());
}

#[test]
fn shared_client_queues_hover_completion_and_definition_requests() {
    let (mut client, receiver, _, pending) = test_client();
    let document = LspDocument::new("file:///unicode.rs", "rust");
    client.did_open(&document, "a😀b");
    let _ = receive_json(&receiver);
    let position = LspPosition { line: 0, character: 2 };

    client.request_hover(&document, position);
    client.request_completion(&document, position);
    client.request_definition(&document, position);

    let hover = receive_json(&receiver);
    let completion = receive_json(&receiver);
    let definition = receive_json(&receiver);
    assert_eq!(hover["id"], 1);
    assert_eq!(hover["method"], "textDocument/hover");
    assert_eq!(hover["params"]["position"]["character"], 3);
    assert_eq!(completion["id"], 2);
    assert_eq!(completion["method"], "textDocument/completion");
    assert_eq!(completion["params"]["context"]["triggerKind"], 1);
    assert_eq!(definition["id"], 3);
    assert_eq!(definition["method"], "textDocument/definition");

    let pending = pending.lock().unwrap();
    assert!(matches!(pending.get(&1), Some(super::LspRequestKind::Hover)));
    assert!(matches!(pending.get(&2), Some(super::LspRequestKind::Completion)));
    assert!(matches!(pending.get(&3), Some(super::LspRequestKind::Definition)));
}

#[test]
fn manager_initializes_empty_and_rejects_unsupported_server_without_spawning() {
    let (tx, _rx) = mpsc::channel();
    let mut manager = super::LspServiceManager::new(tx);

    assert!(manager.services.is_empty());
    assert!(manager.prestarted_projects.is_empty());
    assert!(manager.get_or_create("unsupported-test-server", "file:///tmp").is_err());
    assert!(manager.services.is_empty());
}

#[test]
fn lsp_service_key_scopes_server_to_workspace_root() {
    let first = super::LspServiceKey::new("rust-analyzer", "file:///workspace/one");
    let same = super::LspServiceKey::new("rust-analyzer", "file:///workspace/one");
    let other_root = super::LspServiceKey::new("rust-analyzer", "file:///workspace/two");
    let other_server =
        super::LspServiceKey::new("typescript-language-server", "file:///workspace/one");

    assert_eq!(first, same);
    assert_ne!(first, other_root);
    assert_ne!(first, other_server);
}

#[test]
fn manager_shutdown_all_is_idempotent_without_services() {
    let (tx, rx) = mpsc::channel();
    let mut manager = super::LspServiceManager::new(tx);

    manager.shutdown_all();
    manager.shutdown_all();

    assert!(manager.services.is_empty());
    assert!(rx.try_recv().is_err());
}

#[test]
fn manager_prestarts_empty_project_only_once_without_spawning() {
    let dir = tempfile::tempdir().unwrap();
    let project_path = dir.path().to_str().unwrap();
    let (tx, rx) = mpsc::channel();
    let mut manager = super::LspServiceManager::new(tx);

    manager.prestart_for_project(project_path);
    manager.prestart_for_project(project_path);

    assert_eq!(manager.prestarted_projects.len(), 1);
    assert!(manager.prestarted_projects.contains(project_path));
    assert!(manager.services.is_empty());
    assert!(rx.try_recv().is_err());
}

#[test]
fn detect_project_servers_returns_sorted_unique_server_keys() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname='x'\nversion='0.1.0'\n")
        .unwrap();
    std::fs::write(dir.path().join("src").join("main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(dir.path().join("index.ts"), "const x = 1;\n").unwrap();

    let servers = super::detect_project_servers(dir.path().to_str().unwrap());

    assert!(servers.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(servers.contains(&"rust-analyzer".to_string()));
}

#[test]
fn detect_project_servers_returns_empty_for_missing_or_directory_only_projects() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();

    assert!(super::detect_project_servers(dir.path().join("missing").to_str().unwrap()).is_empty());
    assert!(super::detect_project_servers(dir.path().to_str().unwrap()).is_empty());
}
