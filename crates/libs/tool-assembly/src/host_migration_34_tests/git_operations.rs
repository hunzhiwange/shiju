//! Real agent task-local worktree routing contract.
use super::GitOperationsTool;
use tool_types::{Tool, ToolResult};
use agent_security::{AutonomyLevel, SecurityPolicy};
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

#[tokio::test]
async fn task_pool_git_uses_active_worktree_directory() {
    let tmp = repository();
    let repo = Repository::open(tmp.path()).unwrap();
    let worktree_parent = TempDir::new().unwrap();
    let worktree = worktree_parent.path().join("active");
    repo.worktree("active", &worktree, None).unwrap();
    let context = Arc::new(super::ToolUseContext::new(
        "git-test",
        Some(tmp.path().to_string_lossy().into()),
    ));
    context
        .bind_worktree(worktree.to_string_lossy().into(), "active".into(), "active".into())
        .unwrap();
    super::scope_tool_use_context(context, async {
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
