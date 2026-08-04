#![cfg(not(target_arch = "wasm32"))]

use std::fs;

use serde_json::json;
use tempfile::TempDir;

use super::{
    ChatMessage, ChatRole, ChatSession, load_llm_raw_step, persist_llm_raw_step,
    save_session_step_snapshot,
};

fn sample_session(id: &str) -> ChatSession {
    ChatSession {
        id: id.to_owned(),
        title: "Snapshot".to_owned(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "hello".to_owned(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }],
        message_ids: vec![Some("message-1".to_owned())],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    }
}

#[test]
fn save_snapshot_and_raw_payload_write_pretty_json() {
    let temp = TempDir::new().unwrap();
    let session = sample_session("snap");

    let snapshot =
        save_session_step_snapshot(temp.path(), &session, 2, "finish", Some("repo")).unwrap();
    let raw =
        persist_llm_raw_step(temp.path(), "snap", 2, &json!({"ok": true}), Some("repo")).unwrap();

    assert!(snapshot.ends_with("session-snap-step-2-finish.json"));
    assert!(raw.ends_with("session-snap-step-2-llm_raw.json"));
    assert!(fs::read_to_string(snapshot).unwrap().contains("\n  \"id\": \"snap\""));
    assert!(fs::read_to_string(raw).unwrap().contains("\n  \"ok\": true"));
    assert_eq!(
        load_llm_raw_step(temp.path(), "snap", 2, Some("repo")).unwrap().unwrap()["ok"],
        true
    );
    assert!(load_llm_raw_step(temp.path(), "snap", 3, Some("repo")).unwrap().is_none());
}

#[test]
fn load_llm_raw_step_reports_invalid_json() {
    let temp = TempDir::new().unwrap();
    let raw =
        persist_llm_raw_step(temp.path(), "raw", 0, &json!({"ok": true}), Some("repo")).unwrap();
    fs::write(raw, "not-json").unwrap();

    assert!(load_llm_raw_step(temp.path(), "raw", 0, Some("repo")).is_err());
}

#[test]
fn load_llm_raw_step_reports_non_utf8_payload() {
    let temp = TempDir::new().unwrap();
    let raw =
        persist_llm_raw_step(temp.path(), "raw", 1, &json!({"ok": true}), Some("repo")).unwrap();
    fs::write(raw, [0xff]).unwrap();

    let error = load_llm_raw_step(temp.path(), "raw", 1, Some("repo")).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}
