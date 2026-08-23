// 覆盖 Git 工作区状态及文本行增删统计。

use super::{Status, status_git};
use git2::Repository;
use std::path::Path;
use tempfile::TempDir;

fn signature() -> git2::Signature<'static> {
    git2::Signature::now("Unit Test", "unit@example.com").expect("signature")
}

fn write(path: impl AsRef<Path>, content: &str) {
    std::fs::write(path, content).expect("file should be written");
}

fn init_repository(worktree: &Path) -> Repository {
    let repo = Repository::init(worktree).expect("repo init");
    repo.config().expect("config").set_str("user.email", "unit@example.com").expect("user.email");
    repo.config().expect("config").set_str("user.name", "Unit Test").expect("user.name");
    repo
}

fn commit_all(repo: &Repository, message: &str) {
    let mut index = repo.index().expect("index");
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).expect("add all");
    index.write().expect("index write");
    let tree_id = index.write_tree().expect("tree");
    let tree = repo.find_tree(tree_id).expect("tree lookup");
    let sig = signature();
    let parents = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .into_iter()
        .collect::<Vec<_>>();
    let parent_refs = parents.iter().collect::<Vec<_>>();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs).expect("commit");
}

fn stage_path(repo: &Repository, relative: &str) {
    let mut index = repo.index().expect("index");
    index.add_path(Path::new(relative)).expect("stage path");
    index.write().expect("index write");
}

// 临时目录夹具同时覆盖正常路径和逃逸路径，确保安全判断不依赖当前工作区。
#[test]
fn status_git_returns_empty_for_non_git_directory() {
    let temp = TempDir::new().expect("temp dir");

    assert!(status_git(temp.path()).is_empty());
}

// 缺失资源与越界资源的语义不同：前者可安全回退时仍不得放宽后者的拒绝策略。
#[test]
fn status_git_returns_empty_for_missing_worktree() {
    let temp = TempDir::new().expect("temp dir");
    let missing = temp.path().join("missing");

    assert!(!missing.exists());
    assert!(status_git(missing).is_empty());
}

#[test]
fn status_git_returns_empty_for_clean_repository() {
    let temp = TempDir::new().expect("temp dir");
    let repo = init_repository(temp.path());
    write(temp.path().join("tracked.txt"), "one\ntwo\n");
    commit_all(&repo, "initial");

    assert!(status_git(temp.path()).is_empty());
}

#[test]
fn status_git_reports_staged_new_file_as_modified_once() {
    let temp = TempDir::new().expect("temp dir");
    let repo = init_repository(temp.path());
    write(temp.path().join("tracked.txt"), "tracked\n");
    commit_all(&repo, "initial");
    write(temp.path().join("staged.txt"), "alpha\nbeta\n");
    stage_path(&repo, "staged.txt");

    let changed = status_git(temp.path());
    let staged = changed.iter().filter(|info| info.path == "staged.txt").collect::<Vec<_>>();

    assert_eq!(staged.len(), 1);
    assert_eq!(staged[0].status, Status::Modified);
    assert_eq!(staged[0].added, 2);
    assert_eq!(staged[0].removed, 0);
}

#[test]
fn status_git_collects_modified_added_and_deleted_files() {
    let temp = TempDir::new().expect("temp dir");
    let repo = init_repository(temp.path());
    write(temp.path().join("tracked.txt"), "one\ntwo\n");
    write(temp.path().join("deleted.txt"), "gone\n");
    commit_all(&repo, "initial");

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
    let temp = TempDir::new().expect("temp dir");
    let repo = init_repository(temp.path());
    write(temp.path().join("tracked.txt"), "one\ntwo\nthree\n");
    commit_all(&repo, "initial");
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
    let temp = TempDir::new().expect("temp dir");
    let repo = init_repository(temp.path());
    write(temp.path().join("tracked.txt"), "tracked\n");
    commit_all(&repo, "initial");
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
    let temp = TempDir::new().expect("temp dir");
    let repo = init_repository(temp.path());
    write(temp.path().join("tracked.txt"), "tracked\n");
    commit_all(&repo, "initial");
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
    let temp = TempDir::new().expect("temp dir");
    let repo = init_repository(temp.path());
    std::fs::write(temp.path().join("data.bin"), [0_u8, 1, 2]).expect("binary file");
    commit_all(&repo, "initial");

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

    let temp = TempDir::new().expect("temp dir");
    Repository::init(temp.path()).expect("repo init");
    symlink("missing-target", temp.path().join("dangling")).expect("dangling symlink");

    let changed = status_git(temp.path());

    let added_path = changed
        .iter()
        .find(|info| info.path == "dangling" && info.status == Status::Added)
        .expect("untracked dangling symlink should be reported");
    assert_eq!(added_path.added, 0);
    assert_eq!(added_path.removed, 0);
}
