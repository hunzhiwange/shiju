#[cfg(not(target_arch = "wasm32"))]
use super::ops::{
    DiffFileMeta, checkout_branch, current_branch, get_changed_file_paths, get_diff_file_metas,
    get_diff_file_metas_for_repo_path, git_repo_path_for_app, list_branches,
    load_diff_content_for_repo_path,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::components::git_panel::utils::FileStatus;
#[cfg(not(target_arch = "wasm32"))]
use vw_shared::session::info::{Info, TimeInfo};

#[test]
fn task_721_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("ops_tests.rs"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn diff_content_uses_head_baseline_when_index_also_changed() {
    let temp = tempfile::tempdir().expect("temp repo");
    let repo = git2::Repository::init(temp.path()).expect("init repo");
    let file_path = temp.path().join("note.md");
    std::fs::write(&file_path, "alpha\nbase\nomega\n").expect("write head file");

    commit_file(&repo, "note.md", "initial");

    std::fs::write(&file_path, "alpha\nstaged\nomega\n").expect("write index file");
    stage_path(&repo, "note.md");

    std::fs::write(&file_path, "alpha\nworktree\nomega\n").expect("write worktree file");

    let metas = get_diff_file_metas_for_repo_path(temp.path().to_str().expect("utf8 path"));
    let meta = metas.iter().find(|meta| meta.path == "note.md").expect("changed file");
    let (old_content, new_content) =
        load_diff_content_for_repo_path(temp.path().to_str().expect("utf8 path"), meta);

    assert_eq!(old_content, "alpha\nbase\nomega\n");
    assert_eq!(new_content, "alpha\nworktree\nomega\n");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn branch_helpers_handle_current_listing_checkout_and_detached_head() {
    let temp = tempfile::tempdir().expect("temp repo");
    let repo = git2::Repository::init(temp.path()).expect("init repo");
    std::fs::write(temp.path().join("note.md"), "initial\n").expect("write file");
    commit_file(&repo, "note.md", "initial");

    let path = temp.path().to_str().expect("utf8 path");
    let initial_branch = current_branch(path).expect("initial branch");
    assert!(list_branches(path).expect("branches").contains(&initial_branch));

    repo.branch("feature", &repo.head().expect("head").peel_to_commit().expect("commit"), false)
        .expect("branch");
    checkout_branch(path, "feature").expect("checkout feature");
    assert_eq!(current_branch(path), Some("feature".to_string()));

    let head_commit = repo.head().expect("head").peel_to_commit().expect("commit");
    repo.set_head_detached(head_commit.id()).expect("detached");
    assert_eq!(current_branch(path), None);

    assert!(checkout_branch(path, "missing").is_err());
    assert!(list_branches(temp.path().join("missing").to_str().expect("utf8 path")).is_err());
    assert_eq!(current_branch(temp.path().join("missing").to_str().expect("utf8 path")), None);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn repo_path_prefers_selected_worktree_and_falls_back_to_project_path() {
    let temp = tempfile::tempdir().expect("temp repo");
    let repo = git2::Repository::init(temp.path()).expect("init repo");
    std::fs::write(temp.path().join("note.md"), "initial\n").expect("write file");
    commit_file(&repo, "note.md", "initial");

    let mut app = crate::app::App::new().0;
    app.project_path = Some("/not/validated/by/helper".to_string());
    app.selected_git_worktree_directory = Some(temp.path().to_string_lossy().to_string());
    assert_eq!(git_repo_path_for_app(&app), Some(temp.path().to_string_lossy().to_string()));

    app.selected_git_worktree_directory = Some("   ".to_string());
    assert_eq!(git_repo_path_for_app(&app), None);

    app.selected_git_worktree_directory = None;
    app.project_path = Some(temp.path().to_string_lossy().to_string());
    assert_eq!(git_repo_path_for_app(&app), Some(temp.path().to_string_lossy().to_string()));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn diff_metas_cover_modified_added_untracked_deleted_and_content_loading() {
    let temp = tempfile::tempdir().expect("temp repo");
    let repo = git2::Repository::init(temp.path()).expect("init repo");
    std::fs::write(temp.path().join("modified.txt"), "old\nsame\n").expect("write modified");
    std::fs::write(temp.path().join("deleted.txt"), "gone\n").expect("write deleted");
    std::fs::write(temp.path().join("staged_added.txt"), "added\n").expect("write added");
    commit_file(&repo, "modified.txt", "initial modified");
    stage_path(&repo, "deleted.txt");
    let tree_oid = repo.index().expect("index").write_tree().expect("write tree");
    let tree = repo.find_tree(tree_oid).expect("tree");
    let signature = git2::Signature::now("VibeWindow Test", "test@example.com").expect("signature");
    let parent = repo.head().expect("head").peel_to_commit().expect("parent");
    repo.commit(Some("HEAD"), &signature, &signature, "add deleted", &tree, &[&parent])
        .expect("commit");

    std::fs::write(temp.path().join("modified.txt"), "new\nsame\n").expect("modify");
    std::fs::remove_file(temp.path().join("deleted.txt")).expect("delete");
    stage_path(&repo, "staged_added.txt");
    std::fs::write(temp.path().join("untracked.txt"), "loose\nfile\n").expect("untracked");

    let metas = get_diff_file_metas_for_repo_path(temp.path().to_str().expect("utf8 path"));
    let status_for = |path: &str| {
        metas.iter().find(|meta| meta.path == path).map(|meta| meta.status).expect(path)
    };

    assert_eq!(status_for("modified.txt"), FileStatus::Modified);
    assert_eq!(status_for("deleted.txt"), FileStatus::Deleted);
    assert_eq!(status_for("staged_added.txt"), FileStatus::Added);
    assert_eq!(status_for("untracked.txt"), FileStatus::Untracked);

    let deleted = metas.iter().find(|meta| meta.path == "deleted.txt").expect("deleted");
    assert!(!deleted.new_exists);
    assert!(deleted.deletions >= 1);
    let (old_deleted, new_deleted) =
        load_diff_content_for_repo_path(temp.path().to_str().expect("utf8 path"), deleted);
    assert_eq!(old_deleted, "gone\n");
    assert_eq!(new_deleted, "");

    let changed = get_changed_file_paths(temp.path().to_str().expect("utf8 path"));
    assert!(changed.contains(&"modified.txt".to_string()));
    assert!(changed.contains(&"deleted.txt".to_string()));
    assert!(changed.contains(&"staged_added.txt".to_string()));
    assert!(changed.contains(&"untracked.txt".to_string()));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn changed_paths_filter_ignored_directories_and_ds_store() {
    let temp = tempfile::tempdir().expect("temp repo");
    let repo = git2::Repository::init(temp.path()).expect("init repo");
    std::fs::write(temp.path().join("tracked.txt"), "base\n").expect("tracked");
    commit_file(&repo, "tracked.txt", "initial");

    std::fs::create_dir_all(temp.path().join("node_modules/pkg")).expect("node_modules");
    std::fs::create_dir_all(temp.path().join("target/debug")).expect("target");
    std::fs::write(temp.path().join("tracked.txt"), "changed\n").expect("changed");
    std::fs::write(temp.path().join("node_modules/pkg/index.js"), "ignored\n").expect("node");
    std::fs::write(temp.path().join("target/debug/file"), "ignored\n").expect("target file");
    std::fs::write(temp.path().join(".DS_Store"), "ignored\n").expect("ds store");

    let changed = get_changed_file_paths(temp.path().to_str().expect("utf8 path"));

    assert_eq!(changed, vec!["tracked.txt".to_string()]);
    assert!(
        get_changed_file_paths(temp.path().join("missing").to_str().expect("utf8 path")).is_empty()
    );
    assert!(
        load_diff_content_for_repo_path(
            temp.path().join("missing").to_str().expect("utf8 path"),
            &get_diff_file_metas_for_repo_path(temp.path().to_str().expect("utf8 path"))[0],
        )
        .0
        .is_empty()
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn repo_path_uses_matching_active_session_and_selected_override_is_authoritative() {
    let session_repo = tempfile::tempdir().expect("session repo");
    git2::Repository::init(session_repo.path()).expect("init session repo");

    let mut app = crate::app::App::new().0;
    app.project_path = Some("/project/fallback".to_string());
    app.active_session_id = Some("active".to_string());
    app.sessions.push(session_info(
        "inactive",
        session_repo.path().join("missing").to_string_lossy().as_ref(),
    ));
    app.sessions.push(session_info("active", session_repo.path().to_string_lossy().as_ref()));

    assert_eq!(
        git_repo_path_for_app(&app),
        Some(session_repo.path().to_string_lossy().to_string())
    );

    app.selected_git_worktree_directory =
        Some(session_repo.path().join("missing").to_string_lossy().to_string());
    assert_eq!(git_repo_path_for_app(&app), None);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn empty_repository_has_no_branch_diff_or_changed_paths() {
    let temp = tempfile::tempdir().expect("temp repo");
    git2::Repository::init(temp.path()).expect("init repo");
    let path = temp.path().to_str().expect("utf8 path");

    assert_eq!(current_branch(path), None);
    assert!(list_branches(path).expect("empty branch list").is_empty());
    assert!(get_diff_file_metas_for_repo_path(path).is_empty());
    assert!(get_changed_file_paths(path).is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn diff_metas_report_sizes_line_counts_and_app_path_content() {
    let temp = tempfile::tempdir().expect("temp repo");
    let repo = git2::Repository::init(temp.path()).expect("init repo");
    let tracked_path = temp.path().join("tracked.txt");
    std::fs::write(&tracked_path, "old\nsame\n").expect("write tracked");
    commit_file(&repo, "tracked.txt", "initial");

    std::fs::write(&tracked_path, "new\nsame\nextra\n").expect("modify tracked");
    std::fs::write(temp.path().join("untracked.txt"), "first\nsecond\n").expect("write untracked");

    let mut app = crate::app::App::new().0;
    app.selected_git_worktree_directory = Some(temp.path().to_string_lossy().to_string());
    let metas = get_diff_file_metas(&app);

    let tracked = metas.iter().find(|meta| meta.path == "tracked.txt").expect("tracked meta");
    assert_eq!(tracked.status, FileStatus::Modified);
    assert_eq!(tracked.old_size, "old\nsame\n".len() as u64);
    assert_eq!(tracked.new_size, "new\nsame\nextra\n".len() as u64);
    assert!(tracked.old_oid.is_some());
    assert!(tracked.new_exists);
    assert_eq!((tracked.insertions, tracked.deletions), (2, 1));
    assert_eq!(
        load_diff_content_for_repo_path(temp.path().to_str().expect("utf8 path"), tracked),
        ("old\nsame\n".to_string(), "new\nsame\nextra\n".to_string())
    );

    let untracked = metas.iter().find(|meta| meta.path == "untracked.txt").expect("untracked meta");
    assert_eq!(untracked.status, FileStatus::Untracked);
    assert_eq!(untracked.old_oid, None);
    assert_eq!(untracked.old_size, 0);
    assert!(untracked.new_exists);
    assert_eq!((untracked.insertions, untracked.deletions), (2, 0));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn diff_content_defaults_to_empty_for_missing_blob_and_worktree_file() {
    let temp = tempfile::tempdir().expect("temp repo");
    git2::Repository::init(temp.path()).expect("init repo");
    let meta = DiffFileMeta {
        path: "missing.txt".to_string(),
        status: FileStatus::Modified,
        old_oid: Some(git2::Oid::ZERO_SHA1),
        old_size: 0,
        new_size: 0,
        new_exists: true,
        insertions: 0,
        deletions: 0,
    };

    assert_eq!(
        load_diff_content_for_repo_path(temp.path().to_str().expect("utf8 path"), &meta),
        (String::new(), String::new())
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn branch_listing_excludes_remote_tracking_references() {
    let temp = tempfile::tempdir().expect("temp repo");
    let repo = git2::Repository::init(temp.path()).expect("init repo");
    std::fs::write(temp.path().join("note.md"), "initial\n").expect("write file");
    commit_file(&repo, "note.md", "initial");

    let head = repo.head().expect("head").peel_to_commit().expect("commit");
    repo.branch("local-review", &head, false).expect("local branch");
    repo.reference("refs/remotes/origin/review", head.id(), true, "test remote reference")
        .expect("remote reference");

    let branches = list_branches(temp.path().to_str().expect("utf8 path")).expect("branches");
    assert!(branches.contains(&"local-review".to_string()));
    assert!(!branches.contains(&"origin/review".to_string()));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn repo_path_ignores_unusable_active_session_before_project_fallback() {
    let mut app = crate::app::App::new().0;
    app.project_path = Some("/project/fallback".to_string());
    app.active_session_id = Some("active".to_string());
    app.sessions.push(session_info("active", "   "));

    assert_eq!(git_repo_path_for_app(&app), app.project_path);

    app.sessions[0].directory = "/missing/session/repository".to_string();
    assert_eq!(git_repo_path_for_app(&app), app.project_path);

    app.active_session_id = Some("not-present".to_string());
    assert_eq!(git_repo_path_for_app(&app), app.project_path);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn diff_content_refuses_absent_or_deleted_worktree_content() {
    let temp = tempfile::tempdir().expect("temp repo");
    git2::Repository::init(temp.path()).expect("init repo");
    std::fs::write(temp.path().join("present.txt"), "worktree\n").expect("write worktree file");
    let path = temp.path().to_str().expect("utf8 path");
    let mut meta = DiffFileMeta {
        path: "present.txt".to_string(),
        status: FileStatus::Modified,
        old_oid: None,
        old_size: 0,
        new_size: 0,
        new_exists: false,
        insertions: 0,
        deletions: 0,
    };

    assert_eq!(load_diff_content_for_repo_path(path, &meta), (String::new(), String::new()));

    meta.new_exists = true;
    meta.status = FileStatus::Deleted;
    assert_eq!(load_diff_content_for_repo_path(path, &meta), (String::new(), String::new()));
}

#[cfg(not(target_arch = "wasm32"))]
fn commit_file(repo: &git2::Repository, path: &str, message: &str) {
    stage_path(repo, path);
    let tree_oid = repo.index().expect("index").write_tree().expect("write tree");
    let tree = repo.find_tree(tree_oid).expect("tree");
    let signature = git2::Signature::now("VibeWindow Test", "test@example.com").expect("signature");
    repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[]).expect("commit");
}

#[cfg(not(target_arch = "wasm32"))]
fn stage_path(repo: &git2::Repository, path: &str) {
    let mut index = repo.index().expect("index");
    index.add_path(std::path::Path::new(path)).expect("add path");
    index.write().expect("write index");
}

#[cfg(not(target_arch = "wasm32"))]
fn session_info(id: &str, directory: &str) -> Info {
    Info {
        id: id.to_string(),
        slug: format!("{id}-slug"),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: format!("{id} title"),
        version: "1".to_string(),
        time: TimeInfo { created: 1, updated: 2, compacting: None, archived: None },
        permission: None,
        revert: None,
    }
}
