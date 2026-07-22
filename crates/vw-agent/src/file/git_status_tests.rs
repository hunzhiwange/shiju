use super::{Status, status_git};
use std::ffi::OsString;
use std::path::Path;
use std::sync::MutexGuard;

struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.previous {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }
}

struct GitStatusTestGuard {
    _git_guard: EnvGuard,
    _env_lock: MutexGuard<'static, ()>,
}

fn git_status_test_guard() -> GitStatusTestGuard {
    let env_lock = crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner());
    let git_guard = EnvGuard::set("VIBEWINDOW_TEST_PREFER_SYSTEM_GIT", "1");
    GitStatusTestGuard { _git_guard: git_guard, _env_lock: env_lock }
}

fn git(worktree: &Path, args: &[&str]) {
    let status = crate::app::agent::shell::git_std_command()
        .args(args)
        .current_dir(worktree)
        .status()
        .expect("git command should start");
    assert!(status.success(), "git {:?} failed with {status}", args);
}

fn write(path: impl AsRef<Path>, content: &str) {
    std::fs::write(path, content).expect("file should be written");
}

fn init_repository(worktree: &Path) {
    git(worktree, &["init"]);
    git(worktree, &["config", "user.email", "unit@example.com"]);
    git(worktree, &["config", "user.name", "Unit Test"]);
    git(worktree, &["config", "commit.gpgsign", "false"]);
}

fn commit_all(worktree: &Path) {
    git(worktree, &["add", "."]);
    git(worktree, &["commit", "-m", "initial"]);
}

#[test]
fn status_git_returns_empty_for_non_git_directory() {
    let _guard = git_status_test_guard();
    let temp = tempfile::tempdir().expect("temp dir");

    assert!(status_git(temp.path()).is_empty());
}

#[test]
fn status_git_returns_empty_for_missing_worktree() {
    let _guard = git_status_test_guard();
    let temp = tempfile::tempdir().expect("temp dir");
    let missing = temp.path().join("missing");

    assert!(!missing.exists());
    assert!(status_git(missing).is_empty());
}

#[test]
fn status_git_returns_empty_for_clean_repository() {
    let _guard = git_status_test_guard();
    let temp = tempfile::tempdir().expect("temp dir");
    init_repository(temp.path());
    write(temp.path().join("tracked.txt"), "one\ntwo\n");
    commit_all(temp.path());

    assert!(status_git(temp.path()).is_empty());
}

#[test]
fn status_git_reports_staged_new_file_as_modified_once() {
    let _guard = git_status_test_guard();
    let temp = tempfile::tempdir().expect("temp dir");
    init_repository(temp.path());
    write(temp.path().join("tracked.txt"), "tracked\n");
    commit_all(temp.path());
    write(temp.path().join("staged.txt"), "alpha\nbeta\n");
    git(temp.path(), &["add", "staged.txt"]);

    let changed = status_git(temp.path());
    let staged = changed.iter().filter(|info| info.path == "staged.txt").collect::<Vec<_>>();

    assert_eq!(staged.len(), 1);
    assert_eq!(staged[0].status, Status::Modified);
    assert_eq!(staged[0].added, 2);
    assert_eq!(staged[0].removed, 0);
}

#[test]
fn status_git_collects_modified_added_and_deleted_files() {
    let _guard = git_status_test_guard();
    let temp = tempfile::tempdir().expect("temp dir");
    init_repository(temp.path());
    write(temp.path().join("tracked.txt"), "one\ntwo\n");
    write(temp.path().join("deleted.txt"), "gone\n");
    commit_all(temp.path());

    write(temp.path().join("tracked.txt"), "one\ntwo\nthree\n");
    std::fs::remove_file(temp.path().join("deleted.txt")).expect("tracked file should delete");
    write(temp.path().join("new.txt"), "alpha\nbeta\n");

    let changed = status_git(temp.path());

    let tracked = changed
        .iter()
        .find(|info| info.path == "tracked.txt" && info.status == Status::Modified)
        .expect("modified tracked file should be reported");
    assert_eq!(tracked.added, 1);
    assert_eq!(tracked.removed, 0);

    let added = changed
        .iter()
        .find(|info| info.path == "new.txt" && info.status == Status::Added)
        .expect("untracked file should be reported");
    assert_eq!(added.added, 2);
    assert_eq!(added.removed, 0);

    assert!(
        changed.iter().any(|info| info.path == "deleted.txt" && info.status == Status::Deleted)
    );
}

#[test]
fn status_git_reports_removed_lines_for_modified_text_file() {
    let _guard = git_status_test_guard();
    let temp = tempfile::tempdir().expect("temp dir");
    init_repository(temp.path());
    write(temp.path().join("tracked.txt"), "one\ntwo\nthree\n");
    commit_all(temp.path());
    write(temp.path().join("tracked.txt"), "one\n");

    let changed = status_git(temp.path());

    let tracked = changed
        .iter()
        .find(|info| info.path == "tracked.txt" && info.status == Status::Modified)
        .expect("modified tracked file should be reported");
    assert_eq!(tracked.added, 0);
    assert_eq!(tracked.removed, 2);
}

#[test]
fn status_git_counts_empty_and_unterminated_untracked_files() {
    let _guard = git_status_test_guard();
    let temp = tempfile::tempdir().expect("temp dir");
    init_repository(temp.path());
    write(temp.path().join("tracked.txt"), "tracked\n");
    commit_all(temp.path());
    write(temp.path().join("empty.txt"), "");
    write(temp.path().join("single.txt"), "one line");

    let changed = status_git(temp.path());

    let empty = changed
        .iter()
        .find(|info| info.path == "empty.txt" && info.status == Status::Added)
        .expect("empty untracked file should be reported");
    assert_eq!(empty.added, 0);
    assert_eq!(empty.removed, 0);

    let single = changed
        .iter()
        .find(|info| info.path == "single.txt" && info.status == Status::Added)
        .expect("unterminated untracked file should be reported");
    assert_eq!(single.added, 1);
    assert_eq!(single.removed, 0);
}

#[test]
fn status_git_preserves_unicode_and_spaces_in_untracked_path() {
    let _guard = git_status_test_guard();
    let temp = tempfile::tempdir().expect("temp dir");
    init_repository(temp.path());
    write(temp.path().join("tracked.txt"), "tracked\n");
    commit_all(temp.path());
    write(temp.path().join("notes 你好.txt"), "alpha\nbeta\n");

    let changed = status_git(temp.path());

    let added = changed
        .iter()
        .find(|info| info.path == "notes 你好.txt" && info.status == Status::Added)
        .expect("unicode untracked path should be reported without quoting");
    assert_eq!(added.added, 2);
    assert_eq!(added.removed, 0);
}

#[test]
fn status_git_treats_binary_numstat_counts_as_zero() {
    let _guard = git_status_test_guard();
    let temp = tempfile::tempdir().expect("temp dir");
    init_repository(temp.path());
    std::fs::write(temp.path().join("data.bin"), [0_u8, 1, 2]).expect("binary file");
    commit_all(temp.path());

    std::fs::write(temp.path().join("data.bin"), [0_u8, 3, 4, 5]).expect("binary file");

    let changed = status_git(temp.path());

    let binary = changed
        .iter()
        .find(|info| info.path == "data.bin" && info.status == Status::Modified)
        .expect("modified binary file should be reported");
    assert_eq!(binary.added, 0);
    assert_eq!(binary.removed, 0);
}

#[cfg(unix)]
#[test]
fn status_git_counts_unreadable_untracked_path_as_zero_lines() {
    use std::os::unix::fs::symlink;

    let _guard = git_status_test_guard();
    let temp = tempfile::tempdir().expect("temp dir");
    git(temp.path(), &["init"]);
    symlink("missing-target", temp.path().join("dangling")).expect("dangling symlink");

    let changed = status_git(temp.path());

    let added_path = changed
        .iter()
        .find(|info| info.path == "dangling" && info.status == Status::Added)
        .expect("untracked dangling symlink should be reported by git ls-files");
    assert_eq!(added_path.added, 0);
    assert_eq!(added_path.removed, 0);
}
