use serde_json::json;

use super::todo_parse::todowrite_display_todos;

#[test]
fn status_patch_displays_content_from_structured_result() {
    let value = json!({
        "status": "completed",
        "input": json!({"merge": true, "todos": [{"id": 1, "status": "completed"}]}).to_string(),
        "result": {"data": {"newTodos": [
            {"id": "other", "content": "未更新的任务", "status": "pending"},
            {"id": "1", "content": "检查任务显示", "status": "completed"}
        ]}}
    });

    assert_eq!(
        todowrite_display_todos(&value),
        vec![json!({
            "id": 1, "status": "completed", "content": "检查任务显示"
        })]
    );
}

#[test]
fn status_patch_displays_content_from_legacy_output() {
    let value = json!({
        "status": "success",
        "input": json!({"merge": true, "todos": [{"id": "a", "status": "in_progress"}]}).to_string(),
        "output": json!([{"id": "a", "content": "实现修复", "status": "in_progress"}]).to_string()
    });

    assert_eq!(todowrite_display_todos(&value)[0]["content"], "实现修复");
}

#[test]
fn failed_or_unmatched_patch_does_not_borrow_unrelated_content() {
    let mut value = json!({
        "status": "failed",
        "input": json!({"merge": true, "todos": [{"id": "a", "status": "completed"}]}).to_string(),
        "result": {"data": {"newTodos": [{"id": "a", "content": "旧内容"}]}}
    });
    assert!(todowrite_display_todos(&value)[0].get("content").is_none());
    value["status"] = json!("completed");
    value["result"]["data"]["newTodos"][0]["id"] = json!("b");
    assert!(todowrite_display_todos(&value)[0].get("content").is_none());
}

#[test]
fn content_patch_preserves_input_and_uses_content_match_before_id() {
    let value = json!({
        "status": "completed",
        "input": json!({"merge": true, "todos": [{"id": "a", "content": "目标任务"}]}).to_string(),
        "result": {"data": {"newTodos": [
            {"id": "a", "content": "另一任务", "status": "pending"},
            {"id": "b", "content": "目标任务", "status": "in_progress"}
        ]}}
    });
    assert_eq!(todowrite_display_todos(&value)[0]["status"], "in_progress");
    assert_eq!(todowrite_display_todos(&value)[0]["content"], "目标任务");
}
