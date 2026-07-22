use super::*;
use crate::app::agent::hooks::HookHandler;
use async_trait::async_trait;
use serde_json::json;

struct CancellingHook;

#[async_trait]
impl HookHandler for CancellingHook {
    fn name(&self) -> &str {
        "cancelling-tool-hook"
    }

    async fn before_tool_call(&self, _name: String, _args: Value) -> HookResult<(String, Value)> {
        HookResult::Cancel("blocked by test hook".to_string())
    }
}

#[tokio::test]
async fn pre_tool_hook_without_runner_preserves_tool_call() {
    let input = json!({"path": "notes.txt"});

    let (name, returned_input) =
        PreToolHook::run(None, "read".to_string(), input.clone()).await.unwrap();

    assert_eq!(name, "read");
    assert_eq!(returned_input, input);
}

#[tokio::test]
async fn pre_tool_hook_maps_cancellation_to_denied_error() {
    let mut runner = HookRunner::new();
    runner.register(Box::new(CancellingHook));

    let error = PreToolHook::run(Some(&runner), "shell".to_string(), json!({"command": "pwd"}))
        .await
        .unwrap_err();

    assert_eq!(error.message(), "Cancelled by hook: blocked by test hook");
    assert!(error.permission_request().is_none());
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
