use super::*;
use base64::Engine as _;
use std::ffi::OsString;
use uuid::Uuid;

struct PathGuard {
    old: Option<OsString>,
    _bin_dir: tempfile::TempDir,
}

impl PathGuard {
    /// 用仅含失败 stub 的 PATH 屏蔽真实 `git` 可执行文件。
    ///
    /// 仅清空 PATH 不够：`git_std_command` 会通过补全 PATH / 常见目录
    /// 再次解析到系统 Git。
    fn without_git() -> Self {
        let bin_dir = tempfile::TempDir::new().expect("git stub bin dir");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let stub = bin_dir.path().join("git");
            std::fs::write(&stub, "#!/bin/sh\necho 'git stub: blocked' >&2\nexit 1\n")
                .expect("write git stub");
            let mut perms = std::fs::metadata(&stub).expect("stub metadata").permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&stub, perms).expect("stub permissions");
        }
        #[cfg(windows)]
        {
            let stub = bin_dir.path().join("git.cmd");
            std::fs::write(&stub, "@echo off\r\necho git stub: blocked 1>&2\r\nexit /b 1\r\n")
                .expect("write git stub");
        }
        let old = std::env::var_os("PATH");
        unsafe { std::env::set_var("PATH", bin_dir.path()) };
        Self { old, _bin_dir: bin_dir }
    }
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        match &self.old {
            Some(value) => unsafe { std::env::set_var("PATH", value) },
            None => unsafe { std::env::remove_var("PATH") },
        }
    }
}

/// 测试夹具专用 CLI helper；生产 commit/merge 路径走 git2，不经过本函数。
fn run_git(directory: &str, args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .current_dir(directory)
        .args(args)
        .output()
        .map_err(|err| err.to_string())?;
    let stdout =
        String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n").replace('\r', "\n");
    let stderr =
        String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n").replace('\r', "\n");
    if output.status.success() {
        return Ok(stdout);
    }
    let detail = if stderr.trim().is_empty() {
        stdout.trim().to_string()
    } else {
        stderr.trim().to_string()
    };
    if detail.is_empty() {
        return Err(format!("git command failed: {}", args.join(" ")));
    }
    Err(detail)
}

#[test]
fn router_builds_with_unit_state() {
    let _ = router::<()>();
}

#[test]
fn normalize_path_trims_slashes_and_current_directory() {
    assert_eq!(normalize_path("./src/main.rs"), "src/main.rs");
    assert_eq!(normalize_path("/src/main.rs"), "src/main.rs");
    assert_eq!(normalize_path("src\\main.rs\\"), "src/main.rs");
}

#[test]
fn normalize_path_handles_empty_root_and_trailing_separators() {
    assert_eq!(normalize_path(""), "");
    assert_eq!(normalize_path("/"), "");
    assert_eq!(normalize_path("///repo/path///"), "repo/path");
    assert_eq!(normalize_path(".\\repo\\path\\"), "repo/path");
}

#[test]
fn project_context_directory_prefers_worktree_then_non_empty_sandbox() {
    let primary = test_project_info("/repo/main", vec!["/repo/sandbox"], Some(project::Vcs::Git));
    assert_eq!(project_context_directory(&primary).expect("primary directory"), "/repo/main");

    let fallback =
        test_project_info("  ", vec!["", "  ", "/repo/sandbox"], Some(project::Vcs::Git));
    assert_eq!(project_context_directory(&fallback).expect("sandbox directory"), "/repo/sandbox");

    let missing = test_project_info("", vec!["", "  "], None);
    let error = project_context_directory(&missing).expect_err("missing directory");
    assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.message, "project directory missing");
}

#[test]
fn directory_from_worktree_id_decodes_utf8_and_rejects_invalid_payloads() {
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("/repo/worktree");
    assert_eq!(directory_from_worktree_id(&encoded).expect("decoded path"), "/repo/worktree");

    let malformed = directory_from_worktree_id("%%%")
        .expect_err("malformed base64 should be hidden as not found");
    assert_eq!(malformed.status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(malformed.message, "worktree not found");

    let invalid_utf8 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([0xff]);
    let error = directory_from_worktree_id(&invalid_utf8)
        .expect_err("non-utf8 payload should be hidden as not found");
    assert_eq!(error.status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(error.message, "worktree not found");
}

#[test]
fn gateway_git2_commit_works_without_git_executable() {
    let _path = PathGuard::without_git();
    let repo = init_repo_git2();

    // Initial commit on an unborn repository.
    std::fs::write(repo.path().join("created.txt"), "created\n").expect("created file");
    let mut initial = empty_commit_request();
    initial.message = "initial".to_string();
    initial.stage_all = true;
    let initial_response =
        commit_selected_blocking(repo.path_str(), &initial).expect("initial commit");
    assert!(initial_response.ok);
    assert!(initial_response.created);
    assert_eq!(initial_response.commit.message, "initial");
    assert_eq!(initial_response.commit.sha.len(), 40);
    assert_eq!(
        blob_at_commit(repo.path(), &initial_response.commit.sha, "created.txt"),
        "created\n"
    );

    // Full commit of a subsequent change.
    std::fs::write(repo.path().join("created.txt"), "updated\n").expect("updated file");
    let mut full = empty_commit_request();
    full.message = "full update".to_string();
    full.stage_all = true;
    let full_response = commit_selected_blocking(repo.path_str(), &full).expect("full commit");
    assert!(full_response.ok);
    assert!(full_response.created);
    assert_eq!(full_response.commit.message, "full update");
    assert_eq!(
        blob_at_commit(repo.path(), &full_response.commit.sha, "created.txt"),
        "updated\n"
    );

    // No staged changes keeps created=false and HEAD unchanged.
    let head = full_response.commit.sha.clone();
    let mut noop = empty_commit_request();
    noop.message = "noop".to_string();
    noop.stage_all = true;
    let noop_response = commit_selected_blocking(repo.path_str(), &noop).expect("noop commit");
    assert!(noop_response.ok);
    assert!(!noop_response.created);
    assert_eq!(noop_response.commit.sha, head);
    assert_eq!(noop_response.commit.message, "noop");
}

#[test]
fn gateway_git2_partial_commit_preserves_unselected_changes() {
    let _path = PathGuard::without_git();
    let repo = init_repo_git2();
    let hunk_old = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n";
    let hunk_new = "ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nTEN\n";
    write_and_commit_all_git2(
        repo.path(),
        &[
            ("selected.txt", "base-selected\n"),
            ("other.txt", "base-other\n"),
            ("lines.txt", "alpha\nold\nomega\n"),
            ("hunks.txt", hunk_old),
        ],
        "initial",
    );

    std::fs::write(repo.path().join("selected.txt"), "selected-new\n").expect("selected change");
    std::fs::write(repo.path().join("other.txt"), "other-new\n").expect("other change");
    std::fs::write(repo.path().join("lines.txt"), "alpha\nnew\nomega\n").expect("lines change");
    std::fs::write(repo.path().join("hunks.txt"), hunk_new).expect("hunks change");

    let mut request = empty_commit_request();
    request.message = "partial".to_string();
    request.selected_files = vec!["selected.txt".to_string()];
    request.selected_lines =
        vec![vw_api_types::git::GitLineSelectionDto { path: "lines.txt".to_string(), line: 1 }];
    request.selected_hunks =
        vec![vw_api_types::git::GitHunkSelectionDto { path: "hunks.txt".to_string(), index: 0 }];

    let response = commit_selected_blocking(repo.path_str(), &request).expect("partial commit");
    assert!(response.created);
    assert_eq!(response.commit.message, "partial");

    // File selection commits the full working-tree content.
    assert_eq!(
        blob_at_commit(repo.path(), &response.commit.sha, "selected.txt"),
        "selected-new\n"
    );
    // Line selection of the inserted line keeps the unselected old line.
    assert_eq!(
        blob_at_commit(repo.path(), &response.commit.sha, "lines.txt"),
        "alpha\nnew\nold\nomega\n"
    );
    // Hunk 0 selects only the first distant change.
    assert_eq!(
        blob_at_commit(repo.path(), &response.commit.sha, "hunks.txt"),
        "ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n"
    );
    // Completely unselected file stays at the previous committed content.
    assert_eq!(
        blob_at_commit(repo.path(), &response.commit.sha, "other.txt"),
        "base-other\n"
    );

    // Unselected working-tree changes remain dirty.
    assert_eq!(
        std::fs::read_to_string(repo.path().join("other.txt")).expect("other worktree"),
        "other-new\n"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("hunks.txt")).expect("hunks worktree"),
        hunk_new
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("lines.txt")).expect("lines worktree"),
        "alpha\nnew\nomega\n"
    );
}

#[test]
fn gateway_git2_merge_works_without_git_executable() {
    let _path = PathGuard::without_git();
    let repo = init_repo_git2();

    // Shared base commit on main.
    write_and_commit_all_git2(repo.path(), &[("base.txt", "base\n")], "initial");

    // Already-merged: feature points at the same commit as main.
    create_branch_git2(repo.path(), "feature-already");
    let already =
        merge_branch_blocking(repo.path_str(), "feature-already", "main").expect("already merged");
    assert!(already.ok);
    assert!(already.already_merged);
    assert_eq!(already.source_branch, "feature-already");
    assert_eq!(already.target_branch, "main");
    assert_eq!(already.message, "feature-already already merged into main");
    assert!(git_is_clean(repo.path_str()).expect("clean after already-merged"));

    // Fast-forward: feature is strictly ahead of main.
    create_branch_git2(repo.path(), "feature-ff");
    checkout_branch_git2(repo.path(), "feature-ff");
    write_and_commit_all_git2(repo.path(), &[("ff.txt", "ff\n")], "ff commit");
    let feature_ff_oid = head_oid_git2(repo.path());
    checkout_branch_git2(repo.path(), "main");
    let main_before_ff = head_oid_git2(repo.path());
    let ff = merge_branch_blocking(repo.path_str(), "feature-ff", "main").expect("fast-forward");
    assert!(ff.ok);
    assert!(!ff.already_merged);
    assert_eq!(ff.message, "merged feature-ff into main");
    assert_eq!(head_oid_git2(repo.path()), feature_ff_oid);
    assert_ne!(head_oid_git2(repo.path()), main_before_ff);
    assert_eq!(commit_parent_count_git2(repo.path(), &feature_ff_oid), 1);
    assert_eq!(blob_at_commit(repo.path(), &feature_ff_oid, "ff.txt"), "ff\n");

    // Two-parent merge: main and feature diverge.
    create_branch_git2(repo.path(), "feature-merge");
    checkout_branch_git2(repo.path(), "feature-merge");
    write_and_commit_all_git2(repo.path(), &[("feature-only.txt", "feature\n")], "feature side");
    let feature_merge_oid = head_oid_git2(repo.path());
    checkout_branch_git2(repo.path(), "main");
    write_and_commit_all_git2(repo.path(), &[("main-only.txt", "main\n")], "main side");
    let main_parent_oid = head_oid_git2(repo.path());
    let merged =
        merge_branch_blocking(repo.path_str(), "feature-merge", "main").expect("two-parent merge");
    assert!(merged.ok);
    assert!(!merged.already_merged);
    assert_eq!(merged.message, "merged feature-merge into main");
    let merge_oid = head_oid_git2(repo.path());
    assert_eq!(commit_parent_count_git2(repo.path(), &merge_oid), 2);
    assert_eq!(
        commit_parent_oids_git2(repo.path(), &merge_oid),
        vec![main_parent_oid, feature_merge_oid]
    );
    assert_eq!(blob_at_commit(repo.path(), &merge_oid, "feature-only.txt"), "feature\n");
    assert_eq!(blob_at_commit(repo.path(), &merge_oid, "main-only.txt"), "main\n");
    assert_eq!(
        commit_message_git2(repo.path(), &merge_oid),
        "chore(task): merge feature-merge into main"
    );

    // Dirty target auto-save before merge.
    create_branch_git2(repo.path(), "feature-dirty");
    checkout_branch_git2(repo.path(), "feature-dirty");
    write_and_commit_all_git2(repo.path(), &[("dirty-feature.txt", "feature\n")], "dirty feature");
    checkout_branch_git2(repo.path(), "main");
    std::fs::write(repo.path().join("dirty-target.txt"), "target\n").expect("dirty target");
    let dirty =
        merge_branch_blocking(repo.path_str(), "feature-dirty", "main").expect("dirty auto-save");
    assert!(dirty.ok);
    assert!(!dirty.already_merged);
    assert!(dirty.message.contains("saved target branch changes"));
    assert!(git_is_clean(repo.path_str()).expect("clean after dirty merge"));
    let dirty_head = head_oid_git2(repo.path());
    assert_eq!(blob_at_commit(repo.path(), &dirty_head, "dirty-target.txt"), "target\n");
    assert_eq!(blob_at_commit(repo.path(), &dirty_head, "dirty-feature.txt"), "feature\n");
    assert!(
        commit_message_git2(repo.path(), &dirty_head)
            .contains("chore(task): merge feature-dirty into main")
    );
    let save_oid = commit_parent_oids_git2(repo.path(), &dirty_head)[0].clone();
    assert!(
        commit_message_git2(repo.path(), &save_oid)
            .contains("chore(task): save main changes before merging feature-dirty")
    );
}

#[test]
fn gateway_git2_merge_conflict_restores_target_state() {
    let _path = PathGuard::without_git();
    let repo = init_repo_git2();
    write_and_commit_all_git2(repo.path(), &[("conflict.txt", "base\n")], "initial");

    create_branch_git2(repo.path(), "feature");
    checkout_branch_git2(repo.path(), "feature");
    write_and_commit_all_git2(repo.path(), &[("conflict.txt", "feature\n")], "feature change");

    checkout_branch_git2(repo.path(), "main");
    write_and_commit_all_git2(repo.path(), &[("conflict.txt", "main\n")], "main change");
    let target_oid = head_oid_git2(repo.path());

    let error = merge_branch_blocking(repo.path_str(), "feature", "main")
        .expect_err("conflict should fail");
    assert!(!error.trim().is_empty());

    assert!(!repo.path().join(".git").join("MERGE_HEAD").exists());
    assert!(git_is_clean(repo.path_str()).expect("clean after conflict restore"));
    assert_eq!(head_oid_git2(repo.path()), target_oid);
    assert_eq!(current_branch_git2(repo.path()), "main");
    assert_eq!(
        std::fs::read_to_string(repo.path().join("conflict.txt")).expect("restored worktree"),
        "main\n"
    );
    assert_eq!(blob_at_commit(repo.path(), &target_oid, "conflict.txt"), "main\n");
}

#[test]
fn gateway_git2_core_flow_works_without_git_executable() {
    let _path = PathGuard::without_git();
    let repo = init_repo_git2();

    // 项目发现 / 分支元数据：无系统 Git 时仍可解析。
    let discovered =
        crate::git_repository::discover(repo.path()).expect("discover without git CLI");
    assert_eq!(
        discovered.workdir,
        repo.path().canonicalize().unwrap_or_else(|_| repo.path().to_path_buf())
    );
    let branch =
        crate::git_repository::current_branch(repo.path()).expect("branch without git CLI");
    assert_eq!(branch, "main");

    // Commit via gateway git2 path.
    std::fs::write(repo.path().join("base.txt"), "base\n").expect("base file");
    let mut initial = empty_commit_request();
    initial.message = "initial".to_string();
    initial.stage_all = true;
    let initial_response =
        commit_selected_blocking(repo.path_str(), &initial).expect("initial commit");
    assert!(initial_response.ok);
    assert!(initial_response.created);
    assert_eq!(initial_response.commit.message, "initial");

    // 脏工作区状态元数据同样不依赖系统 Git。
    std::fs::write(repo.path().join("dirty.txt"), "dirty\n").expect("dirty file");
    let status = crate::app::agent::file::status_git(repo.path());
    assert!(
        status.iter().any(|info| info.path == "dirty.txt"),
        "status_git should report untracked file without git CLI"
    );
    std::fs::remove_file(repo.path().join("dirty.txt")).expect("remove dirty");

    // Feature commit + merge via gateway git2 path.
    create_branch_git2(repo.path(), "feature");
    checkout_branch_git2(repo.path(), "feature");
    write_and_commit_all_git2(repo.path(), &[("feature.txt", "feature\n")], "feature commit");
    checkout_branch_git2(repo.path(), "main");
    let merge =
        merge_branch_blocking(repo.path_str(), "feature", "main").expect("merge without git CLI");
    assert!(merge.ok);
    assert!(!merge.already_merged);
    assert_eq!(merge.message, "merged feature into main");
    assert_eq!(
        blob_at_commit(repo.path(), &head_oid_git2(repo.path()), "feature.txt"),
        "feature\n"
    );
    assert!(git_is_clean(repo.path_str()).expect("clean after core flow"));
}

#[tokio::test]
async fn git_commit_v1_rejects_blank_message_before_project_lookup() {
    let mut request = empty_commit_request();
    request.message = " \n\t ".to_string();

    let error = git_commit_v1(Json(request)).await.expect_err("blank commit message");

    assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.message, "commit message is required");
}

#[tokio::test]
async fn git_commit_v1_trims_message_and_commits_registered_project() {
    let repo = init_repo();
    std::fs::write(repo.path().join("created.txt"), "created\n").expect("created file");
    let (project_id, _cleanup) = store_git_test_project(&repo);
    let request = GitCommitRequest {
        project_id: vw_api_types::id::ProjectId(project_id),
        worktree_id: None,
        message: "  add created file  \n".to_string(),
        stage_all: true,
        selected_files: Vec::new(),
        selected_hunks: Vec::new(),
        selected_lines: Vec::new(),
        selected_old_lines: Vec::new(),
    };

    let Json(response) = git_commit_v1(Json(request)).await.expect("commit response");

    assert!(response.ok);
    assert!(response.created);
    assert_eq!(response.commit.message, "add created file");
    assert_eq!(
        run_git(repo.path_str(), &["log", "--format=%s", "-n", "1"]).expect("commit subject"),
        "add created file\n"
    );
}

#[tokio::test]
async fn git_merge_v1_rejects_head_source_before_project_lookup() {
    let request = GitMergeRequest {
        project_id: vw_api_types::id::ProjectId("missing-project".to_string()),
        source_branch: " HEAD ".to_string(),
        target_branch: " main ".to_string(),
    };

    let error = git_merge_v1(Json(request)).await.expect_err("invalid source branch");

    assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(error.message, "source branch is required");
}

#[tokio::test]
async fn git_merge_v1_merges_registered_project_branches() {
    let repo = init_repo();
    run_git(repo.path_str(), &["checkout", "-b", "main"]).expect("main branch");
    std::fs::write(repo.path().join("base.txt"), "base\n").expect("base file");
    run_git(repo.path_str(), &["add", "base.txt"]).expect("add base");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");
    run_git(repo.path_str(), &["checkout", "-b", "feature"]).expect("feature branch");
    std::fs::write(repo.path().join("feature.txt"), "feature\n").expect("feature file");
    run_git(repo.path_str(), &["add", "feature.txt"]).expect("add feature");
    run_git(repo.path_str(), &["commit", "-m", "feature commit"]).expect("feature commit");
    run_git(repo.path_str(), &["checkout", "main"]).expect("checkout main");
    let (project_id, _cleanup) = store_git_test_project(&repo);
    let request = GitMergeRequest {
        project_id: vw_api_types::id::ProjectId(project_id),
        source_branch: " feature ".to_string(),
        target_branch: " main ".to_string(),
    };

    let Json(response) = git_merge_v1(Json(request)).await.expect("merge response");

    assert!(response.ok);
    assert!(!response.already_merged);
    assert_eq!(response.message, "merged feature into main");
    assert_eq!(
        run_git(repo.path_str(), &["show", "--format=", "--name-only", "HEAD"])
            .expect("merged files")
            .trim(),
        "feature.txt"
    );
}

#[tokio::test]
async fn worktree_list_for_non_git_project_is_empty_without_instance_access() {
    let info = test_project_info("/not/accessed", vec!["/also/not/accessed"], None);

    assert_eq!(
        worktree_list_for_project(&info).await.expect("non-git worktrees"),
        Vec::<String>::new()
    );
}

#[tokio::test]
async fn resolve_git_directory_accepts_primary_worktree_and_rejects_unknown_worktree() {
    let repo = init_repo();
    run_git(repo.path_str(), &["checkout", "-b", "main"]).expect("main branch");
    std::fs::write(repo.path().join("base.txt"), "base\n").expect("base file");
    run_git(repo.path_str(), &["add", "base.txt"]).expect("add base");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");
    let (project_id, _cleanup) = store_git_test_project(&repo);
    let encoded_repo =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(repo.path_str().as_bytes());
    let worktree_id = vw_api_types::id::WorktreeId(encoded_repo);

    assert_eq!(
        resolve_git_directory(&project_id, None).await.expect("primary directory"),
        repo.path_str()
    );
    assert_eq!(
        resolve_git_directory(&project_id, Some(&worktree_id)).await.expect("worktree directory"),
        repo.path_str()
    );

    let unknown = tempfile::tempdir().expect("unknown dir");
    let unknown_id = vw_api_types::id::WorktreeId(
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(unknown.path().to_string_lossy().as_bytes()),
    );
    let error =
        resolve_git_directory(&project_id, Some(&unknown_id)).await.expect_err("unknown worktree");
    assert_eq!(error.status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(error.message, "worktree not found");
}

#[test]
fn validate_repo_relative_path_rejects_escape_segments() {
    assert!(validate_repo_relative_path("src/lib.rs").is_ok());
    assert!(validate_repo_relative_path("  ").is_err());
    assert!(validate_repo_relative_path("../secret").is_err());
    assert!(validate_repo_relative_path("/absolute").is_err());
}

#[test]
fn validate_repo_relative_path_rejects_nested_parent_escape() {
    assert!(validate_repo_relative_path("nested/../../secret").is_err());
    assert!(validate_repo_relative_path("nested/../sibling").is_err());
    assert!(validate_repo_relative_path("nested/file.txt").is_ok());
}

#[test]
fn collect_file_selections_merges_files_hunks_and_lines() {
    let request = GitCommitRequest {
        project_id: vw_api_types::id::ProjectId("project".to_string()),
        worktree_id: None,
        message: "commit".to_string(),
        stage_all: false,
        selected_files: vec!["src/lib.rs".to_string()],
        selected_hunks: vec![vw_api_types::git::GitHunkSelectionDto {
            path: "src/lib.rs".to_string(),
            index: 2,
        }],
        selected_lines: vec![vw_api_types::git::GitLineSelectionDto {
            path: "src/lib.rs".to_string(),
            line: 4,
        }],
        selected_old_lines: vec![vw_api_types::git::GitLineSelectionDto {
            path: "old.rs".to_string(),
            line: 1,
        }],
    };

    let selections = collect_file_selections(&request).expect("selections");

    let src = selections.get("src/lib.rs").expect("src selection");
    assert!(src.stage_file);
    assert!(src.hunks.contains(&2));
    assert!(src.new_lines.contains(&4));
    assert!(selections.get("old.rs").expect("old selection").old_lines.contains(&1));
}

#[test]
fn collect_file_selections_rejects_escaping_paths() {
    let request = GitCommitRequest {
        project_id: vw_api_types::id::ProjectId("project".to_string()),
        worktree_id: None,
        message: "commit".to_string(),
        stage_all: false,
        selected_files: vec!["../secret".to_string()],
        selected_hunks: Vec::new(),
        selected_lines: Vec::new(),
        selected_old_lines: Vec::new(),
    };

    assert!(collect_file_selections(&request).is_err());
}

#[test]
fn collect_file_selections_deduplicates_each_selection_kind() {
    let mut request = empty_commit_request();
    request.selected_files = vec!["src/lib.rs".to_string(), "src/lib.rs".to_string()];
    request.selected_hunks = vec![
        vw_api_types::git::GitHunkSelectionDto { path: "src/lib.rs".to_string(), index: 1 },
        vw_api_types::git::GitHunkSelectionDto { path: "src/lib.rs".to_string(), index: 1 },
    ];
    request.selected_lines = vec![
        vw_api_types::git::GitLineSelectionDto { path: "src/lib.rs".to_string(), line: 3 },
        vw_api_types::git::GitLineSelectionDto { path: "src/lib.rs".to_string(), line: 3 },
    ];
    request.selected_old_lines = vec![
        vw_api_types::git::GitLineSelectionDto { path: "src/lib.rs".to_string(), line: 5 },
        vw_api_types::git::GitLineSelectionDto { path: "src/lib.rs".to_string(), line: 5 },
    ];

    let selections = collect_file_selections(&request).expect("deduplicated selections");
    let selection = selections.get("src/lib.rs").expect("file selection");
    assert_eq!(selections.len(), 1);
    assert!(selection.stage_file);
    assert_eq!(selection.hunks.iter().copied().collect::<Vec<_>>(), vec![1]);
    assert_eq!(selection.new_lines.iter().copied().collect::<Vec<_>>(), vec![3]);
    assert_eq!(selection.old_lines.iter().copied().collect::<Vec<_>>(), vec![5]);
}

#[test]
fn collect_file_selections_validates_hunk_new_and_old_line_paths() {
    let mut hunk_request = empty_commit_request();
    hunk_request.selected_hunks =
        vec![vw_api_types::git::GitHunkSelectionDto { path: "../hunk".to_string(), index: 0 }];
    assert!(collect_file_selections(&hunk_request).is_err());

    let mut new_line_request = empty_commit_request();
    new_line_request.selected_lines =
        vec![vw_api_types::git::GitLineSelectionDto { path: "/new-line".to_string(), line: 0 }];
    assert!(collect_file_selections(&new_line_request).is_err());

    let mut old_line_request = empty_commit_request();
    old_line_request.selected_old_lines = vec![vw_api_types::git::GitLineSelectionDto {
        path: "nested/../old-line".to_string(),
        line: 0,
    }];
    assert!(collect_file_selections(&old_line_request).is_err());
}

#[test]
fn validate_merge_branch_pair_rejects_empty_head_and_same_branch() {
    assert!(validate_merge_branch_pair("feature", "main").is_ok());
    assert!(validate_merge_branch_pair("", "main").is_err());
    assert!(validate_merge_branch_pair("HEAD", "main").is_err());
    assert!(validate_merge_branch_pair("feature", "").is_err());
    assert!(validate_merge_branch_pair("feature", "HEAD").is_err());
    assert!(validate_merge_branch_pair("main", "main").is_err());
}

#[test]
fn final_line_ending_only_change_matches_missing_newline() {
    assert!(is_final_line_ending_only_change("123\n", "123"));
    assert!(is_final_line_ending_only_change("123", "123\n"));
    assert!(is_final_line_ending_only_change("123\r\n", "123"));
}

#[test]
fn final_line_ending_only_change_rejects_content_changes() {
    assert!(!is_final_line_ending_only_change("123\n", "124"));
    assert!(!is_final_line_ending_only_change("123\n", "123\n"));
    assert!(!is_final_line_ending_only_change("123\n456\n", "123\n457"));
}

#[test]
fn selected_file_content_replaces_selected_hunk() {
    let mut selection = GitFileSelection::default();
    selection.hunks.insert(0);

    let content =
        build_selected_file_content("alpha\nold\nomega\n", "alpha\nnew\nomega\n", &selection)
            .expect("hunk should build selected content");

    assert_eq!(content, "alpha\nnew\nomega\n");
}

#[test]
fn selected_file_content_keeps_unselected_delete_side() {
    let mut selection = GitFileSelection::default();
    selection.new_lines.insert(1);

    let content =
        build_selected_file_content("alpha\nold\nomega\n", "alpha\nnew\nomega\n", &selection)
            .expect("line selection should build selected content");

    assert_eq!(content, "alpha\nnew\nold\nomega\n");
}

#[test]
fn selected_file_content_keeps_unselected_insert_side() {
    let mut selection = GitFileSelection::default();
    selection.old_lines.insert(1);

    let content =
        build_selected_file_content("alpha\nold\nomega\n", "alpha\nnew\nomega\n", &selection)
            .expect("line selection should build selected content");

    assert_eq!(content, "alpha\nomega\n");
}

#[test]
fn selected_file_content_can_select_insert_only() {
    let mut selection = GitFileSelection::default();
    selection.new_lines.insert(1);

    let content = build_selected_file_content("alpha\nomega\n", "alpha\nnew\nomega\n", &selection)
        .expect("line selection should build selected content");

    assert_eq!(content, "alpha\nnew\nomega\n");
}

#[test]
fn selected_file_content_can_select_delete_only() {
    let mut selection = GitFileSelection::default();
    selection.old_lines.insert(1);

    let content = build_selected_file_content("alpha\nold\nomega\n", "alpha\nomega\n", &selection)
        .expect("line selection should build selected content");

    assert_eq!(content, "alpha\nomega\n");
}

#[test]
fn selected_file_content_without_delete_selection_preserves_deleted_line() {
    let content = build_selected_file_content(
        "alpha\nold\nomega\n",
        "alpha\nomega\n",
        &GitFileSelection::default(),
    )
    .expect("unselected delete content");

    assert_eq!(content, "alpha\nold\nomega\n");
}

#[test]
fn selected_file_content_rejects_bad_hunk_index() {
    let mut selection = GitFileSelection::default();
    selection.hunks.insert(99);

    let error = build_selected_file_content("old\n", "new\n", &selection)
        .expect_err("bad hunk should fail");

    assert_eq!(error, "bad hunk index");
}

#[test]
fn selected_file_content_without_selection_preserves_old_content() {
    let content = build_selected_file_content(
        "alpha\nold\nomega\n",
        "alpha\nnew\nomega\n",
        &GitFileSelection::default(),
    )
    .expect("unselected content");

    assert_eq!(content, "alpha\nold\nomega\n");
}

#[test]
fn selected_file_content_hunk_selects_insert_and_delete_operations() {
    let mut selection = GitFileSelection::default();
    selection.hunks.insert(0);

    let inserted = build_selected_file_content("alpha\nomega\n", "alpha\nnew\nomega\n", &selection)
        .expect("insert hunk");
    assert_eq!(inserted, "alpha\nnew\nomega\n");

    let deleted = build_selected_file_content("alpha\nold\nomega\n", "alpha\nomega\n", &selection)
        .expect("delete hunk");
    assert_eq!(deleted, "alpha\nomega\n");
}

#[test]
fn selected_file_content_handles_empty_sides_and_ignores_out_of_range_lines() {
    let mut insert = GitFileSelection::default();
    insert.new_lines.insert(0);
    assert_eq!(
        build_selected_file_content("", "created\n", &insert).expect("new file selection"),
        "created\n"
    );

    let mut delete = GitFileSelection::default();
    delete.old_lines.insert(0);
    assert_eq!(
        build_selected_file_content("deleted\n", "", &delete).expect("deleted file selection"),
        ""
    );

    let mut out_of_range = GitFileSelection::default();
    out_of_range.new_lines.insert(99);
    assert_eq!(
        build_selected_file_content("old\n", "new\n", &out_of_range)
            .expect("out of range selection"),
        "old\n"
    );
}

#[test]
fn split_lines_preserves_endings_and_empty_input() {
    assert_eq!(split_lines_preserve_ending(""), Vec::<&str>::new());
    assert_eq!(split_lines_preserve_ending("a\nb"), vec!["a\n", "b"]);
}

#[test]
fn push_line_range_appends_available_lines_only() {
    let lines = ["a\n", "b\n", "c"];
    let mut output = String::new();
    push_line_range(&mut output, &lines, 1, 4);
    assert_eq!(output, "b\nc");
}

#[test]
fn strip_final_line_ending_handles_lf_and_crlf() {
    assert_eq!(strip_final_line_ending("a\n"), Some("a"));
    assert_eq!(strip_final_line_ending("a\r\n"), Some("a"));
    assert_eq!(strip_final_line_ending("a"), None);
}

#[test]
fn git_file_mode_defaults_for_untracked_files() {
    let repo = init_repo();
    std::fs::write(repo.path().join("new.txt"), "new\n").expect("new file");

    assert_eq!(git_file_mode(repo.path_str(), "new.txt").expect("mode"), "100644");
}

#[test]
fn git_file_mode_reads_tracked_index_mode() {
    let repo = init_repo();
    std::fs::write(repo.path().join("tracked.txt"), "tracked\n").expect("tracked file");
    run_git(repo.path_str(), &["add", "tracked.txt"]).expect("add tracked");

    assert_eq!(git_file_mode(repo.path_str(), "tracked.txt").expect("tracked mode"), "100644");
}

#[test]
fn git_hash_object_stdin_writes_retrievable_blob() {
    let repo = init_repo();
    let oid = git_hash_object_stdin(repo.path_str(), "alpha\nbeta\n").expect("hash object");

    assert_eq!(oid.len(), 40);
    assert_eq!(
        run_git(repo.path_str(), &["cat-file", "-p", &oid]).expect("read object"),
        "alpha\nbeta\n"
    );
}

#[test]
fn git_hash_object_stdin_surfaces_stderr_for_non_repository() {
    let directory = tempfile::tempdir().expect("temp dir");
    let error = git_hash_object_stdin(directory.path().to_str().expect("utf8 path"), "content")
        .expect_err("hash-object should fail outside a git repository");

    assert!(!error.trim().is_empty());
    assert_ne!(error, "git hash-object failed");
}

#[test]
fn get_file_content_pair_reads_head_and_working_tree() {
    let repo = init_repo();
    std::fs::write(repo.path().join("tracked.txt"), "old\n").expect("old file");
    run_git(repo.path_str(), &["add", "tracked.txt"]).expect("add tracked");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");
    std::fs::write(repo.path().join("tracked.txt"), "new\n").expect("new file");

    assert_eq!(
        get_file_content_pair(repo.path_str(), "tracked.txt"),
        Some(("old\n".to_string(), "new\n".to_string()))
    );
}

#[test]
fn get_file_content_pair_rejects_non_repository() {
    let directory = tempfile::tempdir().expect("temp dir");
    assert_eq!(
        get_file_content_pair(directory.path().to_str().expect("utf8 path"), "missing.txt"),
        None
    );
}

#[test]
fn get_file_content_pair_returns_empty_contents_for_missing_file() {
    let repo = init_repo();

    let (head, working_tree) = get_file_content_pair(repo.path_str(), "missing.txt")
        .expect("repository should still yield a content pair");

    assert_eq!(head, "");
    assert_eq!(working_tree, "");
}

#[test]
fn write_index_content_stages_blob_without_changing_worktree() {
    let repo = init_repo();
    std::fs::write(repo.path().join("tracked.txt"), "old\n").expect("old file");
    run_git(repo.path_str(), &["add", "tracked.txt"]).expect("add tracked");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");

    write_index_content(repo.path_str(), "tracked.txt", "selected\n").expect("write index");

    assert_eq!(
        run_git(repo.path_str(), &["show", ":tracked.txt"]).expect("index blob"),
        "selected\n"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("tracked.txt")).expect("worktree"),
        "old\n"
    );
}

#[test]
fn write_index_content_empty_removes_only_index_entry() {
    let repo = init_repo();
    std::fs::write(repo.path().join("tracked.txt"), "tracked\n").expect("tracked file");
    run_git(repo.path_str(), &["add", "tracked.txt"]).expect("add tracked");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");

    write_index_content(repo.path_str(), "tracked.txt", "").expect("remove index entry");

    assert!(run_git(repo.path_str(), &["ls-files", "--error-unmatch", "tracked.txt"]).is_err());
    assert!(repo.path().join("tracked.txt").is_file());
}

#[test]
fn stage_selected_changes_resets_index_and_stages_only_selected_file() {
    let repo = init_repo();
    std::fs::write(repo.path().join("selected.txt"), "base\n").expect("selected base");
    std::fs::write(repo.path().join("other.txt"), "base\n").expect("other base");
    run_git(repo.path_str(), &["add", "selected.txt", "other.txt"]).expect("add base");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");
    std::fs::write(repo.path().join("selected.txt"), "selected\n").expect("selected change");
    std::fs::write(repo.path().join("other.txt"), "other\n").expect("other change");
    run_git(repo.path_str(), &["add", "other.txt"]).expect("prestage other");
    let mut request = empty_commit_request();
    request.selected_files = vec!["selected.txt".to_string()];

    stage_selected_changes(repo.path_str(), &request).expect("stage selection");

    assert_eq!(
        run_git(repo.path_str(), &["diff", "--cached", "--name-only"]).expect("cached names"),
        "selected.txt\n"
    );
}

#[test]
fn stage_selected_changes_rejects_invalid_path_before_resetting_index() {
    let repo = init_repo();
    std::fs::write(repo.path().join("tracked.txt"), "base\n").expect("tracked base");
    run_git(repo.path_str(), &["add", "tracked.txt"]).expect("add base");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");
    std::fs::write(repo.path().join("tracked.txt"), "changed\n").expect("tracked change");
    run_git(repo.path_str(), &["add", "tracked.txt"]).expect("stage change");
    let mut request = empty_commit_request();
    request.selected_files = vec!["../escape".to_string()];

    assert!(stage_selected_changes(repo.path_str(), &request).is_err());
    assert!(git_has_staged_changes(repo.path_str()).expect("staged state"));
}

#[test]
fn stage_selected_changes_stages_partial_line_selection() {
    let repo = init_repo();
    std::fs::write(repo.path().join("tracked.txt"), "alpha\nold\nomega\n").expect("tracked base");
    run_git(repo.path_str(), &["add", "tracked.txt"]).expect("add tracked");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");
    std::fs::write(repo.path().join("tracked.txt"), "alpha\nnew\nomega\n").expect("tracked change");
    let mut request = empty_commit_request();
    request.selected_lines =
        vec![vw_api_types::git::GitLineSelectionDto { path: "tracked.txt".to_string(), line: 1 }];

    stage_selected_changes(repo.path_str(), &request).expect("stage partial selection");

    let cached =
        run_git(repo.path_str(), &["diff", "--cached", "--", "tracked.txt"]).expect("cached diff");
    assert!(cached.contains("+new"));
    assert!(!cached.contains("-old"));
}

#[test]
fn commit_selected_blocking_stage_all_creates_commit() {
    let repo = init_repo();
    std::fs::write(repo.path().join("created.txt"), "created\n").expect("created file");
    let request = GitCommitRequest {
        project_id: vw_api_types::id::ProjectId("project".to_string()),
        worktree_id: None,
        message: "add created".to_string(),
        stage_all: true,
        selected_files: Vec::new(),
        selected_hunks: Vec::new(),
        selected_lines: Vec::new(),
        selected_old_lines: Vec::new(),
    };

    let response = commit_selected_blocking(repo.path_str(), &request).expect("commit");

    assert!(response.ok);
    assert!(response.created);
    assert_eq!(response.commit.message, "add created");
    assert_eq!(response.commit.sha.len(), 40);
}

#[test]
fn commit_selected_blocking_stage_all_succeeds_without_changes() {
    let repo = init_repo();
    std::fs::write(repo.path().join("created.txt"), "created\n").expect("created file");
    run_git(repo.path_str(), &["add", "created.txt"]).expect("add");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");
    let head = git_head_sha(repo.path_str()).expect("head");
    let request = GitCommitRequest {
        project_id: vw_api_types::id::ProjectId("project".to_string()),
        worktree_id: None,
        message: "no changes".to_string(),
        stage_all: true,
        selected_files: Vec::new(),
        selected_hunks: Vec::new(),
        selected_lines: Vec::new(),
        selected_old_lines: Vec::new(),
    };

    let response = commit_selected_blocking(repo.path_str(), &request).expect("no changes");

    assert!(response.ok);
    assert!(!response.created);
    assert_eq!(response.commit.sha, head);
    assert_eq!(response.commit.message, "no changes");
}

#[test]
fn commit_selected_blocking_selected_noop_succeeds_without_changes() {
    let repo = init_repo();
    std::fs::write(repo.path().join("tracked.txt"), "tracked\n").expect("tracked file");
    run_git(repo.path_str(), &["add", "tracked.txt"]).expect("add");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");
    let head = git_head_sha(repo.path_str()).expect("head");
    let request = GitCommitRequest {
        project_id: vw_api_types::id::ProjectId("project".to_string()),
        worktree_id: None,
        message: "noop selection".to_string(),
        stage_all: false,
        selected_files: vec!["tracked.txt".to_string()],
        selected_hunks: Vec::new(),
        selected_lines: Vec::new(),
        selected_old_lines: Vec::new(),
    };

    let response = commit_selected_blocking(repo.path_str(), &request).expect("no changes");

    assert!(response.ok);
    assert!(!response.created);
    assert_eq!(response.commit.sha, head);
    assert_eq!(response.commit.message, "noop selection");
}

#[test]
fn commit_selected_blocking_rejects_empty_selection() {
    let repo = init_repo();
    let request = GitCommitRequest {
        project_id: vw_api_types::id::ProjectId("project".to_string()),
        worktree_id: None,
        message: "nothing".to_string(),
        stage_all: false,
        selected_files: Vec::new(),
        selected_hunks: Vec::new(),
        selected_lines: Vec::new(),
        selected_old_lines: Vec::new(),
    };

    assert_eq!(
        commit_selected_blocking(repo.path_str(), &request).expect_err("empty selection"),
        "no changes selected"
    );
}

#[test]
fn commit_selected_blocking_selected_file_creates_commit() {
    let repo = init_repo();
    std::fs::write(repo.path().join("selected.txt"), "selected\n").expect("selected file");
    let mut request = empty_commit_request();
    request.message = "commit selected file".to_string();
    request.selected_files = vec!["selected.txt".to_string()];

    let response = commit_selected_blocking(repo.path_str(), &request).expect("selected commit");

    assert!(response.created);
    assert_eq!(response.commit.message, "commit selected file");
    assert_eq!(
        run_git(repo.path_str(), &["show", "--format=", "--name-only", "HEAD"])
            .expect("committed paths")
            .trim(),
        "selected.txt"
    );
}

#[test]
fn git_commit_with_message_file_preserves_multiline_message() {
    let repo = init_repo();
    std::fs::write(repo.path().join("created.txt"), "created\n").expect("created file");
    run_git(repo.path_str(), &["add", "created.txt"]).expect("add created");

    git_commit_with_message_file(repo.path_str(), "subject\n\nbody line")
        .expect("commit message file");

    assert_eq!(
        run_git(repo.path_str(), &["log", "--format=%B", "-n", "1"])
            .expect("commit message")
            .trim(),
        "subject\n\nbody line"
    );
}

#[test]
fn git_state_helpers_track_clean_staged_and_head_states() {
    let repo = init_repo();
    std::fs::write(repo.path().join("tracked.txt"), "base\n").expect("tracked base");
    run_git(repo.path_str(), &["add", "tracked.txt"]).expect("add base");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");
    let head = git_head_sha(repo.path_str()).expect("head sha");

    assert_eq!(head.len(), 40);
    assert!(git_is_clean(repo.path_str()).expect("clean repository"));
    assert!(!git_has_staged_changes(repo.path_str()).expect("empty index diff"));

    std::fs::write(repo.path().join("tracked.txt"), "changed\n").expect("tracked change");
    assert!(!git_is_clean(repo.path_str()).expect("dirty repository"));
    run_git(repo.path_str(), &["add", "tracked.txt"]).expect("stage change");
    assert!(git_has_staged_changes(repo.path_str()).expect("staged index diff"));
}

#[test]
fn verify_local_branch_and_worktree_lookup_distinguish_known_branch() {
    let repo = init_repo();
    run_git(repo.path_str(), &["checkout", "-b", "main"]).expect("main branch");
    std::fs::write(repo.path().join("base.txt"), "base\n").expect("base file");
    run_git(repo.path_str(), &["add", "base.txt"]).expect("add base");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");
    run_git(repo.path_str(), &["branch", "feature"]).expect("feature branch");

    assert!(verify_local_branch(repo.path_str(), "feature").is_ok());
    assert!(verify_local_branch(repo.path_str(), "missing").is_err());
    let main_path = worktree_path_for_branch(repo.path_str(), "main")
        .expect("worktree list")
        .expect("main worktree");
    assert_eq!(
        std::fs::canonicalize(&main_path).expect("canonical worktree path"),
        std::fs::canonicalize(repo.path()).expect("canonical repo path")
    );
    assert_eq!(
        worktree_path_for_branch(repo.path_str(), "missing").expect("missing lookup"),
        None
    );
}

#[test]
fn commit_target_branch_changes_if_needed_skips_clean_repository() {
    let repo = init_repo();
    run_git(repo.path_str(), &["checkout", "-b", "main"]).expect("main branch");
    std::fs::write(repo.path().join("base.txt"), "base\n").expect("base file");
    run_git(repo.path_str(), &["add", "base.txt"]).expect("add base");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");

    assert!(
        !commit_target_branch_changes_if_needed(repo.path_str(), "feature", "main")
            .expect("clean target")
    );
}

#[test]
fn merge_branch_blocking_reports_clean_already_merged_branch() {
    let repo = init_repo();
    run_git(repo.path_str(), &["checkout", "-b", "main"]).expect("main branch");
    std::fs::write(repo.path().join("base.txt"), "base\n").expect("base file");
    run_git(repo.path_str(), &["add", "base.txt"]).expect("add base");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");
    run_git(repo.path_str(), &["branch", "feature"]).expect("feature branch");

    let response =
        merge_branch_blocking(repo.path_str(), "feature", "main").expect("already merged response");

    assert!(response.ok);
    assert!(response.already_merged);
    assert_eq!(response.source_branch, "feature");
    assert_eq!(response.target_branch, "main");
    assert_eq!(response.message, "feature already merged into main");
}

#[test]
fn merge_branch_blocking_rejects_unknown_source_before_mutation() {
    let repo = init_repo();
    run_git(repo.path_str(), &["checkout", "-b", "main"]).expect("main branch");
    std::fs::write(repo.path().join("base.txt"), "base\n").expect("base file");
    run_git(repo.path_str(), &["add", "base.txt"]).expect("add base");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");
    let head = git_head_sha(repo.path_str()).expect("head before");

    assert!(merge_branch_blocking(repo.path_str(), "missing", "main").is_err());
    assert_eq!(git_head_sha(repo.path_str()).expect("head after"), head);
    assert!(git_is_clean(repo.path_str()).expect("clean after rejection"));
}

#[test]
fn merge_branch_blocking_aborts_conflict_state_before_returning_error() {
    let repo = init_repo();
    run_git(repo.path_str(), &["checkout", "-b", "main"]).expect("main branch");
    std::fs::write(repo.path().join("conflict.txt"), "base\n").expect("base file");
    run_git(repo.path_str(), &["add", "conflict.txt"]).expect("add base");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");

    run_git(repo.path_str(), &["checkout", "-b", "feature"]).expect("feature branch");
    std::fs::write(repo.path().join("conflict.txt"), "feature\n").expect("feature change");
    run_git(repo.path_str(), &["add", "conflict.txt"]).expect("add feature");
    run_git(repo.path_str(), &["commit", "-m", "feature change"]).expect("feature commit");

    run_git(repo.path_str(), &["checkout", "main"]).expect("checkout main");
    std::fs::write(repo.path().join("conflict.txt"), "main\n").expect("main change");
    run_git(repo.path_str(), &["add", "conflict.txt"]).expect("add main");
    run_git(repo.path_str(), &["commit", "-m", "main change"]).expect("main commit");

    assert!(merge_branch_blocking(repo.path_str(), "feature", "main").is_err());
    assert!(!repo.path().join(".git").join("MERGE_HEAD").exists());
    assert!(git_is_clean(repo.path_str()).expect("clean after conflict abort"));
    assert_eq!(
        run_git(repo.path_str(), &["rev-parse", "--abbrev-ref", "HEAD"])
            .expect("current branch")
            .trim(),
        "main"
    );
}

#[test]
fn merge_branch_blocking_commits_dirty_target_before_merge() {
    let repo = init_repo();
    run_git(repo.path_str(), &["checkout", "-b", "main"]).expect("main branch");
    std::fs::write(repo.path().join("base.txt"), "base\n").expect("base file");
    run_git(repo.path_str(), &["add", "base.txt"]).expect("add base");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");

    run_git(repo.path_str(), &["checkout", "-b", "feature"]).expect("feature branch");
    std::fs::write(repo.path().join("feature.txt"), "feature\n").expect("feature file");
    run_git(repo.path_str(), &["add", "feature.txt"]).expect("add feature");
    run_git(repo.path_str(), &["commit", "-m", "feature commit"]).expect("feature commit");

    run_git(repo.path_str(), &["checkout", "main"]).expect("checkout main");
    install_failing_pre_commit_hook(&repo);
    std::fs::write(repo.path().join("target.txt"), "target\n").expect("target file");

    let response = merge_branch_blocking(repo.path_str(), "feature", "main").expect("merge branch");

    assert!(response.ok);
    assert!(!response.already_merged);
    assert!(response.message.contains("saved target branch changes"));
    assert!(git_is_clean(repo.path_str()).expect("clean status"));
    let log = run_git(repo.path_str(), &["log", "--format=%s", "-n", "3"]).expect("log");
    assert!(log.contains("chore(task): save main changes before merging feature"));
    assert!(log.contains("chore(task): merge feature into main"));
}

#[test]
fn merge_branch_blocking_commits_dirty_target_before_already_merged_response() {
    let repo = init_repo();
    run_git(repo.path_str(), &["checkout", "-b", "main"]).expect("main branch");
    std::fs::write(repo.path().join("base.txt"), "base\n").expect("base file");
    run_git(repo.path_str(), &["add", "base.txt"]).expect("add base");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");

    run_git(repo.path_str(), &["checkout", "-b", "feature"]).expect("feature branch");
    std::fs::write(repo.path().join("feature.txt"), "feature\n").expect("feature file");
    run_git(repo.path_str(), &["add", "feature.txt"]).expect("add feature");
    run_git(repo.path_str(), &["commit", "-m", "feature commit"]).expect("feature commit");

    run_git(repo.path_str(), &["checkout", "main"]).expect("checkout main");
    run_git(repo.path_str(), &["merge", "--ff-only", "feature"]).expect("ff feature");
    let head_before = run_git(repo.path_str(), &["rev-parse", "HEAD"]).expect("head before");
    std::fs::write(repo.path().join("target.txt"), "target\n").expect("target file");

    let response = merge_branch_blocking(repo.path_str(), "feature", "main").expect("merge branch");

    assert!(response.ok);
    assert!(response.already_merged);
    assert_eq!(response.message, "saved target branch changes; feature already merged into main");
    let head_after = run_git(repo.path_str(), &["rev-parse", "HEAD"]).expect("head after");
    assert_ne!(head_after, head_before);
    let status = run_git(repo.path_str(), &["status", "--short"]).expect("status");
    assert_eq!(status, "");
    let log = run_git(repo.path_str(), &["log", "--format=%s", "-n", "1"]).expect("log");
    assert!(log.contains("chore(task): save main changes before merging feature"));
}

#[test]
fn git_stage_partial_file_stages_selected_new_line() {
    let repo = init_repo();
    std::fs::write(repo.path().join("tracked.txt"), "alpha\nold\nomega\n").expect("tracked");
    run_git(repo.path_str(), &["add", "tracked.txt"]).expect("add");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");
    std::fs::write(repo.path().join("tracked.txt"), "alpha\nnew\nomega\n").expect("modified");
    let mut selection = GitFileSelection::default();
    selection.new_lines.insert(1);

    git_stage_partial_file(repo.path_str(), "tracked.txt", &selection).expect("partial stage");

    let cached =
        run_git(repo.path_str(), &["diff", "--cached", "--", "tracked.txt"]).expect("cached diff");
    assert!(cached.contains("+new"));
    assert!(!cached.contains("-old"));
}

#[test]
fn git_stage_partial_file_stages_final_line_ending_change_as_whole_file() {
    let repo = init_repo();
    std::fs::write(repo.path().join("tracked.txt"), "alpha\n").expect("tracked file");
    run_git(repo.path_str(), &["add", "tracked.txt"]).expect("add tracked");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");
    std::fs::write(repo.path().join("tracked.txt"), "alpha").expect("remove newline");

    git_stage_partial_file(repo.path_str(), "tracked.txt", &GitFileSelection::default())
        .expect("stage line ending change");

    assert_eq!(
        run_git(repo.path_str(), &["show", ":tracked.txt"]).expect("index content"),
        "alpha"
    );
}

#[test]
fn git_stage_partial_file_rejects_selection_with_no_effect() {
    let repo = init_repo();
    std::fs::write(repo.path().join("tracked.txt"), "old\n").expect("tracked file");
    run_git(repo.path_str(), &["add", "tracked.txt"]).expect("add tracked");
    run_git(repo.path_str(), &["commit", "-m", "initial"]).expect("initial commit");
    std::fs::write(repo.path().join("tracked.txt"), "new\n").expect("modified file");

    let error =
        git_stage_partial_file(repo.path_str(), "tracked.txt", &GitFileSelection::default())
            .expect_err("empty selection");

    assert_eq!(error, "selected changes do not change file: tracked.txt");
}

#[test]
fn git_stage_partial_file_rejects_non_repository() {
    let directory = tempfile::tempdir().expect("temp dir");
    let error = git_stage_partial_file(
        directory.path().to_str().expect("utf8 path"),
        "missing.txt",
        &GitFileSelection::default(),
    )
    .expect_err("non-repository selection");

    assert_eq!(error, "Failed to read file content");
}

struct TestRepo {
    temp: tempfile::TempDir,
}

impl TestRepo {
    fn path(&self) -> &std::path::Path {
        self.temp.path()
    }

    fn path_str(&self) -> &str {
        self.temp.path().to_str().expect("utf8 path")
    }
}

fn init_repo() -> TestRepo {
    let temp = tempfile::tempdir().expect("repo dir");
    let repo = TestRepo { temp };
    run_git(repo.path_str(), &["init"]).expect("git init");
    run_git(repo.path_str(), &["config", "user.email", "test@example.com"]).expect("email");
    run_git(repo.path_str(), &["config", "user.name", "Test User"]).expect("name");
    repo
}

fn init_repo_git2() -> TestRepo {
    let temp = tempfile::tempdir().expect("repo dir");
    let mut opts = git2::RepositoryInitOptions::new();
    opts.initial_head("main");
    let repository = git2::Repository::init_opts(temp.path(), &opts).expect("git2 init");
    repository.config().expect("config").set_str("user.name", "Test User").expect("name");
    repository.config().expect("config").set_str("user.email", "test@example.com").expect("email");
    TestRepo { temp }
}

fn write_and_commit_all_git2(path: &std::path::Path, files: &[(&str, &str)], message: &str) {
    for (relative, content) in files {
        let full = path.join(relative);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("parent dir");
        }
        std::fs::write(&full, content).expect("write file");
    }
    let repo = git2::Repository::open(path).expect("open repo");
    let tree_id = {
        let mut index = repo.index().expect("index");
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).expect("add all");
        index.write().expect("index write");
        index.write_tree().expect("tree")
    };
    let tree = repo.find_tree(tree_id).expect("tree");
    let sig = repo.signature().expect("signature");
    let parents = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .into_iter()
        .collect::<Vec<_>>();
    let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs).expect("commit");
}

fn blob_at_commit(path: &std::path::Path, sha: &str, file: &str) -> String {
    let repo = git2::Repository::open(path).expect("open repo");
    let commit = repo
        .revparse_single(sha)
        .expect("revparse")
        .peel_to_commit()
        .expect("commit");
    let tree = commit.tree().expect("tree");
    let entry = tree.get_path(std::path::Path::new(file)).expect("tree path");
    let object = entry.to_object(&repo).expect("object");
    let blob = object.as_blob().expect("blob");
    String::from_utf8_lossy(blob.content()).to_string()
}

fn create_branch_git2(path: &std::path::Path, name: &str) {
    let repo = git2::Repository::open(path).expect("open repo");
    let commit = repo.head().expect("head").peel_to_commit().expect("commit");
    repo.branch(name, &commit, false).expect("create branch");
}

fn checkout_branch_git2(path: &std::path::Path, name: &str) {
    let repo = git2::Repository::open(path).expect("open repo");
    let object = repo.revparse_single(&format!("refs/heads/{name}")).expect("branch object");
    repo.checkout_tree(&object, None).expect("checkout tree");
    repo.set_head(&format!("refs/heads/{name}")).expect("set head");
}

fn head_oid_git2(path: &std::path::Path) -> String {
    let repo = git2::Repository::open(path).expect("open repo");
    repo.head().expect("head").peel_to_commit().expect("commit").id().to_string()
}

fn current_branch_git2(path: &std::path::Path) -> String {
    let repo = git2::Repository::open(path).expect("open repo");
    repo.head()
        .expect("head")
        .shorthand()
        .expect("branch shorthand")
        .to_string()
}

fn commit_parent_count_git2(path: &std::path::Path, sha: &str) -> usize {
    let repo = git2::Repository::open(path).expect("open repo");
    repo.revparse_single(sha)
        .expect("revparse")
        .peel_to_commit()
        .expect("commit")
        .parent_count()
}

fn commit_parent_oids_git2(path: &std::path::Path, sha: &str) -> Vec<String> {
    let repo = git2::Repository::open(path).expect("open repo");
    let commit = repo.revparse_single(sha).expect("revparse").peel_to_commit().expect("commit");
    (0..commit.parent_count())
        .map(|index| commit.parent_id(index).expect("parent id").to_string())
        .collect()
}

fn commit_message_git2(path: &std::path::Path, sha: &str) -> String {
    let repo = git2::Repository::open(path).expect("open repo");
    repo.revparse_single(sha)
        .expect("revparse")
        .peel_to_commit()
        .expect("commit")
        .message()
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn install_failing_pre_commit_hook(repo: &TestRepo) {
    let hook_path = repo.path().join(".git").join("hooks").join("pre-commit");
    std::fs::write(&hook_path, "#!/bin/sh\nexit 1\n").expect("pre-commit hook");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = std::fs::metadata(&hook_path).expect("hook metadata").permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&hook_path, permissions).expect("hook permissions");
    }
}

struct StoredProjectCleanup(String);

impl Drop for StoredProjectCleanup {
    fn drop(&mut self) {
        crate::app::agent::project::remove_local_test_project(&self.0)
            .expect("project should clean up");
    }
}

fn store_git_test_project(repo: &TestRepo) -> (String, StoredProjectCleanup) {
    let project_id = format!("gateway-git-{}", Uuid::new_v4());
    let info = test_project_info(repo.path_str(), Vec::new(), Some(project::Vcs::Git));
    let info = project::Info { id: project_id.clone(), ..info };
    crate::app::agent::project::store_local_test_project(&info).expect("project should write");
    (project_id.clone(), StoredProjectCleanup(project_id))
}

fn empty_commit_request() -> GitCommitRequest {
    GitCommitRequest {
        project_id: vw_api_types::id::ProjectId("project".to_string()),
        worktree_id: None,
        message: "commit".to_string(),
        stage_all: false,
        selected_files: Vec::new(),
        selected_hunks: Vec::new(),
        selected_lines: Vec::new(),
        selected_old_lines: Vec::new(),
    }
}

fn test_project_info(
    worktree: &str,
    sandboxes: Vec<&str>,
    vcs: Option<project::Vcs>,
) -> project::Info {
    project::Info {
        id: "project".to_string(),
        worktree: worktree.to_string(),
        vcs,
        name: None,
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: sandboxes.into_iter().map(str::to_string).collect(),
    }
}
