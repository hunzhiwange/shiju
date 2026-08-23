//! 验证待办事项更新的稀疏字段与稳定状态名称。

use crate::todo::{TodoPriority, TodoStatus, UpdateTodoRequest};
use serde_json::json;

#[test]
fn todo_update_is_sparse_and_status_names_are_stable() {
    let update: UpdateTodoRequest = serde_json::from_value(json!({})).expect("valid update");
    assert_eq!(update.content, None);
    assert_eq!(update.status, None);
    assert_eq!(update.priority, None);

    assert_eq!(
        serde_json::to_value(TodoStatus::InProgress).expect("serialize"),
        json!("in_progress")
    );
    assert_eq!(serde_json::to_value(TodoPriority::High).expect("serialize"), json!("high"));
}
