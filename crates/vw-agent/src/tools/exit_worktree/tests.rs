//! 退出 worktree 工具测试。
//!
//! 覆盖工具元数据、上下文错误、工作树阻断、移除失败，以及真实 Git worktree 的退出路径。

use super::ExitWorktreeTool;
use crate::app::agent::project::instance;
use crate::app::agent::tools::context::{
    ToolUseContext, WorktreeBindingState, scope_tool_use_context,
};
use crate::app::agent::tools::{Tool, ToolCallResult};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};
use tempfile::TempDir;

fn configure_test_instance_paths() {
    static TEST_HOME: OnceLock<PathBuf> = OnceLock::new();

    let home = TEST_HOME.get_or_init(|| {
        let path = tempfile::tempdir().expect("tempdir should create").keep();
        std::fs::create_dir_all(path.join("Library").join("Application Support"))
            .expect("application support dir should create");
        path
    });

    unsafe {
        std::env::set_var("HOME", home);
        std::env::set_var("VIBEWINDOW_TEST_HOME", home);
    }
}

fn git(cwd: &Path, args: &[&str]) {
    let status =
        Command::new("git").args(args).current_dir(cwd).status().expect("git command should run");
    assert!(status.success(), "git {:?} should succeed", args);
}

fn init_git_repo(dir: &TempDir) {
    git(dir.path(), &["init", "-b", "main"]);
    git(dir.path(), &["config", "user.email", "test@example.com"]);
    git(dir.path(), &["config", "user.name", "Test User"]);
    std::fs::write(dir.path().join("README.md"), "seed\n").expect("seed file should write");
    git(dir.path(), &["add", "."]);
    git(dir.path(), &["commit", "-m", "init"]);
}

fn create_worktree(dir: &TempDir, name: &str) -> (PathBuf, String) {
    let worktree_dir = dir.path().join(format!("{name}-worktree"));
    let branch = format!("test/{name}");
    let worktree_arg = worktree_dir.to_string_lossy().to_string();
    git(dir.path(), &["worktree", "add", "-b", &branch, &worktree_arg, "HEAD"]);
    (worktree_dir, branch)
}

async fn call_exit_worktree_in_instance(
    workspace_dir: PathBuf,
    worktree_dir: PathBuf,
    branch: &str,
    input: Value,
) -> (anyhow::Result<ToolCallResult>, WorktreeBindingState) {
    configure_test_instance_paths();

    let tool = ExitWorktreeTool::new();
    let context_dir = workspace_dir.clone();
    let binding_dir = worktree_dir.clone();
    let binding_branch = branch.to_string();

    instance::provide(workspace_dir.as_path(), None, move || {
        let context = Arc::new(ToolUseContext::new(
            "exit-worktree",
            Some(context_dir.to_string_lossy().to_string()),
        ));
        context.bind_worktree(
            binding_dir.to_string_lossy().to_string(),
            "sandbox".to_string(),
            binding_branch,
        );
        Box::pin(async move {
            let result = scope_tool_use_context(context.clone(), tool.call(input)).await;
            (result, context.worktree_binding_state())
        })
    })
    .await
    .expect("instance should be available")
}

#[test]
fn exit_worktree_reports_expected_metadata() {
    let tool = ExitWorktreeTool::new();

    assert_eq!(tool.name(), crate::app::agent::tools::EXIT_WORKTREE_TOOL_ID);
    assert_eq!(
        tool.description(),
        "退出当前绑定的 worktree。默认要求工作树干净，force 模式会强制移除。"
    );
    assert_eq!(
        tool.parameters_schema(),
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "force": {
                    "type": "boolean",
                    "description": "为 true 时强制移除 worktree。"
                }
            }
        })
    );

    let spec = tool.spec();
    assert_eq!(spec.id, crate::app::agent::tools::EXIT_WORKTREE_TOOL_ID);
    assert_eq!(spec.display_name, "ExitWorktree");
    assert_eq!(spec.name, crate::app::agent::tools::EXIT_WORKTREE_TOOL_ID);
    assert_eq!(spec.parameters, tool.parameters_schema());
    assert_eq!(
        spec.aliases,
        crate::app::agent::tools::EXIT_WORKTREE_TOOL_ALIASES
            .iter()
            .map(|alias| alias.to_string())
            .collect::<Vec<_>>()
    );
    assert!(!spec.read_only);
    assert!(spec.destructive);
    assert!(!spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
}

#[tokio::test]
async fn exit_worktree_execute_returns_success_result() {
    let tool = ExitWorktreeTool::new();

    let result = tool.execute(json!({})).await.expect("execute should succeed");

    assert!(result.success);
    assert_eq!(result.output, "Exited worktree");
    assert_eq!(result.error, None);
}

#[tokio::test]
async fn exit_worktree_rejects_invalid_arguments() {
    let tool = ExitWorktreeTool::new();

    let error = tool.call(json!({"force": "yes"})).await.expect_err("invalid args should fail");

    assert!(error.to_string().contains("invalid exit worktree arguments"));
}

#[tokio::test]
async fn exit_worktree_fails_without_active_context() {
    let tool = ExitWorktreeTool::new();

    let error = tool.call(json!({})).await.expect_err("missing context should fail");

    assert!(error.to_string().contains("missing active tool context"));
}

#[tokio::test]
async fn exit_worktree_fails_without_binding() {
    let tool = ExitWorktreeTool::new();
    let context = Arc::new(ToolUseContext::new("exit-worktree", None));

    let result = scope_tool_use_context(context, tool.call(json!({}))).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn exit_worktree_fails_when_bound_directory_is_not_a_git_worktree() {
    let dir = TempDir::new().expect("tempdir should create");
    let tool = ExitWorktreeTool::new();
    let context = Arc::new(ToolUseContext::new(
        "exit-worktree",
        Some(dir.path().to_string_lossy().to_string()),
    ));
    context.bind_worktree(
        dir.path().to_string_lossy().to_string(),
        "sandbox".to_string(),
        "test/non-git".to_string(),
    );

    let error = scope_tool_use_context(context, tool.call(json!({})))
        .await
        .expect_err("non-git directory should fail");

    assert!(error.to_string().contains("failed to read worktree status"));
}

#[tokio::test]
async fn exit_worktree_blocks_untracked_worktree_without_force() {
    let dir = TempDir::new().expect("tempdir should create");
    init_git_repo(&dir);
    let (worktree_dir, branch) = create_worktree(&dir, "untracked-blocked");
    std::fs::write(worktree_dir.join("draft.txt"), "draft\n").expect("untracked file should write");

    let (result, binding) = call_exit_worktree_in_instance(
        dir.path().to_path_buf(),
        worktree_dir.clone(),
        &branch,
        json!({}),
    )
    .await;
    let result = result.expect("untracked worktree should return structured block");

    assert_eq!(result.data["exited"], false);
    assert_eq!(result.data["dirty"], true);
    assert_eq!(result.data["status"], "?? draft.txt\n");
    assert_eq!(binding.directory.as_deref(), Some(worktree_dir.to_string_lossy().as_ref()));
    assert!(worktree_dir.exists(), "untracked worktree should remain on disk");
}

#[tokio::test]
async fn exit_worktree_blocks_dirty_worktree_without_force() {
    let dir = TempDir::new().expect("tempdir should create");
    init_git_repo(&dir);
    let (worktree_dir, branch) = create_worktree(&dir, "dirty-blocked");
    std::fs::write(worktree_dir.join("README.md"), "dirty\n").expect("dirty file should write");

    let (result, binding) = call_exit_worktree_in_instance(
        dir.path().to_path_buf(),
        worktree_dir.clone(),
        &branch,
        json!({}),
    )
    .await;
    let result = result.expect("dirty worktree should return structured block");

    assert_eq!(
        result.data,
        json!({
            "exited": false,
            "directory": worktree_dir.to_string_lossy().to_string(),
            "dirty": true,
            "status": " M README.md\n"
        })
    );
    assert_eq!(result.model_result, Value::String("Worktree exit blocked".to_string()));
    assert_eq!(
        result.render_hint.as_ref().and_then(|hint| hint.summary.as_deref()),
        Some("Worktree has uncommitted changes")
    );
    assert_eq!(result.telemetry.as_ref().map(|telemetry| telemetry.success), Some(false));
    assert_eq!(binding.directory.as_deref(), Some(worktree_dir.to_string_lossy().as_ref()));
    assert!(worktree_dir.exists(), "dirty worktree should remain on disk");
}

#[tokio::test]
async fn exit_worktree_preserves_binding_when_remove_fails() {
    let dir = TempDir::new().expect("tempdir should create");
    init_git_repo(&dir);
    let directory = dir.path().to_path_buf();

    let (result, binding) =
        call_exit_worktree_in_instance(directory.clone(), directory.clone(), "main", json!({}))
            .await;
    let error = result.expect_err("primary worktree removal should fail");

    let message = error.to_string();
    assert!(message.contains("worktree") || message.contains("working tree"), "{message}");
    assert_eq!(binding.directory.as_deref(), Some(directory.to_string_lossy().as_ref()));
    assert!(directory.exists(), "failed removal should leave the worktree on disk");
}

#[tokio::test]
async fn exit_worktree_removes_clean_worktree_and_clears_binding() {
    let dir = TempDir::new().expect("tempdir should create");
    init_git_repo(&dir);
    let (worktree_dir, branch) = create_worktree(&dir, "clean-exit");

    let (result, binding) = call_exit_worktree_in_instance(
        dir.path().to_path_buf(),
        worktree_dir.clone(),
        &branch,
        json!({}),
    )
    .await;
    let result = result.expect("clean worktree should exit successfully");

    assert_eq!(
        result.data,
        json!({
            "exited": true,
            "directory": worktree_dir.to_string_lossy().to_string(),
            "force": false
        })
    );
    assert_eq!(result.model_result, Value::String("Exited worktree".to_string()));
    assert_eq!(
        result.render_hint.as_ref().and_then(|hint| hint.summary.as_deref()),
        Some("Worktree removed and binding cleared")
    );
    assert_eq!(result.telemetry.as_ref().map(|telemetry| telemetry.success), Some(true));
    assert_eq!(binding.directory, None);
    assert!(!worktree_dir.exists(), "clean worktree should be removed");
}

#[tokio::test]
async fn exit_worktree_force_removes_dirty_worktree_and_clears_binding() {
    let dir = TempDir::new().expect("tempdir should create");
    init_git_repo(&dir);
    let (worktree_dir, branch) = create_worktree(&dir, "force-exit");
    std::fs::write(worktree_dir.join("README.md"), "dirty\n").expect("dirty file should write");

    let (result, binding) = call_exit_worktree_in_instance(
        dir.path().to_path_buf(),
        worktree_dir.clone(),
        &branch,
        json!({"force": true}),
    )
    .await;
    let result = result.expect("force exit should remove dirty worktree");

    assert_eq!(
        result.data,
        json!({
            "exited": true,
            "directory": worktree_dir.to_string_lossy().to_string(),
            "force": true
        })
    );
    assert_eq!(binding.directory, None);
    assert!(!worktree_dir.exists(), "force exit should remove dirty worktree");
}
