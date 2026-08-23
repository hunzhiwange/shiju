//! 验证待办读写工具的规格、持久化往返、结构化结果与限流错误。
//! 旧式 execute 与新版 call 两条入口必须共享相同的数据和错误语义。

use super::*;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::traits::Tool;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn session(name: &str) -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("todo-mod-test-{name}-{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

fn security(max_actions_per_hour: u32) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy { max_actions_per_hour, ..SecurityPolicy::default() })
}

#[test]
fn read_tool_spec_is_read_only_and_strict() {
    let tool = TodoReadTool::new("session".to_string());
    let spec = tool.spec();

    assert_eq!(tool.name(), "todoread");
    assert_eq!(spec.id, crate::app::agent::tools::TODO_READ_TOOL_ID);
    assert_eq!(spec.aliases, vec![crate::app::agent::tools::TODO_READ_TOOL_ALIAS]);
    assert!(spec.read_only);
    assert!(!spec.destructive);
    assert!(spec.concurrency_safe);
    assert!(spec.strict);
    assert_eq!(tool.parameters_schema(), json!({"type": "object", "properties": {}}));
}

#[test]
fn write_tool_spec_is_actionable_and_strict() {
    let tool = TodoWriteTool::new("session".to_string(), security(20));
    let spec = tool.spec();

    assert_eq!(tool.name(), "todowrite");
    assert_eq!(spec.id, crate::app::agent::tools::TODO_WRITE_TOOL_ID);
    assert_eq!(spec.aliases, vec![crate::app::agent::tools::TODO_WRITE_TOOL_ALIAS]);
    assert!(!spec.read_only);
    assert!(!spec.destructive);
    assert!(!spec.concurrency_safe);
    assert!(spec.strict);
    assert_eq!(tool.parameters_schema()["required"], json!(["todos"]));
}

#[test]
fn write_rejects_empty_input() {
    let ctx = crate::app::agent::tools::ToolRuntimeContext::new(session("empty-input"), None);

    assert!(write("   ", &ctx).is_err());
}

#[test]
fn write_rejects_malformed_json() {
    let ctx = crate::app::agent::tools::ToolRuntimeContext::new(session("bad-json"), None);
    let err = write("{", &ctx).expect_err("malformed json should fail");

    assert!(err.to_string().contains("EOF"));
}

#[test]
fn write_rejects_missing_todos() {
    let ctx = crate::app::agent::tools::ToolRuntimeContext::new(session("missing-todos"), None);
    let err = write(r#"{"merge":true}"#, &ctx).expect_err("todos is required");

    assert!(err.to_string().contains("todos"));
}

#[test]
fn write_then_read_round_trips_pretty_json() {
    let ctx = crate::app::agent::tools::ToolRuntimeContext::new(session("round-trip"), None);

    let output = write(
        r#"{"todos":[{"id":1,"content":" ship ","status":"pending","priority":"high"}]}"#,
        &ctx,
    )
    .expect("write succeeds");
    let read_output = read(&ctx).expect("read succeeds");

    assert!(output.contains("\"content\": \"ship\""));
    assert_eq!(serde_json::from_str::<Value>(&read_output).unwrap()[0]["content"], "ship");
}

#[tokio::test]
async fn read_tool_call_returns_structured_todos() {
    let session = session("read-call");
    let ctx = crate::app::agent::tools::ToolRuntimeContext::new(session.clone(), None);
    write(r#"{"todos":[{"id":"a","content":"plan"}]}"#, &ctx).expect("seed succeeds");

    let result = TodoReadTool::new(session).call(json!({})).await.expect("call succeeds");

    assert!(result.is_success());
    assert_eq!(result.data["todos"][0]["content"], "plan");
    assert_eq!(result.render_hint.unwrap().metadata["count"], 1);
    assert_eq!(result.content_blocks.len(), 1);
    assert!(result.model_result.as_str().unwrap().contains("\"content\": \"plan\""));
}

#[tokio::test]
async fn read_tool_execute_returns_legacy_result() {
    let session = session("read-execute");
    let ctx = crate::app::agent::tools::ToolRuntimeContext::new(session.clone(), None);
    write(r#"{"todos":[{"content":"review"}]}"#, &ctx).expect("seed succeeds");

    let result = TodoReadTool::new(session).execute(json!({})).await.expect("execute succeeds");

    assert!(result.success);
    assert!(result.output.contains("\"content\": \"review\""));
    assert_eq!(result.error, None);
}

#[tokio::test]
async fn write_tool_call_reports_previous_and_next_todos() {
    let session = session("write-call");
    let ctx = crate::app::agent::tools::ToolRuntimeContext::new(session.clone(), None);
    write(r#"{"todos":[{"id":"old","content":"old task"}]}"#, &ctx).expect("seed succeeds");

    let result = TodoWriteTool::new(session, security(20))
        .call(json!({"todos": [{"id": "new", "content": "new task"}]}))
        .await
        .expect("call succeeds");

    assert!(result.is_success());
    assert_eq!(result.data["oldTodos"][0]["content"], "old task");
    assert_eq!(result.data["newTodos"][0]["content"], "new task");
    assert_eq!(result.data["merge"], false);
    assert_eq!(result.render_hint.unwrap().metadata["count"], 1);
}

#[tokio::test]
async fn write_tool_execute_writes_todos() {
    let session = session("write-execute");
    let result = TodoWriteTool::new(session.clone(), security(20))
        .execute(json!({"todos": [{"id": 9, "content": "execute task"}]}))
        .await
        .expect("execute succeeds");

    assert!(result.success);
    assert_eq!(serde_json::from_str::<Value>(&result.output).unwrap()[0]["id"], "9");
    assert_eq!(read_for_tool(&session).unwrap()[0].content, "execute task");
}

#[tokio::test]
async fn write_tool_call_returns_structured_rate_limit_error() {
    let result = TodoWriteTool::new(session("call-rate-limit"), security(0))
        .call(json!({"todos": [{"content": "blocked"}]}))
        .await
        .expect("call returns legacy error result");

    assert!(!result.is_success());
    assert_eq!(result.data["error"], "Rate limit exceeded");
    assert_eq!(
        result.render_hint.unwrap().summary.as_deref(),
        Some("Todo update blocked by rate limit")
    );
}

#[tokio::test]
async fn write_tool_execute_returns_rate_limit_error() {
    let result = TodoWriteTool::new(session("execute-rate-limit"), security(0))
        .execute(json!({"todos": [{"content": "blocked"}]}))
        .await
        .expect("execute returns tool result");

    assert!(!result.success);
    assert_eq!(result.output, "");
    assert_eq!(result.error.as_deref(), Some("Rate limit exceeded"));
}
