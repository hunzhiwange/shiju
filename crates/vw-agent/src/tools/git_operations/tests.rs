//! 验证 Git 工具基于仓库状态执行读写操作、返回结构化结果并遵守安全约束。
//! 测试使用临时真实仓库覆盖发现、状态、提交、分支、暂存及只读拒绝路径。

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

// 每个用例获得独立仓库，避免分支、索引或工作树状态在测试之间泄漏。
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
    let committed = output(commit);
    assert_eq!(committed["message"], "add file\ndetail");
    assert_eq!(committed["commit"].as_str().unwrap().len(), 40);
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

#[tokio::test]
async fn task_pool_git_commit_requires_explicit_allowed_call() {
    let tmp = repository();
    let repo = Repository::open(tmp.path()).unwrap();
    let before = repo.head().unwrap().target();
    fs::write(tmp.path().join("tracked.txt"), "changed\n").unwrap();
    let denied = GitOperationsTool::new(
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() }),
        tmp.path().into(),
    );
    assert!(
        !denied.execute(json!({"operation":"commit","message":"denied"})).await.unwrap().success
    );
    assert_eq!(repo.head().unwrap().target(), before);
    let tool = test_tool(tmp.path());
    assert!(tool.execute(json!({"operation":"add","paths":"tracked.txt"})).await.unwrap().success);
    assert!(
        tool.execute(json!({"operation":"commit","message":"explicit"})).await.unwrap().success
    );
    assert_ne!(repo.head().unwrap().target(), before);
}

fn merge_fixture(tmp: &TempDir, conflict: bool) {
    let repo = Repository::open(tmp.path()).unwrap();
    let base = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch("feature", &base, false).unwrap();
    repo.set_head("refs/heads/feature").unwrap();
    fs::write(tmp.path().join("tracked.txt"), "feature\n").unwrap();
    commit_all(&repo, "feature");
    repo.set_head("refs/heads/master").unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();
    if conflict {
        fs::write(tmp.path().join("tracked.txt"), "target\n").unwrap();
        commit_all(&repo, "target");
    }
}

#[tokio::test]
async fn task_pool_git_merge_obeys_permissions_and_target_lock() {
    let tmp = repository();
    merge_fixture(&tmp, false);
    let args = json!({"operation":"merge","source":"feature","target":"master"});
    let denied = GitOperationsTool::new(
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() }),
        tmp.path().into(),
    );
    assert!(!denied.execute(args.clone()).await.unwrap().success);
    let repo = Repository::open(tmp.path()).unwrap();
    let mut lock = repo.transaction().unwrap();
    lock.lock_ref("refs/heads/master").unwrap();
    assert!(!test_tool(tmp.path()).execute(args.clone()).await.unwrap().success);
    drop(lock);
    fs::write(tmp.path().join("dirty"), "preserve").unwrap();
    assert!(!test_tool(tmp.path()).execute(args.clone()).await.unwrap().success);
    fs::remove_file(tmp.path().join("dirty")).unwrap();
    assert!(test_tool(tmp.path()).execute(args.clone()).await.unwrap().success);
    let repeated = output(test_tool(tmp.path()).execute(args).await.unwrap());
    assert_eq!(repeated["already_merged"], true);
    let divergent = repository();
    merge_fixture(&divergent, false);
    let repo = Repository::open(divergent.path()).unwrap();
    fs::write(divergent.path().join("target-only"), "target\n").unwrap();
    commit_all(&repo, "target-only");
    let merged = output(
        test_tool(divergent.path())
            .execute(json!({"operation":"merge","source":"feature","target":"master"}))
            .await
            .unwrap(),
    );
    assert_eq!(merged["already_merged"], false);
    assert_eq!(repo.head().unwrap().peel_to_commit().unwrap().parent_count(), 2);
    assert_eq!(fs::read_to_string(divergent.path().join("tracked.txt")).unwrap(), "feature\n");
    assert!(
        !test_tool(divergent.path())
            .execute(json!({"operation":"merge","source":"master","target":"feature"}))
            .await
            .unwrap()
            .success
    );
}

#[tokio::test]
async fn task_pool_git_uses_active_worktree_directory() {
    let tmp = repository();
    let repo = Repository::open(tmp.path()).unwrap();
    let worktree_parent = TempDir::new().unwrap();
    let worktree = worktree_parent.path().join("active");
    repo.worktree("active", &worktree, None).unwrap();
    let context = Arc::new(crate::tools::ToolUseContext::new(
        "git-test",
        Some(tmp.path().to_string_lossy().into()),
    ));
    context
        .bind_worktree(worktree.to_string_lossy().into(), "active".into(), "active".into())
        .unwrap();
    crate::tools::scope_tool_use_context(context, async {
        let status =
            output(test_tool(tmp.path()).execute(json!({"operation":"status"})).await.unwrap());
        assert_eq!(status["branch"], "active");
    })
    .await;
    let non_git = TempDir::new().unwrap();
    assert!(
        !test_tool(non_git.path())
            .execute(json!({"operation":"merge","source":"a","target":"b"}))
            .await
            .unwrap()
            .success
    );
}

#[tokio::test]
async fn task_pool_git_conflict_keeps_recoverable_state() {
    let tmp = repository();
    merge_fixture(&tmp, true);
    let repo = Repository::open(tmp.path()).unwrap();
    let before = repo.head().unwrap().target();
    let result = test_tool(tmp.path())
        .execute(json!({"operation":"merge","source":"feature","target":"master"}))
        .await
        .unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("conflict"));
    assert_eq!(repo.head().unwrap().target(), before);
    assert_eq!(fs::read_to_string(tmp.path().join("tracked.txt")).unwrap(), "target\n");
    assert_eq!(repo.state(), git2::RepositoryState::Clean);
}
