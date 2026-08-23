//! 待办事项兼容反序列化与默认值测试。
//!
//! 覆盖字符串或数字 ID、可选 ID、旧结构兼容以及非法 JSON 类型拒绝。

use serde::Deserialize;

#[test]
fn todo_defaults_are_applied_when_fields_are_missing() {
    let todo: super::Todo = serde_json::from_str(r#"{"content":"ship","id":7}"#).unwrap();

    assert_eq!(todo.content, "ship");
    assert_eq!(todo.id, "7");
    assert_eq!(todo.status, "pending");
    assert_eq!(todo.priority, "medium");
}

#[test]
fn todo_id_accepts_string_or_number_only() {
    let string_id: super::Todo =
        serde_json::from_str(r#"{"content":"ship","id":"task-1"}"#).unwrap();
    let number_id: super::Todo = serde_json::from_str(r#"{"content":"ship","id":42}"#).unwrap();
    let invalid = serde_json::from_str::<super::Todo>(r#"{"content":"ship","id":true}"#);

    assert_eq!(string_id.id, "task-1");
    assert_eq!(number_id.id, "42");
    assert!(invalid.is_err());
}

#[test]
fn todo_preserves_explicit_status_and_priority() {
    let todo: super::Todo = serde_json::from_str(
        r#"{"content":"ship","id":"task-1","status":"done","priority":"high"}"#,
    )
    .unwrap();

    assert_eq!(todo.status, "done");
    assert_eq!(todo.priority, "high");
}

#[test]
fn legacy_todo_matches_todo_shape() {
    let todo: super::LegacyTodo = serde_json::from_str(r#"{"content":"ship","id":17}"#).unwrap();

    assert_eq!(todo.content, "ship");
    assert_eq!(todo.id, "17");
    assert_eq!(todo.status, "pending");
    assert_eq!(todo.priority, "medium");
}

#[derive(Deserialize)]
struct OptionalId {
    #[serde(deserialize_with = "super::de_opt_string_or_number")]
    id: Option<String>,
}

#[test]
fn optional_id_accepts_null_string_or_number() {
    let null_id: OptionalId = serde_json::from_str(r#"{"id":null}"#).unwrap();
    let string_id: OptionalId = serde_json::from_str(r#"{"id":"task-1"}"#).unwrap();
    let number_id: OptionalId = serde_json::from_str(r#"{"id":42}"#).unwrap();

    assert_eq!(null_id.id, None);
    assert_eq!(string_id.id.as_deref(), Some("task-1"));
    assert_eq!(number_id.id.as_deref(), Some("42"));
}

#[test]
fn optional_id_rejects_other_json_types() {
    let invalid_bool = serde_json::from_str::<OptionalId>(r#"{"id":true}"#);
    let invalid_array = serde_json::from_str::<OptionalId>(r#"{"id":[]}"#);

    assert!(invalid_bool.is_err());
    assert!(invalid_array.is_err());
}

#[test]
fn default_helpers_return_stable_values() {
    assert_eq!(super::default_todo_status(), "pending");
    assert_eq!(super::default_todo_priority(), "medium");
}
