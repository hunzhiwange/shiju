use super::store::{read_for_tool, read_for_ui, write_todos};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};

fn session(name: &str) -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("store-test-{name}-{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

fn parse(output: &str) -> Vec<Value> {
    serde_json::from_str(output).unwrap()
}

#[test]
fn read_empty_session_returns_empty_list() {
    let session = session("empty");
    assert!(read_for_ui(&session).is_empty());
    assert!(read_for_tool(&session).unwrap().is_empty());
}

#[test]
fn write_todos_replaces_and_normalizes_items() {
    let session = session("replace");
    let output = write_todos(
        &session,
        json!({"todos": [{"id": "", "content": " task ", "status": "pending"}]}),
    )
    .expect("write succeeds");

    assert!(output.contains("\"content\": \"task\""));
    let todos = read_for_tool(&session).expect("read succeeds");
    assert_eq!(todos[0].id, "1");
}

#[test]
fn write_todos_merge_updates_existing_item() {
    let session = session("merge");
    write_todos(&session, json!({"todos": [{"id": 1, "content": "task"}]})).expect("seed succeeds");
    write_todos(&session, json!({"merge": true, "todos": [{"id": 1, "status": "completed"}]}))
        .expect("merge succeeds");

    let todos = read_for_tool(&session).expect("read succeeds");
    assert_eq!(todos[0].status, "completed");
}

#[test]
fn merge_updates_by_content_and_prepends_new_items() {
    let session = session("merge-content");
    write_todos(
        &session,
        json!({"todos": [
            {"id": "a", "content": "Alpha", "status": "pending", "priority": "low"},
            {"id": "b", "content": "Beta", "status": "pending", "priority": "medium"}
        ]}),
    )
    .unwrap();

    let output = write_todos(
        &session,
        json!({"merge": true, "todos": [
            {"content": "Alpha", "status": "completed"},
            {"id": "c", "content": "Gamma"}
        ]}),
    )
    .unwrap();
    let todos = parse(&output);

    assert_eq!(todos.len(), 3);
    assert_eq!(todos[0]["content"], "Gamma");
    assert_eq!(todos[1]["content"], "Alpha");
    assert_eq!(todos[1]["status"], "completed");
}

#[test]
fn merge_ignores_patch_without_content_when_target_missing() {
    let session = session("missing-target");
    let output =
        write_todos(&session, json!({"merge": true, "todos": [{"id": "missing"}]})).unwrap();
    assert!(parse(&output).is_empty());
}

#[test]
fn merge_by_id_trims_updated_content_and_priority() {
    let session = session("merge-id-fields");
    write_todos(
        &session,
        json!({"todos": [{
            "id": "1",
            "content": "Original",
            "status": "pending",
            "priority": "low"
        }]}),
    )
    .unwrap();

    let output = write_todos(
        &session,
        json!({"merge": true, "todos": [{
            "id": " 1 ",
            "content": " Updated ",
            "priority": " high "
        }]}),
    )
    .unwrap();
    let todos = parse(&output);

    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0]["id"], "1");
    assert_eq!(todos[0]["content"], "Updated");
    assert_eq!(todos[0]["status"], "pending");
    assert_eq!(todos[0]["priority"], "high");
}

#[test]
fn malformed_arguments_return_errors() {
    let session = session("errors");
    assert!(write_todos(&session, json!({})).is_err());
    assert!(write_todos(&session, json!({"todos": [{"status": "pending"}]})).is_err());
    assert!(write_todos(&session, json!({"merge": true, "todos": [{"content": 1}]})).is_err());
}
