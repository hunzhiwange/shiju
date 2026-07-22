fn legacy_worktree_path(repo_root: &str, leaf: &str) -> String {
    format!(
        "{repo_root}/{}/{}/{leaf}",
        vw_config_types::paths::PROD_HOME_CONFIG_DIR_NAME,
        vw_config_types::paths::TASK_WORKTREES_DIR_NAME
    )
}

fn unique_claim_path(label: &str) -> String {
    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("/tmp/vibewindow-{label}-{}-{id}", std::process::id())
}

static WORKTREE_ADMIN_ENV_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

fn worktree_admin_env_lock() -> std::sync::MutexGuard<'static, ()> {
    WORKTREE_ADMIN_ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct EnvGuard {
    key: &'static str,
    original: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set_os(key: &'static str, value: &std::ffi::OsStr) -> Self {
        let original = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}

fn run_git_test(cwd: &std::path::Path, args: &[&str]) -> std::process::Output {
    let output = vw_shared::shell::git_std_command()
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("git command should start");
    assert!(
        output.status.success(),
        "git {:?} failed: stdout={} stderr={}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn create_git_repo() -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempfile::TempDir::new().expect("temp repo should be created");
    let repo = temp.path().join("repo");
    std::fs::create_dir_all(&repo).expect("repo dir should be created");
    run_git_test(&repo, &["init", "--initial-branch=main"]);
    run_git_test(&repo, &["config", "user.name", "VibeWindow Test"]);
    run_git_test(&repo, &["config", "user.email", "test@vibewindow.local"]);
    std::fs::write(repo.join("README.md"), "seed\n").expect("seed file should be written");
    run_git_test(&repo, &["add", "."]);
    run_git_test(&repo, &["commit", "-m", "init"]);
    (temp, repo)
}

fn managed_slot_path(home: &std::path::Path, leaf: &str) -> std::path::PathBuf {
    vw_config_types::paths::task_worktrees_dir(home).join(leaf)
}

fn add_worktree(repo: &std::path::Path, path: &std::path::Path, branch: &str) {
    std::fs::create_dir_all(path.parent().expect("slot parent")).expect("slot parent should exist");
    run_git_test(repo, &["branch", branch]);
    run_git_test(repo, &["worktree", "add", path.to_string_lossy().as_ref(), branch]);
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("worktree_admin_tests"));
}

#[test]
fn valid_branch_name_rejects_empty_and_head() {
    assert!(!super::valid_branch_name(""));
    assert!(!super::valid_branch_name("  "));
    assert!(!super::valid_branch_name("HEAD"));
    assert!(!super::valid_branch_name(" HEAD "));
    assert!(super::valid_branch_name("main"));
    assert!(super::valid_branch_name(" feature/task "));
}

#[test]
fn resolve_merge_target_prefers_task_branch_when_valid() {
    let mut task = crate::app::task::models::Task::new(1);
    task.merge_target_branch = Some(" release ".into());

    assert_eq!(
        super::resolve_merge_target_branch(&task, "/tmp/not-a-repo").as_deref(),
        Some("release")
    );

    task.merge_target_branch = Some("HEAD".into());
    assert_eq!(super::resolve_merge_target_branch(&task, "/tmp/not-a-repo"), None);
}

#[test]
fn cleanup_candidate_keeps_gateway_path_without_local_directory() {
    let repo_root = "/gateway/repo";
    let slot_path = legacy_worktree_path(repo_root, "slot-1");
    let candidate = super::cleanup_worktree_candidate(
        repo_root,
        super::WorktreeEntry { path: slot_path.clone(), branch: Some("vw/task/one".into()) },
    );

    assert_eq!(candidate, Some((slot_path, Some("vw/task/one".into()))));
    assert_eq!(
        super::cleanup_worktree_candidate(
            repo_root,
            super::WorktreeEntry { path: repo_root.into(), branch: Some("main".into()) }
        ),
        None
    );
    assert_eq!(
        super::cleanup_worktree_candidate(
            repo_root,
            super::WorktreeEntry {
                path: "/gateway/repo/detached".into(),
                branch: Some("HEAD".into())
            }
        ),
        None
    );
    assert_eq!(
        super::cleanup_worktree_candidate(
            repo_root,
            super::WorktreeEntry { path: "/gateway/repo/detached".into(), branch: None }
        ),
        None
    );
}

#[test]
fn delete_candidates_keep_managed_gateway_paths_without_local_directory() {
    let repo_root = "/gateway/repo";
    let slot_path = legacy_worktree_path(repo_root, "slot-1");
    let candidates = super::delete_worktree_candidates(
        repo_root,
        vec![
            super::WorktreeEntry { path: slot_path.clone(), branch: Some("vw/task/one".into()) },
            super::WorktreeEntry {
                path: "/gateway/repo/other/slot-2".into(),
                branch: Some("other".into()),
            },
        ],
    );

    assert_eq!(candidates, vec![(slot_path, Some("vw/task/one".into()))]);
}

#[test]
fn delete_candidates_preserve_managed_order_and_detached_branch_state() {
    let repo_root = "/gateway/repo";
    let first = legacy_worktree_path(repo_root, "slot-2");
    let second = legacy_worktree_path(repo_root, "slot-1");

    let candidates = super::delete_worktree_candidates(
        repo_root,
        vec![
            super::WorktreeEntry { path: first.clone(), branch: None },
            super::WorktreeEntry { path: second.clone(), branch: Some("HEAD".into()) },
            super::WorktreeEntry {
                path: legacy_worktree_path(repo_root, "fork-1"),
                branch: Some("feature/ignored".into()),
            },
        ],
    );

    assert_eq!(candidates, vec![(first, None), (second, Some("HEAD".into()))]);
}

#[test]
fn force_delete_state_logs_claims_without_blocking() {
    let path = unique_claim_path("force-delete");
    let (sender, receiver) = std::sync::mpsc::channel();

    assert!(super::claim_worktree_path(&path));
    super::emit_force_delete_state_logs(&[(path.clone(), None)], Some(&sender));
    super::release_claimed_worktree(&path);

    let logs = receiver.try_iter().collect::<Vec<_>>();
    assert!(logs.iter().any(|entry| match entry {
        super::TaskLogStream::Stderr(message) => {
            message.contains("强制删除继续执行") && message.contains("正在使用中的 worktree")
        }
        _ => false,
    }));
}

#[test]
fn force_delete_state_logs_nothing_when_candidates_are_unclaimed() {
    let path = unique_claim_path("unclaimed-delete");
    let (sender, receiver) = std::sync::mpsc::channel();

    super::emit_force_delete_state_logs(&[(path, Some("feature".into()))], Some(&sender));

    assert!(receiver.try_iter().next().is_none());
}

#[test]
fn claim_worktree_path_is_exclusive_until_released() {
    let path = unique_claim_path("claim");

    assert!(super::claim_worktree_path(&path));
    assert!(!super::claim_worktree_path(&path));
    super::release_claimed_worktree(&path);
    assert!(super::claim_worktree_path(&path));
    super::release_claimed_worktree(&path);
}

#[test]
fn claim_worktree_path_normalizes_equivalent_existing_paths() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let direct = temp.path().to_string_lossy().to_string();
    let equivalent = temp.path().join(".").to_string_lossy().to_string();

    assert!(super::claim_worktree_path(&direct));
    assert!(!super::claim_worktree_path(&equivalent));
    super::release_claimed_worktree(&equivalent);
    assert!(super::claim_worktree_path(&direct));
    super::release_claimed_worktree(&direct);
}

#[test]
fn claim_guard_releases_claim_on_drop() {
    let path = unique_claim_path("guard");

    assert!(super::claim_worktree_path(&path));
    {
        let _guard = super::WorktreeClaimGuard::new(Some(path.clone()));
        assert!(!super::claim_worktree_path(&path));
    }
    assert!(super::claim_worktree_path(&path));
    super::release_claimed_worktree(&path);
}

#[test]
fn assign_task_execution_worktree_uses_existing_selected_path_without_git_pool() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let selected = temp.path().join("selected");
    std::fs::create_dir_all(&selected).expect("selected dir");

    let mut task = crate::app::task::models::Task::new(1);
    task.id = "T20260612.0001".into();
    task.selected_worktree_path = Some(selected.to_string_lossy().to_string());
    task.merge_target_branch = Some("main".into());

    let (sender, receiver) = std::sync::mpsc::channel();
    let assigned = super::assign_task_execution_worktree("/tmp/not-a-repo", &task, Some(&sender))
        .expect("assigned selected path");

    let normalized = super::normalize_path(&selected.to_string_lossy());
    assert_eq!(assigned.as_deref(), Some(normalized.as_str()));
    assert!(receiver.try_iter().any(|entry| match entry {
        super::TaskLogStream::Stdout(message) => {
            message.contains("使用已选择工作区") && message.contains(normalized.as_str())
        }
        _ => false,
    }));
}

#[test]
fn assign_task_execution_worktree_defers_valid_target_branch_to_gateway() {
    let mut task = crate::app::task::models::Task::new(1);
    task.id = "T20260612.0003".into();
    task.merge_target_branch = Some(" release ".into());
    let (sender, receiver) = std::sync::mpsc::channel();

    let assigned = super::assign_task_execution_worktree("/tmp/not-a-repo", &task, Some(&sender))
        .expect("gateway allocation should not fail");

    assert_eq!(assigned, None);
    assert!(receiver.try_iter().any(|entry| match entry {
        super::TaskLogStream::Stdout(message) => {
            message.contains("gateway 预分配") && message.contains(task.id.as_str())
        }
        _ => false,
    }));
}

#[test]
fn assign_task_execution_worktree_ignores_missing_selected_path() {
    let mut task = crate::app::task::models::Task::new(1);
    task.selected_worktree_path = Some(unique_claim_path("missing-selected"));
    task.merge_target_branch = Some("main".into());

    let assigned = super::assign_task_execution_worktree("/tmp/not-a-repo", &task, None)
        .expect("missing selected path should fall back to gateway allocation");

    assert_eq!(assigned, None);
}

#[test]
fn resolve_task_execution_workspace_uses_selected_path_and_claims_it() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let selected = temp.path().join("selected");
    std::fs::create_dir_all(&selected).expect("selected dir");
    let selected_string = selected.to_string_lossy().to_string();
    let normalized = super::normalize_path(&selected_string);

    let mut task = crate::app::task::models::Task::new(1);
    task.id = "T20260612.0002".into();
    task.selected_worktree_path = Some(selected_string);
    task.merge_source_branch = Some("feature".into());

    let (workspace, guard) =
        super::resolve_task_execution_workspace(&task, "/tmp/not-a-repo", None).expect("workspace");

    assert_eq!(workspace.slot_id, None);
    assert_eq!(workspace.execution_path, normalized);
    assert_eq!(workspace.selected_worktree_path.as_deref(), Some(normalized.as_str()));
    assert_eq!(workspace.selected_worktree_branch.as_deref(), Some("feature"));
    assert!(!super::claim_worktree_path(&normalized));

    drop(guard);
    assert!(super::claim_worktree_path(&normalized));
    super::release_claimed_worktree(&normalized);
}

#[test]
fn resolve_task_execution_workspace_without_selected_path_keeps_task_metadata() {
    let project_path = unique_claim_path("project");
    let mut task = crate::app::task::models::Task::new(1);
    task.selected_worktree_slot_id = Some("slot-7".into());
    task.selected_worktree_path = Some(unique_claim_path("missing-worktree"));
    task.merge_source_branch = Some(" feature/source ".into());
    task.merge_target_branch = Some(" release ".into());

    let (workspace, _guard) =
        super::resolve_task_execution_workspace(&task, &project_path, None).expect("workspace");

    assert_eq!(workspace.slot_id.as_deref(), Some("slot-7"));
    assert_eq!(workspace.execution_path, project_path);
    assert_eq!(workspace.project_path, project_path);
    assert_eq!(workspace.selected_worktree_path, None);
    assert_eq!(workspace.selected_worktree_branch.as_deref(), Some(" feature/source "));
    assert_eq!(workspace.merge_target_branch.as_deref(), Some("release"));
}

#[test]
fn resolve_workspace_does_not_release_a_path_claimed_by_another_execution() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let selected = temp.path().join("selected");
    std::fs::create_dir_all(&selected).expect("selected dir");
    let selected = super::normalize_path(&selected.to_string_lossy());

    assert!(super::claim_worktree_path(&selected));

    let mut task = crate::app::task::models::Task::new(1);
    task.selected_worktree_path = Some(selected.clone());
    task.merge_source_branch = Some("feature/claimed".into());
    task.merge_target_branch = Some("main".into());

    let (workspace, guard) =
        super::resolve_task_execution_workspace(&task, "/tmp/not-a-repo", None).expect("workspace");
    assert_eq!(workspace.selected_worktree_path.as_deref(), Some(selected.as_str()));

    drop(guard);
    assert!(!super::claim_worktree_path(&selected));
    super::release_claimed_worktree(&selected);
}

#[test]
fn resolve_workspace_rejects_invalid_source_branch_metadata() {
    let project_path = unique_claim_path("invalid-source");
    let mut task = crate::app::task::models::Task::new(1);
    task.merge_source_branch = Some(" HEAD ".into());
    task.merge_target_branch = Some("release".into());

    let (workspace, _guard) =
        super::resolve_task_execution_workspace(&task, &project_path, None).expect("workspace");

    assert_eq!(workspace.selected_worktree_branch, None);
    assert_eq!(workspace.merge_target_branch.as_deref(), Some("release"));
}

#[test]
fn repo_admin_entrypoints_reject_non_git_directories_with_clear_errors() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    let path = temp.path().to_string_lossy();

    assert!(
        super::reset_all_managed_worktrees(&path, false)
            .expect_err("reset outside git should fail")
            .contains("当前目录不在 git 仓库中")
    );
    assert!(
        super::delete_all_managed_worktrees(&path)
            .expect_err("delete outside git should fail")
            .contains("当前目录不在 git 仓库中")
    );
    assert!(
        super::commit_merge_all_worktrees(&path, &[])
            .expect_err("merge outside git should fail")
            .contains("无法批量合并")
    );
}

#[test]
fn reset_all_managed_worktrees_resets_slots_and_cleans_untracked_files() {
    let _env_lock = worktree_admin_env_lock();
    let (temp, repo) = create_git_repo();
    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).expect("home dir");
    let _home_guard = EnvGuard::set_os("HOME", home.as_os_str());
    let slot = managed_slot_path(&home, "slot-reset");
    add_worktree(&repo, &slot, "vw/reset");

    std::fs::write(slot.join("README.md"), "changed\n").expect("modify worktree");
    std::fs::write(slot.join("scratch.txt"), "temporary\n").expect("untracked file");
    let (sender, receiver) = std::sync::mpsc::channel();

    let count = super::reset_all_managed_worktrees_with_logs(
        repo.to_string_lossy().as_ref(),
        true,
        Some(&sender),
    )
    .expect("managed worktree should reset");

    assert_eq!(count, 1);
    assert_eq!(std::fs::read_to_string(slot.join("README.md")).expect("read seed"), "seed\n");
    assert!(!slot.join("scratch.txt").exists());
    assert!(receiver.try_iter().any(|entry| match entry {
        super::TaskLogStream::Stdout(message) => {
            message.contains("一键清理完成") && message.contains("1 个工作区")
        }
        _ => false,
    }));
}

#[test]
fn delete_all_managed_worktrees_removes_slots_branches_and_claims() {
    let _env_lock = worktree_admin_env_lock();
    let (temp, repo) = create_git_repo();
    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).expect("home dir");
    let _home_guard = EnvGuard::set_os("HOME", home.as_os_str());
    let slot = managed_slot_path(&home, "slot-delete");
    add_worktree(&repo, &slot, "vw/delete");
    let slot_string = slot.to_string_lossy().to_string();
    let normalized_slot = super::normalize_path(&slot_string);

    assert!(super::claim_worktree_path(&normalized_slot));
    let (sender, receiver) = std::sync::mpsc::channel();
    let count = super::delete_all_managed_worktrees_with_logs(
        repo.to_string_lossy().as_ref(),
        Some(&sender),
    )
    .expect("managed worktree should delete");

    assert_eq!(count, 1);
    assert!(!slot.exists());
    assert!(super::claim_worktree_path(&normalized_slot));
    super::release_claimed_worktree(&normalized_slot);

    let branches = run_git_test(&repo, &["branch", "--list", "vw/delete"]);
    assert!(String::from_utf8_lossy(&branches.stdout).trim().is_empty());
    assert!(receiver.try_iter().any(|entry| match entry {
        super::TaskLogStream::Stderr(message) => {
            message.contains("强制删除继续执行") && message.contains("正在使用中的 worktree")
        }
        _ => false,
    }));
}

#[test]
fn commit_merge_all_worktrees_commits_dirty_worktree_and_merges_branch() {
    let _env_lock = worktree_admin_env_lock();
    let (temp, repo) = create_git_repo();
    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).expect("home dir");
    let _home_guard = EnvGuard::set_os("HOME", home.as_os_str());
    let slot = managed_slot_path(&home, "slot-merge");
    add_worktree(&repo, &slot, "vw/merge");
    std::fs::write(slot.join("feature.txt"), "merged\n").expect("write feature file");

    let mut task = crate::app::task::models::Task::new(7);
    task.id = "T20260718.0007".to_string();
    task.merge_source_branch = Some("vw/merge".to_string());
    let (sender, receiver) = std::sync::mpsc::channel();

    let count = super::commit_merge_all_worktrees_with_logs(
        repo.to_string_lossy().as_ref(),
        &[task],
        Some(&sender),
    )
    .expect("dirty worktree branch should commit and merge");

    assert_eq!(count, 1);
    assert_eq!(std::fs::read_to_string(repo.join("feature.txt")).expect("merged file"), "merged\n");
    assert!(receiver.try_iter().any(|entry| match entry {
        super::TaskLogStream::Stdout(message) => {
            message.contains("批量合并完成") && message.contains("1 个分支")
        }
        _ => false,
    }));
}
