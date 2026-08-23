//! 验证待办写入与局部更新参数的反序列化默认值。

use super::schema::{TodoInput, TodoPatch, WriteArgs};
use serde_json::json;

#[test]
fn write_args_defaults_merge_to_false() {
    let args: WriteArgs = serde_json::from_value(json!({"todos": []})).expect("valid args");

    assert!(!args.merge);
}

#[test]
fn todo_input_defaults_status_priority_and_numeric_id() {
    let input: TodoInput =
        serde_json::from_value(json!({"id": 7, "content": "ship"})).expect("valid todo");

    assert_eq!(input.id.as_deref(), Some("7"));
    assert_eq!(input.status, "pending");
    assert_eq!(input.priority, "medium");
}

#[test]
fn todo_patch_accepts_partial_update() {
    let patch: TodoPatch =
        serde_json::from_value(json!({"id": 1, "status": "completed"})).expect("valid patch");

    assert_eq!(patch.id.as_deref(), Some("1"));
    assert_eq!(patch.content, None);
}
