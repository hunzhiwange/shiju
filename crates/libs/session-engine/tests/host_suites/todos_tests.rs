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

    assert!(read(&ctx).is_empty());
    assert!(session.messages.is_empty());
}
