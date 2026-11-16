use tool_context::{hooks::PermissionHook, PermissionDecision};
use tool_types::{ToolResult, ToolCallResult};
use std::time::Duration;
use serde_json::json;
type PreToolHook = tool_context::hooks::PreToolHook<super::Host>;
type PostToolHook = tool_context::hooks::PostToolHook<super::Host>;
#[tokio::test]
async fn pre_tool_hook_without_runner_preserves_tool_call() {
    let input = json!({"path": "notes.txt"});

    let (name, returned_input) =
        PreToolHook::run(None, "read".to_string(), input.clone()).await.unwrap();

    assert_eq!(name, "read");
    assert_eq!(returned_input, input);
}

#[test]
fn permission_hook_preserves_each_decision_variant() {
    let allow = PermissionHook::adapt(PermissionDecision::allow(json!({"safe": true})));
    assert_eq!(allow.updated_input(), Some(&json!({"safe": true})));

    let ask = PermissionHook::adapt(PermissionDecision::ask("confirm", json!({"id": 1})));
    assert_eq!(ask.reason(), Some("confirm"));

    let deny = PermissionHook::adapt(PermissionDecision::deny("blocked"));
    assert_eq!(deny.reason(), Some("blocked"));
}

#[tokio::test]
async fn post_tool_hooks_without_runner_accept_structured_and_legacy_results() {
    let legacy =
        ToolResult { success: false, output: String::new(), error: Some("failed".to_string()) };
    let structured = ToolCallResult::from_legacy_result(legacy.clone());

    PostToolHook::run(None, "demo", &structured, Duration::from_millis(2)).await;
    PostToolHook::run_legacy(None, "demo", &legacy, Duration::from_millis(2)).await;
}
