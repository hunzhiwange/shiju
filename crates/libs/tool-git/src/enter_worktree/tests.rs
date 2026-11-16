//! 进入 worktree 工具的集成测试。
//!
//! 测试会创建临时 Git 仓库，确保工具使用真实 worktree 绑定，而不是仅更新内存状态。

use super::EnterWorktreeTool;
use agent_project::instance;
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
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().to_string_lossy().into_owned();
    context
        .bind_worktree(path.clone(), "existing".to_string(), "codex/existing".to_string())
        .unwrap();

    let error = scope_tool_use_context(context.clone(), tool.call(json!({})))
        .await
        .expect_err("a second worktree binding should be rejected");

    assert_eq!(error.to_string(), "a worktree is already bound to this session");
    let binding = context.worktree_binding_state();
    assert_eq!(binding.directory.as_deref(), Some(path.as_str()));
    assert_eq!(binding.name.as_deref(), Some("existing"));
    assert_eq!(binding.branch.as_deref(), Some("codex/existing"));
}

#[tokio::test]
async fn task_pool_worktree_binding_is_scoped_to_run() {
    let dir = tempfile::tempdir().unwrap();
    let root = Some(dir.path().to_string_lossy().into_owned());
    let parent = Arc::new(ToolUseContext::new("run-one", root.clone()));
    parent.bind_worktree(root.clone().unwrap(), "owned".into(), "owned".into()).unwrap();
    scope_tool_use_context(parent, async {
        let same = ToolUseContext::new("run-one", root.clone()).inherit_delegated_authority();
        let retry = ToolUseContext::new("run-two", root).inherit_delegated_authority();
        assert!(same.worktree_binding_state().directory.is_some());
        assert!(retry.worktree_binding_state().directory.is_none());
    })
    .await;
}
