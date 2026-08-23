//! 验证 Git 工具基于仓库状态执行读写操作、返回结构化结果并遵守安全约束。

use super::super::*;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use git2::{Repository, Signature};
use serde_json::{Value, json};
use std::{fs, path::Path, sync::Arc};
use tempfile::TempDir;

fn test_tool(dir: &Path) -> GitOperationsTool {
    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        ..SecurityPolicy::default()
    });
    GitOperationsTool::new(security, dir.to_path_buf())
}

fn repository() -> TempDir {
    let tmp = TempDir::new().unwrap();
    let repo = Repository::init(tmp.path()).unwrap();
    fs::write(tmp.path().join("tracked.txt"), "first line\n").unwrap();
    commit_all(&repo, "initial commit");
    tmp
}

fn commit_all(repo: &Repository, message: &str) {
    let mut index = repo.index().unwrap();
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = Signature::now("Test User", "test@example.com").unwrap();
    let parents = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parent_refs = parents.iter().collect::<Vec<_>>();
    repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &parent_refs).unwrap();
}

fn output(result: ToolResult) -> Value {
    assert!(result.success, "tool failed: {:?}", result.error);
    serde_json::from_str(&result.output).unwrap()
}

#[test]
fn production_module_does_not_launch_system_git() {
    let source = include_str!("mod.rs");
    assert!(!source.contains("git_tokio_command"));
    assert!(!source.contains("Command::new(\"git\")"));
}

#[tokio::test]
async fn status_and_diff_use_git2_repository_state() {
    let tmp = repository();
    fs::write(tmp.path().join("tracked.txt"), "changed line\n").unwrap();
    fs::write(tmp.path().join("untracked.txt"), "new file\n").unwrap();
    let tool = test_tool(tmp.path());

    let status = output(tool.execute(json!({"operation": "status"})).await.unwrap());
    assert_eq!(status["branch"], "master");
    assert_eq!(status["untracked"], json!(["untracked.txt"]));
    assert_eq!(status["unstaged"][0]["path"], "tracked.txt");
    assert_eq!(status["clean"], false);

    let diff =
        output(tool.execute(json!({"operation": "diff", "files": "tracked.txt"})).await.unwrap());
    assert!(diff["file_count"].as_u64().unwrap() >= 1);
    assert_eq!(diff["hunks"][0]["file"], "tracked.txt");
}

#[tokio::test]
async fn add_commit_and_log_preserve_structured_contract() {
    let tmp = repository();
    fs::write(tmp.path().join("added.txt"), "content\n").unwrap();
    let tool = test_tool(tmp.path());

    let add = tool.execute(json!({"operation": "add", "paths": "added.txt"})).await.unwrap();
    assert!(add.success, "{:?}", add.error);
    let staged = output(tool.execute(json!({"operation": "status"})).await.unwrap());
    assert_eq!(staged["staged"][0]["path"], "added.txt");

    let commit = tool
        .execute(json!({"operation": "commit", "message": "  add file  \n\n  detail  "}))
        .await
        .unwrap();
    assert_eq!(commit.output, "Committed: add file\ndetail");
    let log = output(tool.execute(json!({"operation": "log", "limit": 1})).await.unwrap());
    assert_eq!(log["commits"][0]["message"], "add file detail");
    assert!(log["commits"][0]["author"].as_str().is_some_and(|author| !author.is_empty()));
}

#[tokio::test]
async fn branch_checkout_and_stash_use_git2_repository_state() {
    let tmp = repository();
    let repo = Repository::open(tmp.path()).unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch("feature", &head, false).unwrap();
    let tool = test_tool(tmp.path());

    let branches = output(tool.execute(json!({"operation": "branch"})).await.unwrap());
    assert_eq!(branches["current"], "master");
    assert!(
        branches["branches"].as_array().unwrap().iter().any(|branch| branch["name"] == "feature")
    );

    let checkout =
        tool.execute(json!({"operation": "checkout", "branch": "feature"})).await.unwrap();
    assert_eq!(checkout.output, "Switched to branch: feature");
    fs::write(tmp.path().join("tracked.txt"), "stash this\n").unwrap();
    let push = tool.execute(json!({"operation": "stash", "action": "push"})).await.unwrap();
    assert!(push.success, "{:?}", push.error);
    let list = tool.execute(json!({"operation": "stash", "action": "list"})).await.unwrap();
    assert!(list.success, "{:?}", list.error);
    assert!(list.output.contains("auto-stash"));
    let pop = tool.execute(json!({"operation": "stash", "action": "pop"})).await.unwrap();
    assert!(pop.success, "{:?}", pop.error);
}

#[tokio::test]
async fn rejects_non_repository_before_dispatch() {
    let tmp = TempDir::new().unwrap();
    let result = test_tool(tmp.path()).execute(json!({"operation": "status"})).await.unwrap();
    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("Not in a git repository"));
}

#[tokio::test]
async fn finds_parent_repository_before_dispatch() {
    let tmp = repository();
    let nested = tmp.path().join("nested");
    fs::create_dir(&nested).unwrap();
    let result = test_tool(&nested).execute(json!({"operation": "unsupported"})).await.unwrap();
    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("Unknown operation: unsupported"));
}

#[tokio::test]
async fn blocks_readonly_write_operations() {
    let tmp = repository();
    let security =
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() });
    let result = GitOperationsTool::new(security, tmp.path().to_path_buf())
        .execute(json!({"operation": "commit", "message": "test"}))
        .await
        .unwrap();
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or_default().contains("higher autonomy"));
}

#[test]
fn truncate_commit_message_preserves_unicode_limit() {
    let truncated = GitOperationsTool::truncate_commit_message(&"🦀".repeat(2500));
    assert_eq!(truncated.chars().count(), 2000);
}

#[tokio::test]
async fn checkout_rejects_invalid_branch_names() {
    let tmp = repository();
    for branch in ["", "main other", "feature@{1}", "main^", "main~1"] {
        let result = test_tool(tmp.path()).git_checkout(json!({"branch": branch})).await;
        assert!(result.is_err(), "{branch:?} should be rejected");
    }
}
