use super::*;
use crate::app::agent::tools::context::{ToolUseContext, scope_tool_use_context};
use crate::app::agent::tools::traits::Tool;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn session(name: &str) -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("verify-plan-{name}-{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

fn save_todos(session: &str, statuses: &[&str]) {
    let todos = statuses
        .iter()
        .enumerate()
        .map(|(index, status)| crate::app::agent::tools::todo::Todo {
            id: (index + 1).to_string(),
            content: format!("Todo {index}"),
            status: (*status).to_string(),
            priority: "medium".to_string(),
        })
        .collect::<Vec<_>>();
    crate::app::agent::tools::todo::write_session_todos_for_test(session, todos)
        .expect("save test todos");
}

#[test]
fn spec_is_read_only_strict_and_empty_schema() {
    let spec = VerifyPlanExecutionTool::new().spec();
    assert_eq!(spec.display_name, "VerifyPlanExecution");
    assert!(spec.read_only);
    assert!(spec.concurrency_safe);
    assert!(spec.strict);
    assert!(spec.input_schema["properties"].as_object().unwrap().is_empty());
}

#[tokio::test]
async fn call_reports_blockers_and_ready_state() {
    let blocked_session = session("blocked");
    save_todos(&blocked_session, &[]);
    let blocked = scope_tool_use_context(
        Arc::new(ToolUseContext::new(blocked_session, None)),
        VerifyPlanExecutionTool::new().call(json!({})),
    )
    .await
    .unwrap();
    assert!(!blocked.data["ready"].as_bool().unwrap());
    assert_eq!(blocked.data["todo_count"], 0);

    let ready_session = session("ready");
    save_todos(&ready_session, &["pending", "in_progress", "completed"]);
    let context = Arc::new(ToolUseContext::new(ready_session, None));
    context.enter_plan_mode(Some("goal".into()), Some("note".into()));
    let ready = scope_tool_use_context(context, VerifyPlanExecutionTool::new().call(json!({})))
        .await
        .unwrap();
    assert!(ready.data["ready"].as_bool().unwrap());
    assert_eq!(ready.data["pending_count"], 2);
    assert_eq!(ready.data["in_progress_count"], 1);
}

#[tokio::test]
async fn call_blocks_multiple_in_progress_todos() {
    let session = session("multi");
    save_todos(&session, &["in_progress", "in_progress"]);
    let context = Arc::new(ToolUseContext::new(session, None));
    context.enter_plan_mode(None, None);

    let result = scope_tool_use_context(context, VerifyPlanExecutionTool::new().call(json!({})))
        .await
        .unwrap();
    assert!(!result.data["ready"].as_bool().unwrap());
    assert_eq!(result.data["in_progress_count"], 2);
}

#[tokio::test]
async fn call_allows_active_plan_when_all_todos_are_completed() {
    let session = session("completed");
    save_todos(&session, &["completed", "completed"]);
    let context = Arc::new(ToolUseContext::new(session, None));
    context.enter_plan_mode(Some("finished goal".into()), None);

    let result = scope_tool_use_context(context, VerifyPlanExecutionTool::new().call(json!({})))
        .await
        .unwrap();

    assert!(result.data["ready"].as_bool().unwrap());
    assert_eq!(result.data["todo_count"], 2);
    assert_eq!(result.data["pending_count"], 0);
    assert_eq!(result.data["in_progress_count"], 0);
    assert_eq!(result.render_hint.unwrap().summary.as_deref(), Some("Ready to execute 0 todo(s)"));
}
