//! 进入 worktree 工具的集成测试。
//!
//! 测试会创建临时 Git 仓库，确保工具使用真实 worktree 绑定，而不是仅更新内存状态。

use super::EnterWorktreeTool;
use crate::app::agent::project::instance;
use crate::app::agent::tools::Tool;
use crate::app::agent::tools::context::{ToolUseContext, scope_tool_use_context};
use serde_json::json;
use std::process::Command;
use std::sync::Arc;
use tempfile::TempDir;

fn init_git_repo(dir: &TempDir) {
    // worktree 只能从有提交的仓库创建，因此测试夹具显式初始化一次提交。
    let status = Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir.path())
        .status()
        .expect("git init should run");
    assert!(status.success());
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .status()
        .expect("git config email should run");
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .status()
        .expect("git config name should run");
    std::fs::write(dir.path().join("README.md"), "seed\n").expect("seed file should write");
    let add_status = Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .status()
        .expect("git add should run");
    assert!(add_status.success());
    let commit_status = Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(dir.path())
        .status()
        .expect("git commit should run");
    assert!(commit_status.success());
}

#[tokio::test]
async fn enter_worktree_creates_and_binds_real_worktree() {
    let dir = TempDir::new().expect("tempdir should create");
    init_git_repo(&dir);
    let tool = EnterWorktreeTool::new();
    let workspace_dir = dir.path().to_path_buf();
    let context_dir = workspace_dir.clone();

    let binding = instance::provide(workspace_dir.as_path(), None, move || {
        let context = Arc::new(ToolUseContext::new(
            "enter-worktree",
            Some(context_dir.to_string_lossy().to_string()),
        ));
        Box::pin(async move {
            scope_tool_use_context(
                context.clone(),
                tool.call(json!({
                    "name": "plan-mode-sandbox"
                })),
            )
            .await
            .expect("tool should succeed");
            context.worktree_binding_state()
        })
    })
    .await
    .expect("instance should be available");

    assert!(binding.directory.is_some());
    assert_eq!(binding.name.as_deref(), Some("plan-mode-sandbox"));
}

#[tokio::test]
async fn enter_worktree_rejects_an_existing_session_binding() {
    let tool = EnterWorktreeTool::new();
    let context = Arc::new(ToolUseContext::new("enter-worktree-bound", None));
    context.bind_worktree(
        "/tmp/existing-worktree".to_string(),
        "existing".to_string(),
        "codex/existing".to_string(),
    );

    let error = scope_tool_use_context(context.clone(), tool.call(json!({})))
        .await
        .expect_err("a second worktree binding should be rejected");

    assert_eq!(error.to_string(), "a worktree is already bound to this session");
    let binding = context.worktree_binding_state();
    assert_eq!(binding.directory.as_deref(), Some("/tmp/existing-worktree"));
    assert_eq!(binding.name.as_deref(), Some("existing"));
    assert_eq!(binding.branch.as_deref(), Some("codex/existing"));
}

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

    assert_eq!(tool.name(), crate::app::agent::tools::ENTER_WORKTREE_TOOL_ID);
    assert!(tool.description().contains("Git worktree"));
    let schema = tool.parameters_schema();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["properties"]["name"]["type"], "string");
    assert_eq!(schema["properties"]["startCommand"]["type"], "string");

    let spec = tool.spec();
    assert_eq!(spec.id, crate::app::agent::tools::ENTER_WORKTREE_TOOL_ID);
    assert_eq!(spec.display_name, "EnterWorktree");
    assert_eq!(
        spec.aliases,
        crate::app::agent::tools::ENTER_WORKTREE_TOOL_ALIASES
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
