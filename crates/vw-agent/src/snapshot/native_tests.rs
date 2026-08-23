//! 验证原生文件快照的创建、差异计算和恢复边界。

use super::*;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::sync::MutexGuard;

#[cfg(not(target_arch = "wasm32"))]
struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

#[cfg(not(target_arch = "wasm32"))]
impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, previous }
    }
}

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
struct GitTestGuard {
    _git_guard: EnvGuard,
    _env_lock: MutexGuard<'static, ()>,
}

#[cfg(not(target_arch = "wasm32"))]
fn git_test_guard() -> GitTestGuard {
    let env_lock = crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner());
    let git_guard = EnvGuard::set("VIBEWINDOW_TEST_PREFER_SYSTEM_GIT", "1");
    GitTestGuard { _git_guard: git_guard, _env_lock: env_lock }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn ensure_gitdir_reports_whether_directory_was_created() {
    let dir = tempfile::tempdir().expect("temp dir");
    let git = dir.path().join("snapshot.git");

    assert!(ensure_gitdir(&git).expect("create gitdir"));
    assert!(git.is_dir());
    assert!(!ensure_gitdir(&git).expect("existing gitdir"));
}

#[cfg(not(target_arch = "wasm32"))]
fn git(repo: &Path, args: &[&str]) -> String {
    let output = crate::app::agent::shell::git_std_command()
        .current_dir(repo)
        .env("GIT_AUTHOR_NAME", "Snapshot Test")
        .env("GIT_AUTHOR_EMAIL", "snapshot@example.com")
        .env("GIT_COMMITTER_NAME", "Snapshot Test")
        .env("GIT_COMMITTER_EMAIL", "snapshot@example.com")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn temp_git_worktree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init"]);
    git(dir.path(), &["config", "user.name", "Snapshot Test"]);
    git(dir.path(), &["config", "user.email", "snapshot@example.com"]);
    fs::write(dir.path().join("file.txt"), "one\n").unwrap();
    fs::write(dir.path().join("repo-id.txt"), dir.path().to_string_lossy().as_bytes()).unwrap();
    git(dir.path(), &["add", "."]);
    git(dir.path(), &["commit", "-m", "init"]);
    dir
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn file_relative_strips_worktree_prefix_and_preserves_external_path() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("nested").join("file.txt");
    let external = Path::new("elsewhere").join("file.txt");

    assert_eq!(
        file_relative(dir.path(), &nested.to_string_lossy()),
        Path::new("nested").join("file.txt").to_string_lossy().to_string()
    );
    assert_eq!(
        file_relative(dir.path(), &external.to_string_lossy()),
        external.to_string_lossy().to_string()
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn root_commit_id_returns_none_for_repository_without_references() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    assert_eq!(root_commit_id(&repo), None);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn show_file_returns_empty_when_revision_path_is_missing() {
    let _git_guard = git_test_guard();
    let dir = temp_git_worktree();
    let git_dir = dir.path().join(".git");

    let content = show_file(dir.path(), &git_dir, "missing-revision", "file.txt").unwrap();

    assert!(content.is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn snapshot_operations_return_empty_outside_git_repo() {
    let dir = tempfile::tempdir().unwrap();

    cleanup(dir.path()).unwrap();
    assert_eq!(track(dir.path()).unwrap(), None);
    assert!(patch(dir.path(), "missing").unwrap().files.is_empty());
    assert_eq!(diff(dir.path(), "missing").unwrap(), "");
    assert!(diff_full(dir.path(), "from", "to").unwrap().is_empty());
    restore(dir.path(), "missing").unwrap();
    revert(dir.path(), &[Patch { hash: "missing".to_string(), files: vec![] }]).unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn track_diff_patch_restore_and_revert_roundtrip() {
    let _git_guard = git_test_guard();
    let dir = temp_git_worktree();
    let worktree = dir.path();
    let baseline = track(worktree).unwrap().expect("baseline snapshot");

    fs::write(worktree.join("file.txt"), "two\n").unwrap();
    fs::write(worktree.join("added.txt"), "added\n").unwrap();

    let changed = patch(worktree, &baseline).unwrap();
    assert_eq!(changed.hash, baseline);
    assert!(changed.files.iter().any(|file| file.ends_with("file.txt")));
    assert!(changed.files.iter().any(|file| file.ends_with("added.txt")));

    let diff_text = diff(worktree, &changed.hash).unwrap();
    assert!(diff_text.contains("-one"));
    assert!(diff_text.contains("+two"));
    assert!(diff_text.contains("added.txt"));

    let modified = track(worktree).unwrap().expect("modified snapshot");
    let full = diff_full(worktree, &changed.hash, &modified).unwrap();
    assert!(full.iter().any(|file| {
        file.file == "file.txt"
            && file.before == "one\n"
            && file.after == "two\n"
            && file.status == Some(DiffStatus::Modified)
    }));
    assert!(full.iter().any(|file| {
        file.file == "added.txt"
            && file.after == "added\n"
            && file.status == Some(DiffStatus::Added)
    }));

    revert(worktree, &[changed]).unwrap();
    assert_eq!(fs::read_to_string(worktree.join("file.txt")).unwrap(), "one\n");
    assert!(!worktree.join("added.txt").exists());

    fs::write(worktree.join("file.txt"), "three\n").unwrap();
    restore(worktree, &modified).unwrap();
    assert_eq!(fs::read_to_string(worktree.join("file.txt")).unwrap(), "two\n");
    assert_eq!(fs::read_to_string(worktree.join("added.txt")).unwrap(), "added\n");

    cleanup(worktree).unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn revert_uses_first_patch_for_duplicate_file() {
    let _git_guard = git_test_guard();
    let dir = temp_git_worktree();
    let worktree = dir.path();
    let file = worktree.join("file.txt");
    let baseline = track(worktree).unwrap().expect("baseline snapshot");

    fs::write(&file, "two\n").unwrap();
    let modified = track(worktree).unwrap().expect("modified snapshot");
    fs::write(&file, "three\n").unwrap();

    revert(
        worktree,
        &[
            Patch { hash: baseline, files: vec![file.to_string_lossy().to_string()] },
            Patch { hash: modified, files: vec![file.to_string_lossy().to_string()] },
        ],
    )
    .unwrap();

    assert_eq!(fs::read_to_string(file).unwrap(), "one\n");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn diff_full_reports_deleted_and_binary_files() {
    let _git_guard = git_test_guard();
    let dir = temp_git_worktree();
    let worktree = dir.path();
    let binary = worktree.join("binary.bin");
    fs::write(&binary, [0_u8, 1, 2]).unwrap();
    let baseline = track(worktree).unwrap().expect("baseline snapshot");

    fs::remove_file(worktree.join("file.txt")).unwrap();
    fs::write(&binary, [0_u8, 3, 4]).unwrap();
    let changed = track(worktree).unwrap().expect("changed snapshot");

    let full = diff_full(worktree, &baseline, &changed).unwrap();
    let deleted = full.iter().find(|file| file.file == "file.txt").unwrap();
    assert_eq!(deleted.before, "one\n");
    assert_eq!(deleted.after, "");
    assert_eq!(deleted.additions, 0);
    assert_eq!(deleted.deletions, 1);
    assert_eq!(deleted.status, Some(DiffStatus::Deleted));

    let binary = full.iter().find(|file| file.file == "binary.bin").unwrap();
    assert_eq!(binary.before, "");
    assert_eq!(binary.after, "");
    assert_eq!(binary.additions, 0);
    assert_eq!(binary.deletions, 0);
    assert_eq!(binary.status, Some(DiffStatus::Modified));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn invalid_snapshot_hashes_return_empty_outputs_without_error() {
    let _git_guard = git_test_guard();
    let dir = temp_git_worktree();
    let worktree = dir.path();
    assert!(track(worktree).unwrap().is_some());

    assert!(patch(worktree, "not-a-valid-snapshot").unwrap().files.is_empty());
    assert_eq!(diff(worktree, "not-a-valid-snapshot").unwrap(), "");
    assert!(diff_full(worktree, "bad-from", "bad-to").unwrap().is_empty());
    restore(worktree, "not-a-valid-snapshot").unwrap();

    cleanup(worktree).unwrap();
}
