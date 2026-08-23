#![cfg(not(target_arch = "wasm32"))]

//! 验证原生 worktree 的仓库发现、候选命名、基准分支解析与启动脚本顺序。

use super::native::{
    CmdResult, candidate, canonical, find_entry, parse_worktree_list, path_exists,
    resolve_reset_target, run_git, run_start_scripts, worktree_root,
};
use super::{Error, Info};
use crate::app::agent::project;
use crate::app::agent::project::instance;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

static WORKTREE_PROJECT_CONTEXT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[test]
fn parse_worktree_list_preserves_paths_and_branches() {
    let entries =
        parse_worktree_list("worktree /repo\nbranch refs/heads/main\n\nworktree /repo-two\nbare\n");

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].path, "/repo");
    assert_eq!(entries[0].branch.as_deref(), Some("refs/heads/main"));
    assert_eq!(entries[1].branch, None);
}

#[test]
fn parse_worktree_list_ignores_orphan_branches_and_unknown_records() {
    let entries = parse_worktree_list(
        "branch refs/heads/orphan\nunknown value\nworktree /repo\nHEAD abc123\nbranch refs/heads/first\nbranch refs/heads/final\n",
    );

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, "/repo");
    assert_eq!(entries[0].branch.as_deref(), Some("refs/heads/final"));
}

#[test]
fn cmd_result_prefers_stderr_then_stdout_for_error_text() {
    let result = CmdResult { success: false, stdout: "out\n".into(), stderr: "err\n".into() };
    assert_eq!(result.error_text("fallback"), "err\nout");

    let empty = CmdResult { success: false, stdout: String::new(), stderr: String::new() };
    assert_eq!(empty.error_text("fallback"), "fallback");
}

#[test]
fn cmd_result_ignores_whitespace_only_streams_for_error_text() {
    let stdout_only =
        CmdResult { success: false, stdout: " output \n".into(), stderr: " \n".into() };
    assert_eq!(stdout_only.error_text("fallback"), "output");

    let whitespace = CmdResult { success: false, stdout: "\t".into(), stderr: " \n".into() };
    assert_eq!(whitespace.error_text("fallback"), "fallback");
}

#[tokio::test]
async fn path_exists_reports_temp_directory() {
    let dir = tempfile::tempdir().expect("temp dir");

    assert!(path_exists(dir.path()).await);
    assert!(!path_exists(dir.path().join("missing").as_path()).await);
}

#[tokio::test]
async fn path_exists_reports_regular_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = dir.path().join("file.txt");
    std::fs::write(&file, "content").expect("write file");

    assert!(path_exists(&file).await);
}

#[tokio::test]
async fn canonical_absolutizes_missing_relative_paths() {
    let path = canonical("definitely-missing-worktree-test-path").await;

    assert!(path.is_absolute());
    assert!(path.ends_with("definitely-missing-worktree-test-path"));
}

#[tokio::test]
async fn canonical_resolves_existing_paths() {
    let dir = tempfile::tempdir().expect("temp dir");
    let nested = dir.path().join("nested");
    std::fs::create_dir(&nested).expect("create nested directory");
    let expected = std::fs::canonicalize(&nested).expect("canonical path");
    let expected = if cfg!(windows) {
        std::path::PathBuf::from(expected.to_string_lossy().to_ascii_lowercase())
    } else {
        expected
    };

    assert_eq!(canonical(&nested).await, expected);
}

#[tokio::test]
async fn find_entry_matches_canonical_target_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let entries = vec![super::native::WorktreeEntry {
        path: dir.path().to_string_lossy().to_string(),
        branch: Some("refs/heads/demo".to_string()),
    }];

    let found = find_entry(&entries, dir.path()).await.expect("entry");

    assert_eq!(found.branch.as_deref(), Some("refs/heads/demo"));
}

#[tokio::test]
async fn find_entry_returns_none_for_unlisted_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let entries = vec![super::native::WorktreeEntry {
        path: dir.path().to_string_lossy().to_string(),
        branch: None,
    }];

    assert!(find_entry(&entries, &dir.path().join("missing")).await.is_none());
}

#[tokio::test]
async fn run_git_and_resolve_reset_target_work_in_temp_repo() {
    let repo = tempfile::tempdir().expect("temp repo");
    init_repo(repo.path());

    let status =
        run_git(&["status", "--porcelain"], &repo.path().to_string_lossy()).await.expect("git");
    assert!(status.success);

    let target = resolve_reset_target(repo.path(), Some("main")).await.expect("main target");
    assert_eq!(target.target, "main");
    assert_eq!(target.remote, None);
    assert_eq!(target.remote_branch, None);

    let invalid = match resolve_reset_target(repo.path(), Some("missing-branch")).await {
        Ok(_) => panic!("missing branch should be rejected"),
        Err(error) => error,
    };
    assert!(
        invalid.to_string().contains("missing-branch")
            || invalid.to_string().contains("Needed a single revision")
    );
}

#[tokio::test]
async fn candidate_uses_requested_base_when_directory_and_branch_are_available() {
    let repo = tempfile::tempdir().expect("temp repo");
    init_repo(repo.path());
    let root = tempfile::tempdir().expect("worktree root");
    let primary = repo.path().to_path_buf();
    let root_path = root.path().to_path_buf();

    let info = in_project(&primary, move || {
        Box::pin(async move { candidate(&root_path, Some("feature")).await })
    })
    .await
    .expect("candidate");

    assert_eq!(info.name, "feature");
    assert_eq!(info.branch, "vibewindow/feature");
    assert_eq!(PathBuf::from(info.directory), root.path().join("feature"));
}

#[tokio::test]
async fn candidate_skips_existing_directory_for_requested_base() {
    let repo = tempfile::tempdir().expect("temp repo");
    init_repo(repo.path());
    let root = tempfile::tempdir().expect("worktree root");
    std::fs::create_dir(root.path().join("feature")).expect("create occupied candidate dir");
    let primary = repo.path().to_path_buf();
    let root_path = root.path().to_path_buf();

    let info = in_project(&primary, move || {
        Box::pin(async move { candidate(&root_path, Some("feature")).await })
    })
    .await
    .expect("candidate");

    assert_ne!(info.name, "feature");
    assert!(info.name.starts_with("feature-"));
    assert_eq!(info.branch, format!("vibewindow/{}", info.name));
    assert_eq!(PathBuf::from(info.directory), root.path().join(&info.name));
}

#[tokio::test]
async fn candidate_skips_existing_branch_for_requested_base() {
    let repo = tempfile::tempdir().expect("temp repo");
    init_repo(repo.path());
    run(Command::new("git").arg("-C").arg(repo.path()).args(["branch", "vibewindow/feature"]));
    let root = tempfile::tempdir().expect("worktree root");
    let primary = repo.path().to_path_buf();
    let root_path = root.path().to_path_buf();

    let info = in_project(&primary, move || {
        Box::pin(async move { candidate(&root_path, Some("feature")).await })
    })
    .await
    .expect("candidate");

    assert_ne!(info.name, "feature");
    assert!(info.name.starts_with("feature-"));
    assert_eq!(info.branch, format!("vibewindow/{}", info.name));
}

#[tokio::test]
async fn candidate_generates_random_name_without_base() {
    let repo = tempfile::tempdir().expect("temp repo");
    init_repo(repo.path());
    let root = tempfile::tempdir().expect("worktree root");
    let primary = repo.path().to_path_buf();
    let root_path = root.path().to_path_buf();

    let info =
        in_project(&primary, move || Box::pin(async move { candidate(&root_path, None).await }))
            .await
            .expect("candidate");

    assert!(!info.name.is_empty());
    assert_eq!(info.branch, format!("vibewindow/{}", info.name));
    assert_eq!(PathBuf::from(info.directory), root.path().join(&info.name));
}

#[tokio::test]
async fn run_start_scripts_runs_extra_when_project_lookup_is_unavailable() {
    let dir = tempfile::tempdir().expect("worktree dir");

    run_start_scripts(&dir.path().to_string_lossy(), " \n ", "echo extra > extra.txt")
        .await
        .expect("run extra script");

    assert!(dir.path().join("extra.txt").exists());
}

#[tokio::test]
async fn run_start_scripts_runs_project_start_before_extra_script() {
    let dir = tempfile::tempdir().expect("worktree dir");
    let mut project = project_info(&dir.path().to_string_lossy());
    project.id = unique_project_id("worktree-start-success");
    project.commands = Some(project::Commands { start: Some("echo start > started.txt".into()) });
    project::store_local_test_project(&project).expect("store project");

    run_start_scripts(
        &dir.path().to_string_lossy(),
        &project.id,
        "test -f started.txt && echo extra > extra.txt",
    )
    .await
    .expect("run start scripts");

    assert!(dir.path().join("started.txt").exists());
    assert!(dir.path().join("extra.txt").exists());
    project::remove_local_test_project(&project.id).expect("remove project");
}

#[tokio::test]
async fn run_start_scripts_stops_when_project_start_fails() {
    let dir = tempfile::tempdir().expect("worktree dir");
    let mut project = project_info(&dir.path().to_string_lossy());
    project.id = unique_project_id("worktree-start-failure");
    project.commands = Some(project::Commands { start: Some(failing_shell_command().to_string()) });
    project::store_local_test_project(&project).expect("store project");

    run_start_scripts(&dir.path().to_string_lossy(), &project.id, "echo extra > extra.txt")
        .await
        .expect("failed project start should be non-fatal");

    assert!(!dir.path().join("extra.txt").exists());
    project::remove_local_test_project(&project.id).expect("remove project");
}

#[tokio::test]
async fn resolve_reset_target_parses_valid_slash_ref_as_remote_branch() {
    let repo = tempfile::tempdir().expect("temp repo");
    init_repo(repo.path());
    run(Command::new("git").arg("-C").arg(repo.path()).arg("branch").arg("fork/topic"));

    let target = resolve_reset_target(repo.path(), Some("fork/topic")).await.expect("slash target");

    assert_eq!(target.target, "fork/topic");
    assert_eq!(target.remote.as_deref(), Some("fork"));
    assert_eq!(target.remote_branch.as_deref(), Some("topic"));
}

#[tokio::test]
async fn resolve_reset_target_uses_main_for_blank_base_ref() {
    let repo = tempfile::tempdir().expect("temp repo");
    init_repo(repo.path());

    let target = resolve_reset_target(repo.path(), Some(" \n ")).await.expect("default target");

    assert_eq!(target.target, "main");
    assert_eq!(target.remote, None);
    assert_eq!(target.remote_branch, None);
}

#[tokio::test]
async fn resolve_reset_target_falls_back_to_master_without_remotes() {
    let repo = tempfile::tempdir().expect("temp repo");
    init_repo_with_branch(repo.path(), "master");

    let target = resolve_reset_target(repo.path(), None).await.expect("master target");

    assert_eq!(target.target, "master");
    assert_eq!(target.remote, None);
    assert_eq!(target.remote_branch, None);
}

#[tokio::test]
async fn resolve_reset_target_prefers_origin_remote_head() {
    let repo = tempfile::tempdir().expect("temp repo");
    init_repo(repo.path());
    add_remote_head(repo.path(), "origin", "trunk");

    let target = resolve_reset_target(repo.path(), None).await.expect("origin target");

    assert_eq!(target.target, "origin/trunk");
    assert_eq!(target.remote.as_deref(), Some("origin"));
    assert_eq!(target.remote_branch.as_deref(), Some("trunk"));
}

#[tokio::test]
async fn resolve_reset_target_uses_single_non_origin_remote_head() {
    let repo = tempfile::tempdir().expect("temp repo");
    init_repo(repo.path());
    add_remote_head(repo.path(), "fork", "stable");

    let target = resolve_reset_target(repo.path(), None).await.expect("single remote target");

    assert_eq!(target.target, "fork/stable");
    assert_eq!(target.remote.as_deref(), Some("fork"));
    assert_eq!(target.remote_branch.as_deref(), Some("stable"));
}

#[tokio::test]
async fn resolve_reset_target_prefers_upstream_among_multiple_non_origin_remotes() {
    let repo = tempfile::tempdir().expect("temp repo");
    init_repo(repo.path());
    add_remote(repo.path(), "mirror");
    add_remote_head(repo.path(), "upstream", "develop");

    let target = resolve_reset_target(repo.path(), None).await.expect("upstream target");

    assert_eq!(target.target, "upstream/develop");
    assert_eq!(target.remote.as_deref(), Some("upstream"));
    assert_eq!(target.remote_branch.as_deref(), Some("develop"));
}

#[tokio::test]
async fn resolve_reset_target_uses_local_main_when_multiple_remotes_have_no_preferred_name() {
    let repo = tempfile::tempdir().expect("temp repo");
    init_repo(repo.path());
    add_remote(repo.path(), "fork");
    add_remote(repo.path(), "mirror");

    let target = resolve_reset_target(repo.path(), None).await.expect("local main target");

    assert_eq!(target.target, "main");
    assert_eq!(target.remote, None);
    assert_eq!(target.remote_branch, None);
}

#[tokio::test]
async fn resolve_reset_target_falls_back_to_local_main_when_remote_head_is_missing() {
    let repo = tempfile::tempdir().expect("temp repo");
    init_repo(repo.path());
    add_remote(repo.path(), "origin");

    let target = resolve_reset_target(repo.path(), None).await.expect("local main target");

    assert_eq!(target.target, "main");
    assert_eq!(target.remote, None);
    assert_eq!(target.remote_branch, None);
}

#[tokio::test]
async fn resolve_reset_target_rejects_repository_without_default_branch() {
    let repo = tempfile::tempdir().expect("temp repo");
    init_repo_with_branch(repo.path(), "topic");

    let error = match resolve_reset_target(repo.path(), None).await {
        Ok(_) => panic!("repository without main, master, or remote HEAD should be rejected"),
        Err(error) => error,
    };

    assert!(matches!(error, Error::Invalid(message) if message == "Default branch not found"));
}

#[tokio::test]
async fn resolve_reset_target_reports_non_repository_error() {
    let directory = tempfile::tempdir().expect("temp directory");

    let error = match resolve_reset_target(directory.path(), None).await {
        Ok(_) => panic!("non-repository should be rejected"),
        Err(error) => error,
    };

    assert!(matches!(error, Error::Invalid(message) if !message.trim().is_empty()));
}

#[test]
fn info_is_cloneable_for_native_callers() {
    let info = Info { name: "n".into(), branch: "b".into(), directory: "d".into() };

    assert_eq!(info.clone().directory, "d");
}

#[test]
fn worktree_root_uses_shared_task_worktrees_directory() {
    let project = project_info("/repo/main");

    let root = worktree_root(&project).expect("worktree root");

    assert_eq!(
        root.file_name().and_then(|value| value.to_str()),
        Some(vw_config_types::paths::TASK_WORKTREES_DIR_NAME)
    );
}

#[test]
fn worktree_root_rejects_blank_project_worktree() {
    let error =
        worktree_root(&project_info(" \n ")).expect_err("blank worktree should be rejected");

    assert!(
        matches!(error, Error::MissingProject(message) if message == "missing project worktree root")
    );
}

fn init_repo(path: &Path) {
    init_repo_with_branch(path, "main");
}

fn init_repo_with_branch(path: &Path, branch: &str) {
    run(Command::new("git").arg("init").arg("-b").arg(branch).arg(path));
    run(Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("config")
        .arg("user.email")
        .arg("test@example.com"));
    run(Command::new("git").arg("-C").arg(path).arg("config").arg("user.name").arg("Test User"));
    std::fs::write(path.join("README.md"), "demo").expect("write readme");
    run(Command::new("git").arg("-C").arg(path).arg("add").arg("README.md"));
    run(Command::new("git").arg("-C").arg(path).arg("commit").arg("-m").arg("init"));
}

fn add_remote(path: &Path, name: &str) {
    run(Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("remote")
        .arg("add")
        .arg(name)
        .arg(format!("https://example.invalid/{name}.git")));
}

fn add_remote_head(path: &Path, remote: &str, branch: &str) {
    add_remote(path, remote);
    run(Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("update-ref")
        .arg(format!("refs/remotes/{remote}/{branch}"))
        .arg("HEAD"));
    run(Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("symbolic-ref")
        .arg(format!("refs/remotes/{remote}/HEAD"))
        .arg(format!("refs/remotes/{remote}/{branch}")));
}

fn project_info(worktree: &str) -> project::Info {
    project::Info {
        id: "project-1".to_string(),
        worktree: worktree.to_string(),
        vcs: Some(project::Vcs::Git),
        name: None,
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 0, updated: 0, initialized: None },
        sandboxes: Vec::new(),
    }
}

async fn in_project<T: Send + 'static>(
    primary: &Path,
    operation: impl FnOnce() -> project::BoxFuture<T> + Send + 'static,
) -> T {
    let _lock = WORKTREE_PROJECT_CONTEXT_LOCK.lock().await;
    instance::provide(primary, None, operation).await.expect("project context")
}

fn unique_project_id(prefix: &str) -> String {
    format!("{}-{}", prefix, uuid::Uuid::new_v4())
}

fn failing_shell_command() -> &'static str {
    if cfg!(windows) { "exit /b 7" } else { "exit 7" }
}

fn run(command: &mut Command) {
    let output = command.output().expect("run command");
    assert!(
        output.status.success(),
        "command failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
