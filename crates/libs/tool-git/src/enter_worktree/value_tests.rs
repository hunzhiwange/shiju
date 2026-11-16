//! 工具输入与规格的局部契约，不调用宿主服务。

use super::value_test_host::InputHost;
use serde_json::json;
use tool_types::Tool;
type EnterWorktreeTool = super::EnterWorktreeTool<InputHost>;

#[tokio::test]
async fn enter_worktree_rejects_non_object_arguments() {
    let error = EnterWorktreeTool::new()
        .call(json!("invalid"))
        .await
        .expect_err("string arguments should be rejected before context lookup");

    assert!(error.to_string().contains("invalid enter worktree arguments"));
}

#[tokio::test]
async fn enter_worktree_reports_metadata_and_legacy_execute_result() {
    let tool = EnterWorktreeTool::new();

    assert_eq!(tool.name(), "enter_worktree");
    assert!(tool.description().contains("Git worktree"));
    let schema = tool.parameters_schema();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["properties"]["name"]["type"], "string");
    assert_eq!(schema["properties"]["startCommand"]["type"], "string");

    let spec = tool.spec();
    assert_eq!(spec.id, "enter_worktree");
    assert_eq!(spec.display_name, "EnterWorktree");
    assert_eq!(
        spec.aliases,
        ["EnterWorktree"]
            .iter()
            .map(|alias| alias.to_string())
            .collect::<Vec<_>>()
    );
    assert!(!spec.read_only);
    assert!(spec.destructive);
    assert!(!spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);

    let result = tool.execute(json!({})).await.expect("legacy execute should succeed");
    assert!(result.success);
    assert_eq!(result.output, "Entered worktree");
    assert_eq!(result.error, None);
}

#[tokio::test]
async fn enter_worktree_rejects_object_arguments_without_tool_context() {
    let error = EnterWorktreeTool::new()
        .call(json!({}))
        .await
        .expect_err("object arguments should require an active tool context");

    assert_eq!(error.to_string(), "missing active tool context");
}
