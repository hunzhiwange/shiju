use super::*;
use crate::app::agent::bus;
use crate::app::agent::file::watcher as file_watcher;
use git2::Repository;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::TempDir;

fn repo_with_initial_commit() -> (TempDir, Repository) {
    let temp = TempDir::new().expect("tempdir should create");
    let repo = Repository::init(temp.path()).expect("repo should init");
    let sig = git2::Signature::now("Vibe Window", "vw@example.com").expect("signature");
    let tree_id = {
        let mut index = repo.index().expect("index");
        index.write_tree().expect("tree")
    };
    let tree = repo.find_tree(tree_id).expect("tree should load");
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).expect("commit should write");
    drop(tree);
    (temp, repo)
}

#[cfg(not(target_arch = "wasm32"))]
fn stop_watcher(temp: &TempDir) {
    file_watcher::stop(temp.path());
}

#[cfg(target_arch = "wasm32")]
fn stop_watcher(_temp: &TempDir) {}

#[test]
fn vcs_info_default_has_no_branch() {
    let info = Info { branch: None };
    assert!(info.branch.is_none());
}

#[test]
fn current_branch_returns_none_outside_git_repo() {
    let temp = TempDir::new().expect("tempdir should create");
    assert!(current_branch(&temp.path().to_string_lossy()).is_none());
}

#[test]
fn current_branch_returns_none_for_missing_worktree() {
    let temp = TempDir::new().expect("tempdir should create");
    let missing = temp.path().join("missing");

    assert!(current_branch(&missing.to_string_lossy()).is_none());
}

#[test]
fn vcs_info_serializes_branch_and_event_type_is_stable() {
    let info = Info { branch: Some("main".to_string()) };
    let value = serde_json::to_value(&info).expect("info should serialize");
    assert_eq!(value["branch"], "main");
    assert_eq!(event::BRANCH_UPDATED.r#type, "vcs.branch.updated");
}

#[test]
fn vcs_info_omits_and_deserializes_missing_branch() {
    let value = serde_json::to_value(Info { branch: None }).expect("info should serialize");
    assert_eq!(value, json!({}));

    let info: Info = serde_json::from_value(json!({})).expect("info should deserialize");
    assert!(info.branch.is_none());
}

#[test]
fn current_branch_reads_checked_out_git_branch() {
    let (temp, _repo) = repo_with_initial_commit();

    let branch = current_branch(&temp.path().to_string_lossy()).expect("branch should read");
    assert!(!branch.is_empty());
    assert_ne!(branch, "HEAD");
}

#[test]
fn current_branch_reports_detached_head_literal() {
    let (temp, repo) = repo_with_initial_commit();
    let commit = repo.head().expect("head should exist").peel_to_commit().expect("head commit");
    repo.set_head_detached(commit.id()).expect("head should detach");

    assert_eq!(current_branch(&temp.path().to_string_lossy()).as_deref(), Some("HEAD"));
}

#[test]
fn current_branch_preserves_unicode_branch_name() {
    let (temp, repo) = repo_with_initial_commit();
    let commit = repo.head().expect("head should exist").peel_to_commit().expect("head commit");
    repo.branch("feature/分支", &commit, false).expect("branch should create");
    repo.set_head("refs/heads/feature/分支").expect("head should switch");

    assert_eq!(current_branch(&temp.path().to_string_lossy()).as_deref(), Some("feature/分支"));
}

#[tokio::test]
async fn init_returns_shared_info_without_branch_for_non_git_worktree() {
    let temp = TempDir::new().expect("tempdir should create");

    let info = init(temp.path().to_path_buf()).await;
    stop_watcher(&temp);

    assert!(info.lock().expect("info lock").branch.is_none());
}

#[tokio::test]
async fn init_returns_shared_info_with_initial_branch() {
    let (temp, _repo) = repo_with_initial_commit();

    let info = init(temp.path().to_path_buf()).await;
    stop_watcher(&temp);
    let branch = info.lock().expect("info lock").branch.clone();
    assert!(branch.is_some());
}

#[tokio::test]
async fn init_ignores_watcher_events_without_head_file() {
    let (temp, _repo) = repo_with_initial_commit();
    let info = init(temp.path().to_path_buf()).await;
    let initial = info.lock().expect("info lock").branch.clone();

    bus::publish_value(file_watcher::event::UPDATED, serde_json::Value::Null, None);
    bus::publish_value(
        file_watcher::event::UPDATED,
        json!({"file": "src/lib.rs", "event": "change"}),
        None,
    );
    tokio::time::sleep(Duration::from_millis(25)).await;
    stop_watcher(&temp);

    assert_eq!(info.lock().expect("info lock").branch, initial);
}

#[tokio::test]
async fn init_updates_info_and_publishes_when_head_changes() {
    let (temp, repo) = repo_with_initial_commit();
    let info = init(temp.path().to_path_buf()).await;
    let commit = repo.head().expect("head should exist").peel_to_commit().expect("head commit");
    repo.branch("feature-vcs-update", &commit, false).expect("branch should create");
    repo.set_head("refs/heads/feature-vcs-update").expect("head should switch");

    let seen = Arc::new(Mutex::new(Vec::new()));
    let slot = Arc::clone(&seen);
    let unsubscribe = bus::subscribe(event::BRANCH_UPDATED, move |payload| {
        if payload["properties"]["branch"] == "feature-vcs-update" {
            slot.lock().expect("seen lock").push(payload);
        }
    });

    bus::publish_value(
        file_watcher::event::UPDATED,
        json!({"file": ".git/HEAD", "event": "change"}),
        None,
    );

    for _ in 0..20 {
        let branch = info.lock().expect("info lock").branch.clone();
        let updated = branch.as_deref() == Some("feature-vcs-update");
        let published = !seen.lock().expect("seen lock").is_empty();
        if updated && published {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }

    unsubscribe();
    stop_watcher(&temp);

    assert_eq!(info.lock().expect("info lock").branch.as_deref(), Some("feature-vcs-update"));
    assert!(!seen.lock().expect("seen lock").is_empty());
}

#[tokio::test]
async fn init_does_not_publish_when_head_event_keeps_same_branch() {
    let (temp, repo) = repo_with_initial_commit();
    let commit = repo.head().expect("head should exist").peel_to_commit().expect("head commit");
    repo.branch("feature-vcs-unchanged", &commit, false).expect("branch should create");
    repo.set_head("refs/heads/feature-vcs-unchanged").expect("head should switch");
    let info = init(temp.path().to_path_buf()).await;

    let seen = Arc::new(Mutex::new(Vec::new()));
    let slot = Arc::clone(&seen);
    let unsubscribe = bus::subscribe(event::BRANCH_UPDATED, move |payload| {
        if payload["properties"]["branch"] == "feature-vcs-unchanged" {
            slot.lock().expect("seen lock").push(payload);
        }
    });
    bus::publish_value(
        file_watcher::event::UPDATED,
        json!({"file": ".git/HEAD", "event": "change"}),
        None,
    );
    tokio::time::sleep(Duration::from_millis(50)).await;

    unsubscribe();
    stop_watcher(&temp);

    assert_eq!(info.lock().expect("info lock").branch.as_deref(), Some("feature-vcs-unchanged"));
    assert!(seen.lock().expect("seen lock").is_empty());
}

#[tokio::test]
async fn init_updates_branch_to_none_when_repository_disappears() {
    let (temp, repo) = repo_with_initial_commit();
    let info = init(temp.path().to_path_buf()).await;
    assert!(info.lock().expect("info lock").branch.is_some());

    drop(repo);
    std::fs::rename(temp.path().join(".git"), temp.path().join(".git-disabled"))
        .expect("repository metadata should move");
    bus::publish_value(
        file_watcher::event::UPDATED,
        json!({"file": ".git/HEAD", "event": "change"}),
        None,
    );

    for _ in 0..20 {
        if info.lock().expect("info lock").branch.is_none() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    stop_watcher(&temp);

    assert!(info.lock().expect("info lock").branch.is_none());
}

#[tokio::test]
async fn init_recovers_poisoned_info_lock_when_head_changes() {
    let (temp, repo) = repo_with_initial_commit();
    let info = init(temp.path().to_path_buf()).await;
    let poisoned = Arc::clone(&info);
    let _ = std::panic::catch_unwind(move || {
        let _guard = poisoned.lock().expect("info lock should start healthy");
        panic!("poison vcs info lock");
    });

    let commit = repo.head().expect("head should exist").peel_to_commit().expect("head commit");
    repo.branch("feature-vcs-poison", &commit, false).expect("branch should create");
    repo.set_head("refs/heads/feature-vcs-poison").expect("head should switch");
    bus::publish_value(
        file_watcher::event::UPDATED,
        json!({"file": ".git/HEAD", "event": "change"}),
        None,
    );

    for _ in 0..20 {
        let branch = info.lock().unwrap_or_else(|error| error.into_inner()).branch.clone();
        if branch.as_deref() == Some("feature-vcs-poison") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    stop_watcher(&temp);

    assert_eq!(
        info.lock().unwrap_or_else(|error| error.into_inner()).branch.as_deref(),
        Some("feature-vcs-poison")
    );
}
