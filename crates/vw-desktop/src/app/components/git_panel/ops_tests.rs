//! 验证仓库操作对分支、工作树、差异内容和变更文件的处理边界。
//! 临时仓库 fixture 组合 HEAD、索引与工作区状态，避免依赖用户仓库。

#[cfg(not(target_arch = "wasm32"))]
use super::ops::{get_diff_file_metas, git_repo_path_for_app, load_diff_content_for_repo_path};
#[cfg(not(target_arch = "wasm32"))]
use crate::app::components::git_panel::utils::FileStatus;
#[cfg(not(target_arch = "wasm32"))]
use session_info::{Info, TimeInfo};

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
fn repo_path_ignores_unusable_active_session_before_project_fallback() {
    let mut app = crate::app::App::new().0;
    let project = tempfile::tempdir().expect("temporary repository");
    let _repo = git2::Repository::init(project.path()).expect("init repository");
    app.project_path = Some(project.path().to_string_lossy().into_owned());
    app.active_session_id = Some("active".to_string());
    app.sessions.push(session_info("active", "   "));

    assert_eq!(git_repo_path_for_app(&app), app.project_path);

    app.sessions[0].directory = "/missing/session/repository".to_string();
    assert_eq!(git_repo_path_for_app(&app), app.project_path);

    app.active_session_id = Some("not-present".to_string());
    assert_eq!(git_repo_path_for_app(&app), app.project_path);
}

#[cfg(not(target_arch = "wasm32"))]
// 以下辅助函数只操作临时仓库，用最少提交步骤制造索引和工作区差异。
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
        fork_execution_started: None,
        revert: None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn repo_path_rejects_non_git_project_and_accepts_new_repository() {
    let project = tempfile::tempdir().expect("temporary project");
    let mut app = crate::app::App::new().0;
    app.project_path = Some(project.path().to_string_lossy().into_owned());
    assert_eq!(git_repo_path_for_app(&app), None);
    let _repo = git2::Repository::init(project.path()).expect("init repository");
    assert_eq!(git_repo_path_for_app(&app), app.project_path);
}
