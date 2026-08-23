#![cfg(not(target_arch = "wasm32"))]

//! 验证 worktree 创建、列举、重置和删除操作及缺失项目上下文的失败边界。
//! 破坏性场景仅作用于临时仓库，并显式确认主工作区不会被重置或删除。

use super::{CreateInput, Error, RemoveInput, ResetInput, create, list_directories, remove, reset};
use crate::app::agent::project::{self, instance};
use std::path::{Path, PathBuf};
use std::process::Command;

// 统一匹配错误变体，避免只比较字符串而漏掉错误分类回归。
fn assert_missing_project_context(error: Error) {
    match error {
        Error::MissingProject(message) => assert_eq!(message, "missing project context"),
        other => panic!("expected missing project context, got {other}"),
    }
}

#[test]
fn operation_inputs_deserialize_aliases_and_defaults() {
    let create: CreateInput =
        serde_json::from_value(serde_json::json!({"name": "demo", "startCommand": "echo ok"}))
            .expect("create input");
    let remove: RemoveInput =
        serde_json::from_value(serde_json::json!({"directory": "/tmp/wt"})).expect("remove input");
    let reset: ResetInput =
        serde_json::from_value(serde_json::json!({"directory": "/tmp/wt", "baseRef": "main"}))
            .expect("reset input");

    assert_eq!(create.start_command.as_deref(), Some("echo ok"));
    assert_eq!(create.name.as_deref(), Some("demo"));
    assert!(!remove.force);
    assert_eq!(reset.base_ref.as_deref(), Some("main"));
}

#[test]
fn operation_inputs_serialize_camel_case_fields() {
    let create = serde_json::to_value(CreateInput {
        name: Some("demo".to_string()),
        start_command: Some("npm test".to_string()),
    })
    .expect("create json");
    let reset = serde_json::to_value(ResetInput {
        directory: "/tmp/wt".to_string(),
        base_ref: Some("main".to_string()),
    })
    .expect("reset json");
    let remove =
        serde_json::to_value(RemoveInput { directory: "/tmp/wt".to_string(), force: true })
            .expect("remove json");

    assert_eq!(create["startCommand"], "npm test");
    assert_eq!(reset["baseRef"], "main");
    assert_eq!(remove["force"], true);
}

#[tokio::test]
async fn create_without_project_context_returns_missing_project_before_creating() {
    let error = create(Some(CreateInput {
        name: Some("demo".to_string()),
        start_command: Some("should-not-run".to_string()),
    }))
    .await
    .expect_err("create should require project context");

    assert_missing_project_context(error);
}

#[tokio::test]
async fn remove_without_project_context_preserves_requested_directory() {
    let directory = tempfile::tempdir().expect("temp directory");
    let error = remove(RemoveInput {
        directory: directory.path().to_string_lossy().to_string(),
        force: true,
    })
    .await
    .expect_err("remove should require project context");

    assert_missing_project_context(error);
    assert!(directory.path().is_dir());
}

#[tokio::test]
async fn list_directories_without_project_context_returns_missing_project() {
    let error = list_directories().await.expect_err("list should require project context");

    assert_missing_project_context(error);
}

#[tokio::test]
async fn reset_without_project_context_preserves_requested_directory() {
    let directory = tempfile::tempdir().expect("temp directory");
    let error = reset(ResetInput {
        directory: directory.path().to_string_lossy().to_string(),
        base_ref: Some("main".to_string()),
    })
    .await
    .expect_err("reset should require project context");

    assert_missing_project_context(error);
    assert!(directory.path().is_dir());
}

#[tokio::test]
async fn list_directories_returns_primary_and_linked_worktrees() {
    let fixture = GitWorktreeFixture::new();
    let primary = fixture.primary.clone();
    let expected_primary = canonical_string(&fixture.primary);
    let expected_secondary = canonical_string(&fixture.secondary);

    let directories = in_project(&primary, || Box::pin(async { list_directories().await }))
        .await
        .expect("list worktrees");

    assert!(directories.iter().any(|directory| directory == &expected_primary));
    assert!(directories.iter().any(|directory| directory == &expected_secondary));
}

#[tokio::test]
async fn reset_rejects_primary_workspace_without_modifying_it() {
    let fixture = GitWorktreeFixture::new();
    let primary = fixture.primary.clone();
    let primary_input = fixture.primary.to_string_lossy().to_string();

    let error = in_project(&primary, move || {
        Box::pin(async move {
            reset(ResetInput { directory: primary_input, base_ref: Some("main".to_string()) }).await
        })
    })
    .await
    .expect_err("primary workspace must not be reset");

    assert!(
        matches!(error, Error::Invalid(message) if message == "Cannot reset the primary workspace")
    );
    assert_eq!(
        std::fs::read_to_string(fixture.primary.join("README.md")).expect("read primary file"),
        "demo"
    );
}

#[tokio::test]
async fn reset_restores_tracked_files_and_removes_untracked_files() {
    let fixture = GitWorktreeFixture::new();
    std::fs::write(fixture.secondary.join("README.md"), "changed").expect("change tracked file");
    std::fs::write(fixture.secondary.join("untracked.txt"), "remove me")
        .expect("write untracked file");
    let primary = fixture.primary.clone();
    let secondary = fixture.secondary.to_string_lossy().to_string();

    let reset_result = in_project(&primary, move || {
        Box::pin(async move {
            reset(ResetInput { directory: secondary, base_ref: Some("main".to_string()) }).await
        })
    })
    .await;

    assert!(reset_result.expect("reset linked worktree"));
    assert_eq!(
        std::fs::read_to_string(fixture.secondary.join("README.md")).expect("read restored file"),
        "demo"
    );
    assert!(!fixture.secondary.join("untracked.txt").exists());
}

#[tokio::test]
async fn remove_registered_worktree_deletes_directory_and_branch() {
    let fixture = GitWorktreeFixture::new();
    let primary = fixture.primary.clone();
    let secondary = fixture.secondary.to_string_lossy().to_string();

    let removed = in_project(&primary, move || {
        Box::pin(async move { remove(RemoveInput { directory: secondary, force: false }).await })
    })
    .await
    .expect("remove linked worktree");

    assert!(removed);
    assert!(!fixture.secondary.exists());
    let branch = Command::new("git")
        .arg("-C")
        .arg(&fixture.primary)
        .args(["show-ref", "--verify", "refs/heads/vibewindow/ops-test"])
        .output()
        .expect("inspect removed branch");
    assert!(!branch.status.success());
}

async fn in_project<T: Send + 'static>(
    primary: &Path,
    operation: impl FnOnce() -> project::BoxFuture<T> + Send + 'static,
) -> T {
    instance::provide(primary, None, operation).await.expect("project context")
}

struct GitWorktreeFixture {
    _root: tempfile::TempDir,
    primary: PathBuf,
    secondary: PathBuf,
}

impl GitWorktreeFixture {
    fn new() -> Self {
        let root = tempfile::tempdir().expect("fixture root");
        let primary = root.path().join("primary");
        let secondary = root.path().join("secondary");
        run(Command::new("git").args(["init", "-b", "main"]).arg(&primary));
        run(Command::new("git").arg("-C").arg(&primary).args([
            "config",
            "user.email",
            "test@example.com",
        ]));
        run(Command::new("git").arg("-C").arg(&primary).args(["config", "user.name", "Test User"]));
        std::fs::write(primary.join("README.md"), "demo").expect("write fixture file");
        std::fs::write(primary.join("FIXTURE_ID"), primary.to_string_lossy().as_bytes())
            .expect("write unique fixture id");
        run(Command::new("git").arg("-C").arg(&primary).args(["add", "README.md", "FIXTURE_ID"]));
        run(Command::new("git").arg("-C").arg(&primary).args(["commit", "-m", "init"]));
        run(Command::new("git")
            .arg("-C")
            .arg(&primary)
            .args(["worktree", "add", "-b", "vibewindow/ops-test"])
            .arg(&secondary));
        Self { _root: root, primary, secondary }
    }
}

fn canonical_string(path: &Path) -> String {
    std::fs::canonicalize(path).expect("canonical fixture path").to_string_lossy().to_string()
}

fn run(command: &mut Command) {
    let output = command.output().expect("run git command");
    assert!(
        output.status.success(),
        "git command failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
