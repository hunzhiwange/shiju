//! 验证待办读取、未完成判定和状态补丁生成。

use crate::app::agent::session::session::{Role, Session};
use crate::app::agent::tools::{ToolRuntimeContext, todo};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SESSION: AtomicU64 = AtomicU64::new(0);

fn unique_session(prefix: &str) -> String {
    format!("{prefix}-{}-{}", std::process::id(), NEXT_SESSION.fetch_add(1, Ordering::Relaxed))
}

fn ctx(session: &str) -> ToolRuntimeContext {
    ToolRuntimeContext::new(session.to_string(), None)
}

fn item(id: &str, content: &str, status: &str) -> todo::Todo {
    todo::Todo {
        id: id.to_string(),
        content: content.to_string(),
        status: status.to_string(),
        priority: "medium".to_string(),
    }
}

fn seed(ctx: &ToolRuntimeContext, items: Vec<todo::Todo>) {
    let input = serde_json::json!({ "todos": items }).to_string();
    todo::write(&input, ctx).expect("seed todos");
}

fn read(ctx: &ToolRuntimeContext) -> Vec<todo::Todo> {
    serde_json::from_str(&todo::read(ctx).expect("read todos")).expect("todo json")
}

#[test]
fn has_incomplete_todos_false_for_empty_or_all_completed_and_true_for_pending() {
    let session = unique_session("todos-incomplete");
    let ctx = ctx(&session);

    assert!(!super::has_incomplete_todos(&ctx));

    seed(&ctx, vec![item("1", "done", "completed")]);
    assert!(!super::has_incomplete_todos(&ctx));

    seed(&ctx, vec![item("1", "done", "completed"), item("2", "todo", "pending")]);
    assert!(super::has_incomplete_todos(&ctx));
}

#[test]
// 取消状态不等于完成，仍应阻止会话错误宣告全部完成。
fn has_incomplete_todos_treats_cancelled_as_incomplete() {
    let session = unique_session("todos-cancelled");
    let ctx = ctx(&session);
    seed(&ctx, vec![item("1", "cancelled", "cancelled")]);

    assert!(super::has_incomplete_todos(&ctx));
}

#[test]
fn read_todos_or_empty_returns_current_items_or_empty_list() {
    let empty_session = unique_session("todos-empty");
    assert!(super::read_todos_or_empty(&ctx(&empty_session)).is_empty());

    let session = unique_session("todos-read");
    let ctx = ctx(&session);
    seed(&ctx, vec![item("1", "alpha", "pending"), item("2", "beta", "in_progress")]);

    let items = super::read_todos_or_empty(&ctx);

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].content, "alpha");
    assert_eq!(items[1].status, "in_progress");
}

#[test]
fn build_todo_status_patches_updates_all_or_only_first_needed_item() {
    let items = vec![
        item("1", "completed", "completed"),
        item("2", "active", "in_progress"),
        item("3", "waiting", "pending"),
    ];

    let all = super::build_todo_status_patches(&items, "completed", false);
    assert_eq!(all.len(), 2);
    assert_eq!(all[0], serde_json::json!({ "id": "2", "status": "completed" }));
    assert_eq!(all[1], serde_json::json!({ "id": "3", "status": "completed" }));

    let first = super::build_todo_status_patches(&items, "in_progress", true);
    assert!(first.is_empty());
}

#[test]
fn build_todo_status_patches_skips_completed_and_target_status_items() {
    let items = vec![
        item("1", "done", "completed"),
        item("2", "already", "pending"),
        item("3", "later", "in_progress"),
    ];

    let patches = super::build_todo_status_patches(&items, "pending", false);
    assert_eq!(patches, vec![serde_json::json!({ "id": "3", "status": "pending" })]);

    let only_first_stops_when_first_incomplete_already_has_target_status =
        super::build_todo_status_patches(&items, "pending", true);
    assert!(only_first_stops_when_first_incomplete_already_has_target_status.is_empty());

    let completed = vec![item("1", "done", "completed")];
    assert!(super::build_todo_status_patches(&completed, "completed", false).is_empty());
}

#[test]
fn build_todo_status_patches_returns_empty_for_empty_input() {
    assert!(super::build_todo_status_patches(&[], "completed", false).is_empty());
    assert!(super::build_todo_status_patches(&[], "in_progress", true).is_empty());
}

#[test]
fn build_todo_status_patches_only_updates_first_eligible_item() {
    let items = vec![
        item("1", "done", "completed"),
        item("2", "first", "pending"),
        item("3", "second", "pending"),
    ];

    let patches = super::build_todo_status_patches(&items, "in_progress", true);

    assert_eq!(patches, vec![serde_json::json!({ "id": "2", "status": "in_progress" })]);
}

#[test]
// 只自动推进首个待处理项且不产生多余用户事件。
fn maybe_mark_todo_in_progress_updates_first_pending_item_silently() {
    let session_id = unique_session("todos-progress");
    let ctx = ctx(&session_id);
    seed(
        &ctx,
        vec![
            item("1", "done", "completed"),
            item("2", "first", "pending"),
            item("3", "second", "pending"),
        ],
    );
    let mut session = Session::new(session_id);
    let mut tool_state = super::super::ToolSessionState::default();

    super::maybe_mark_todo_in_progress(&mut session, &ctx, &mut tool_state);

    let items = read(&ctx);
    assert_eq!(items[0].status, "completed");
    assert_eq!(items[1].status, "in_progress");
    assert_eq!(items[2].status, "pending");
    assert!(session.messages.iter().any(|message| matches!(message.role, Role::Tool)));
}

#[test]
fn maybe_mark_todo_in_progress_noops_when_first_incomplete_is_already_in_progress() {
    let session_id = unique_session("todos-progress-noop");
    let ctx = ctx(&session_id);
    seed(&ctx, vec![item("1", "active", "in_progress"), item("2", "waiting", "pending")]);
    let mut session = Session::new(session_id);
    let mut tool_state = super::super::ToolSessionState::default();

    super::maybe_mark_todo_in_progress(&mut session, &ctx, &mut tool_state);

    let items = read(&ctx);
    assert_eq!(items[0].status, "in_progress");
    assert_eq!(items[1].status, "pending");
    assert!(session.messages.is_empty());
}

#[test]
fn maybe_mark_todo_in_progress_noops_for_empty_list_without_side_effects() {
    let session_id = unique_session("todos-progress-empty");
    let ctx = ctx(&session_id);
    seed(&ctx, Vec::new());
    let mut session = Session::new(session_id);
    let mut tool_state = super::super::ToolSessionState::default();

    super::maybe_mark_todo_in_progress(&mut session, &ctx, &mut tool_state);

    assert!(super::read_todos_or_empty(&ctx).is_empty());
    assert!(session.messages.is_empty());
}
