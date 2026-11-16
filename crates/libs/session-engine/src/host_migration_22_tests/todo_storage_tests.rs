//! Todo read policy through the same real session storage used by the host.
use todo_types as todo;
fn unique_session(prefix: &str) -> String { format!("{prefix}-{}", uuid::Uuid::new_v4()) }
fn ctx(session: &str) -> String { session.to_string() }
fn item(id: &str, content: &str, status: &str) -> todo::Todo {
    todo::Todo { id: id.into(), content: content.into(), status: status.into(), priority: "medium".into() }
}
fn seed(session: &str, items: Vec<todo::Todo>) {
    tool_services::todo::write_for_session(&serde_json::json!({"todos":items}).to_string(), session).expect("seed todos");
}
#[test]
fn has_incomplete_todos_false_for_empty_or_all_completed_and_true_for_pending() {
    let session = unique_session("todos-incomplete");
    let ctx = ctx(&session);

    assert!(!crate::runner_todos::has_incomplete_todos(&ctx));

    seed(&ctx, vec![item("1", "done", "completed")]);
    assert!(!crate::runner_todos::has_incomplete_todos(&ctx));

    seed(&ctx, vec![item("1", "done", "completed"), item("2", "todo", "pending")]);
    assert!(crate::runner_todos::has_incomplete_todos(&ctx));
}

#[test]
// 取消状态不等于完成，仍应阻止会话错误宣告全部完成。
fn has_incomplete_todos_treats_cancelled_as_incomplete() {
    let session = unique_session("todos-cancelled");
    let ctx = ctx(&session);
    seed(&ctx, vec![item("1", "cancelled", "cancelled")]);

    assert!(crate::runner_todos::has_incomplete_todos(&ctx));
}

#[test]
fn read_todos_or_empty_returns_current_items_or_empty_list() {
    let empty_session = unique_session("todos-empty");
    assert!(crate::runner_todos::read_todos_or_empty(&ctx(&empty_session)).is_empty());

    let session = unique_session("todos-read");
    let ctx = ctx(&session);
    seed(&ctx, vec![item("1", "alpha", "pending"), item("2", "beta", "in_progress")]);

    let items = crate::runner_todos::read_todos_or_empty(&ctx);

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].content, "alpha");
    assert_eq!(items[1].status, "in_progress");
}
