use super::*;
use git2::{Repository, Signature};
use vw_api_types::git::{GitFileStatus, GitRemoteAction};

fn commit(repo: &Repository, path: &str, contents: &str, message: &str) -> git2::Oid {
    std::fs::write(repo.workdir().unwrap().join(path), contents).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new(path)).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = Signature::now("测试作者", "test@example.com").unwrap();
    let parent = repo.head().ok().map(|head| head.peel_to_commit().unwrap());
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &parent.iter().collect::<Vec<_>>(),
    )
    .unwrap()
}

#[test]
fn gateway_history_reads_commit_objects_and_file_statuses() {
    let temp = tempfile::tempdir().unwrap();
    let repo = Repository::init(temp.path()).unwrap();
    let path = temp.path().to_str().unwrap();
    assert!(history::snapshot(path, 100).unwrap().commits.is_empty());
    let root = commit(&repo, "中文.txt", "初始内容\n", "首个提交");
    let second = commit(&repo, "中文.txt", "已提交内容\n", "修改内容");
    std::fs::write(temp.path().join("中文.txt"), "未提交内容\n").unwrap();
    let page = history::snapshot(path, 1).unwrap();
    assert!(page.has_more);
    assert_eq!(page.commits[0].sha, second.to_string());
    let first = history::detail(path, &root.to_string()).unwrap();
    assert_eq!(first.files[0].status, GitFileStatus::Added);
    let detail = history::detail(path, &second.to_string()).unwrap();
    assert_eq!(detail.files[0].status, GitFileStatus::Modified);
    assert_eq!(detail.files[0].new_content.as_deref(), Some("已提交内容\n"));
    assert_eq!(detail.files[0].old_content.as_deref(), Some("初始内容\n"));
    std::fs::remove_file(temp.path().join("中文.txt")).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(std::path::Path::new("中文.txt")).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = repo.find_commit(second).unwrap();
    let signature = Signature::now("测试作者", "test@example.com").unwrap();
    let deleted =
        repo.commit(Some("HEAD"), &signature, &signature, "delete", &tree, &[&parent]).unwrap();
    assert_eq!(
        history::detail(path, &deleted.to_string()).unwrap().files[0].status,
        GitFileStatus::Deleted
    );
}

#[test]
fn gateway_remote_publish_fetch_pull_and_divergence_use_git2_only() {
    let temp = tempfile::tempdir().unwrap();
    let bare_path = temp.path().join("remote.git");
    let bare = Repository::init_bare(&bare_path).unwrap();
    let local_path = temp.path().join("local");
    let local = Repository::init(&local_path).unwrap();
    let initial = commit(&local, "note.txt", "initial\n", "initial");
    local.remote("origin", bare_path.to_str().unwrap()).unwrap();
    let path = local_path.to_str().unwrap();
    assert!(remote::sync(path, GitRemoteAction::Pull).is_err());
    remote::sync(path, GitRemoteAction::Push).unwrap();
    let snapshot = history::snapshot(path, 1).unwrap();
    assert!(snapshot.upstream);
    let branch = snapshot.branch.unwrap();
    assert_eq!(
        bare.find_reference(&format!("refs/heads/{branch}")).unwrap().target(),
        Some(initial)
    );
    let peer_path = temp.path().join("peer");
    let peer = Repository::clone(path, &peer_path).unwrap();
    peer.remote_set_url("origin", bare_path.to_str().unwrap()).unwrap();
    let advance = commit(&peer, "note.txt", "remote update\n", "advance");
    remote::sync(peer_path.to_str().unwrap(), GitRemoteAction::Push).unwrap();
    remote::sync(path, GitRemoteAction::Fetch).unwrap();
    assert_eq!(history::snapshot(path, 1).unwrap().ahead_behind, Some((0, 1)));
    remote::sync(path, GitRemoteAction::Pull).unwrap();
    assert_eq!(local.head().unwrap().target(), Some(advance));
    assert_eq!(std::fs::read_to_string(local_path.join("note.txt")).unwrap(), "remote update\n");
    // 未提交内容必须完整保留。
    std::fs::write(local_path.join("note.txt"), "未提交\n").unwrap();
    assert!(remote::sync(path, GitRemoteAction::Pull).is_err());
    assert_eq!(std::fs::read_to_string(local_path.join("note.txt")).unwrap(), "未提交\n");
    commit(&local, "note.txt", "local\n", "local diverges");
    commit(&peer, "note.txt", "peer\n", "peer diverges");
    remote::sync(peer_path.to_str().unwrap(), GitRemoteAction::Push).unwrap();
    let old = local.head().unwrap().target();
    assert!(remote::sync(path, GitRemoteAction::Pull).unwrap_err().contains("分叉"));
    assert_eq!(local.head().unwrap().target(), old);
    assert!(remote::sync(path, GitRemoteAction::Push).is_err());
}

#[tokio::test]
async fn gateway_repository_validates_requests_and_project_scope() {
    let _router = router::<()>();
    let invalid = repository(Json(GitRepositoryRequest {
        project_id: vw_api_types::id::ProjectId("missing".into()),
        worktree_id: None,
        limit: 0,
    }))
    .await
    .unwrap_err();
    assert_eq!(invalid.status, axum::http::StatusCode::BAD_REQUEST);
    let missing = repository(Json(GitRepositoryRequest {
        project_id: vw_api_types::id::ProjectId("missing".into()),
        worktree_id: None,
        limit: 100,
    }))
    .await
    .unwrap_err();
    assert_eq!(missing.status, axum::http::StatusCode::NOT_FOUND);
    let invalid = detail(Json(GitHistoryDetailRequest {
        project_id: vw_api_types::id::ProjectId("missing".into()),
        worktree_id: None,
        sha: "HEAD;invalid".into(),
    }))
    .await
    .unwrap_err();
    assert_eq!(invalid.status, axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn gateway_repository_routes_use_registered_projects_and_reject_foreign_worktrees() {
    use crate::app::agent::project;
    use base64::Engine;
    let temp = tempfile::tempdir().unwrap();
    let repo = Repository::init(temp.path()).unwrap();
    let sha = commit(&repo, "note.txt", "base\n", "base");
    let id = format!("git-history-{}", uuid::Uuid::new_v4());
    struct Cleanup(String);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            project::remove_local_test_project(&self.0).unwrap();
        }
    }
    let _cleanup = Cleanup(id.clone());
    project::store_local_test_project(&project::Info {
        id: id.clone(),
        worktree: temp.path().to_string_lossy().into_owned(),
        vcs: Some(project::Vcs::Git),
        name: None,
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: vec![],
    })
    .unwrap();
    let project_id = vw_api_types::id::ProjectId(id);
    let Json(result) = repository(Json(GitRepositoryRequest {
        project_id: project_id.clone(),
        worktree_id: None,
        limit: 100,
    }))
    .await
    .unwrap();
    assert_eq!(result.commits[0].sha, sha.to_string());
    let Json(result) = detail(Json(GitHistoryDetailRequest {
        project_id: project_id.clone(),
        worktree_id: None,
        sha: sha.to_string(),
    }))
    .await
    .unwrap();
    assert_eq!(result.files[0].status, GitFileStatus::Added);
    let other = tempfile::tempdir().unwrap();
    let branch = repo.branch("history-side", &repo.find_commit(sha).unwrap(), false).unwrap();
    let mut options = git2::WorktreeAddOptions::new();
    options.reference(Some(branch.get()));
    let linked_path = other.path().join("linked");
    let _linked = repo.worktree("history-side", &linked_path, Some(&options)).unwrap();
    let linked_id = vw_api_types::id::WorktreeId(
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(linked_path.canonicalize().unwrap().to_str().unwrap()),
    );
    let Json(linked) = repository(Json(GitRepositoryRequest {
        project_id: project_id.clone(),
        worktree_id: Some(linked_id),
        limit: 100,
    }))
    .await
    .unwrap();
    assert_eq!(linked.branch.as_deref(), Some("history-side"));
    let foreign = vw_api_types::id::WorktreeId(
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(other.path().to_str().unwrap()),
    );
    let error = repository(Json(GitRepositoryRequest {
        project_id,
        worktree_id: Some(foreign),
        limit: 100,
    }))
    .await
    .unwrap_err();
    assert_eq!(error.status, axum::http::StatusCode::NOT_FOUND);
}

#[test]
fn gateway_history_recognizes_renamed_files() {
    let temp = tempfile::tempdir().unwrap();
    let repo = Repository::init(temp.path()).unwrap();
    commit(&repo, "old.txt", "same content\n", "initial");
    std::fs::rename(temp.path().join("old.txt"), temp.path().join("new.txt")).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(std::path::Path::new("old.txt")).unwrap();
    index.write().unwrap();
    let sha = commit(&repo, "new.txt", "same content\n", "rename");
    let detail = history::detail(temp.path().to_str().unwrap(), &sha.to_string()).unwrap();
    assert_eq!(detail.files.len(), 1);
    assert_eq!(detail.files[0].status, GitFileStatus::Renamed);
    assert_eq!(detail.files[0].path, "new.txt");
}
