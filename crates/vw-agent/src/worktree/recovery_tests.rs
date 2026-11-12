use super::{recovery, registry};
use std::path::{Path, PathBuf};
use std::process::Command;
use vw_api_types::id::{ProjectId, ProjectScope};

// Git children briefly inherit the process FD table before exec. Serialize these
// subprocess-backed fixtures so one fixture cannot inherit another fixture's flock.
static GIT_PROCESS_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn git(root: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git").arg("-C").arg(root).args(args).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    output.stdout
}
async fn fixture() -> (tempfile::TempDir, PathBuf, registry::Record) {
    let temp = tempfile::tempdir().unwrap();
    let primary = temp.path().join("primary");
    std::fs::create_dir(&primary).unwrap();
    git(&primary, &["init", "-b", "main"]);
    std::fs::write(primary.join("tracked"), "base").unwrap();
    std::fs::write(primary.join("deleted"), "delete me").unwrap();
    git(&primary, &["add", "."]);
    git(
        &primary,
        &["-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-m", "init"],
    );
    let secondary = temp.path().join("secondary");
    git(&primary, &["worktree", "add", "-b", "feature", secondary.to_str().unwrap()]);
    let scope = ProjectScope::for_default_tenant(ProjectId::from(uuid::Uuid::new_v4().to_string()))
        .unwrap();
    let record = registry::register(&scope, &primary, &secondary).await.unwrap();
    (temp, primary, record)
}
fn blob(repo: &git2::Repository, reference: &str, path: &str) -> Vec<u8> {
    repo.revparse_single(&format!("{reference}:{path}"))
        .unwrap()
        .as_blob()
        .unwrap()
        .content()
        .to_vec()
}

#[tokio::test]
async fn worktree_recovery_snapshot_preserves_head_index_and_files() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (temp, primary, record) = fixture().await;
    let root = &record.directory;
    std::fs::write(root.join("tracked"), "staged").unwrap();
    git(root, &["add", "tracked"]);
    std::fs::write(root.join("tracked"), "unstaged\r\n").unwrap();
    std::fs::remove_file(root.join("deleted")).unwrap();
    std::fs::write(root.join("中文 binary"), [0, 255, 13, 10]).unwrap();
    std::fs::write(root.join(".gitignore"), "ignored\n").unwrap();
    std::fs::write(root.join("ignored"), "excluded").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        std::fs::write(temp.path().join("outside"), "secret").unwrap();
        symlink(temp.path().join("outside"), root.join("link")).unwrap();
        std::fs::write(root.join("executable"), "run").unwrap();
        std::fs::set_permissions(root.join("executable"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
    }
    let before_head = git(root, &["rev-parse", "HEAD"]);
    let before_head_bytes = std::fs::read(record.git_dir.join("HEAD")).unwrap();
    let before_config = std::fs::read(record.common_dir.join("config")).unwrap();
    let before_status = git(root, &["status", "--porcelain=v1", "-z"]);
    let before_index = std::fs::read(record.git_dir.join("index")).unwrap();
    let reference =
        recovery::prepare(&record.scope, &primary, &record.id, false).await.unwrap().reference;
    let repo = git2::Repository::open(&primary).unwrap();
    assert_eq!(blob(&repo, &reference, "index/tracked"), b"staged");
    assert_eq!(blob(&repo, &reference, "worktree/tracked"), b"unstaged\r\n");
    assert_eq!(blob(&repo, &reference, "worktree/中文 binary"), [0, 255, 13, 10]);
    assert_eq!(blob(&repo, &reference, "index.raw"), before_index);
    assert!(repo.revparse_single(&format!("{reference}:worktree/deleted")).is_err());
    assert!(repo.revparse_single(&format!("{reference}:worktree/ignored")).is_err());
    let manifest: serde_json::Value =
        serde_json::from_slice(&blob(&repo, &reference, "manifest.json")).unwrap();
    assert_eq!(manifest["record"]["id"], record.id);
    assert_eq!(manifest["head"], String::from_utf8(before_head.clone()).unwrap().trim());
    assert_eq!(manifest["branch"], "refs/heads/feature");
    assert_eq!(manifest["has_ignored_files"], true);
    #[cfg(unix)]
    {
        let tree = repo.revparse_single(&format!("{reference}:worktree")).unwrap();
        assert_eq!(tree.as_tree().unwrap().get_name("link").unwrap().filemode(), 0o120000);
        assert_eq!(tree.as_tree().unwrap().get_name("executable").unwrap().filemode(), 0o100755);
        assert_eq!(
            blob(&repo, &reference, "worktree/link"),
            temp.path().join("outside").as_os_str().as_encoded_bytes()
        );
    }
    assert_eq!(std::fs::read(record.git_dir.join("index")).unwrap(), before_index);
    assert_eq!(git(root, &["rev-parse", "HEAD"]), before_head);
    assert_eq!(std::fs::read(record.git_dir.join("HEAD")).unwrap(), before_head_bytes);
    assert_eq!(std::fs::read(record.common_dir.join("config")).unwrap(), before_config);
    assert_eq!(std::fs::read(root.join("tracked")).unwrap(), b"unstaged\r\n");
    assert_eq!(std::fs::read(root.join("中文 binary")).unwrap(), [0, 255, 13, 10]);
    assert_eq!(git(root, &["status", "--porcelain=v1", "-z"]), before_status);
}

#[tokio::test]
async fn worktree_recovery_snapshot_survives_gc() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    if let Some(primary) = std::env::var_os("SHIJU_RECOVERY_CHILD") {
        let repo = git2::Repository::open(primary).unwrap();
        let reference = std::env::var("SHIJU_RECOVERY_REF").unwrap();
        assert_eq!(blob(&repo, &reference, "worktree/untracked"), b"recover me");
        assert!(!blob(&repo, &reference, "manifest.json").is_empty());
        return;
    }
    let (_temp, primary, record) = fixture().await;
    std::fs::write(record.directory.join("untracked"), "recover me").unwrap();
    let reference =
        recovery::prepare(&record.scope, &primary, &record.id, false).await.unwrap().reference;
    git(&primary, &["worktree", "remove", "--force", record.directory.to_str().unwrap()]);
    git(&primary, &["branch", "-D", "feature"]);
    git(&primary, &["reflog", "expire", "--expire=now", "--all"]);
    git(&primary, &["gc", "--prune=now"]);
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "worktree::worktree::recovery_tests::worktree_recovery_snapshot_survives_gc",
            "--nocapture",
        ])
        .env("SHIJU_RECOVERY_CHILD", &primary)
        .env("SHIJU_RECOVERY_REF", &reference)
        .output()
        .unwrap();
    assert!(child.status.success(), "{}", String::from_utf8_lossy(&child.stderr));
    assert!(String::from_utf8_lossy(&child.stdout).contains("1 passed"));
}

#[tokio::test]
async fn worktree_recovery_snapshot_failure_keeps_checkout() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (_temp, primary, record) = fixture().await;
    let root = &record.directory;
    let blocker = record.common_dir.join("refs/shiju-worktree-recovery");
    std::fs::write(&blocker, "not a directory").unwrap();
    assert!(recovery::prepare(&record.scope, &primary, &record.id, false).await.is_err());
    assert_eq!(std::fs::read(root.join("tracked")).unwrap(), b"base");
    assert!(registry::authorize(&record.scope, &primary, &record.id).await.is_ok());
    std::fs::remove_file(blocker).unwrap();
    // A failed publication must leave the source usable for a subsequent attempt.
    assert!(recovery::prepare(&record.scope, &primary, &record.id, false).await.is_ok());
}

#[tokio::test]
async fn worktree_recovery_snapshot_automatic_skips_ignored_files() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (_temp, primary, record) = fixture().await;
    std::fs::write(record.directory.join(".gitignore"), "ignored\n").unwrap();
    std::fs::write(record.directory.join("ignored"), "keep me").unwrap();
    let error = recovery::prepare(&record.scope, &primary, &record.id, true).await.err().unwrap();
    assert!(error.to_string().contains("Ignored files"));
    assert_eq!(std::fs::read(record.directory.join("ignored")).unwrap(), b"keep me");
    assert!(git(&primary, &["for-each-ref", "refs/shiju-worktree-recovery"]).is_empty());
}

#[tokio::test]
async fn worktree_recovery_snapshot_rejects_unsupported_states() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (_temp, primary, record) = fixture().await;
    let nested = record.directory.join("nested");
    std::fs::create_dir(&nested).unwrap();
    git(&nested, &["init"]);
    let error = recovery::prepare(&record.scope, &primary, &record.id, false).await.err().unwrap();
    assert!(error.to_string().contains("Nested Git"));
    std::fs::remove_dir_all(nested).unwrap();
    let repo = git2::Repository::open(&record.directory).unwrap();
    let mut index = repo.index().unwrap();
    let mut ancestor = index.get_path(Path::new("tracked"), 0).unwrap();
    let mut ours = index.get_path(Path::new("tracked"), 0).unwrap();
    let mut theirs = index.get_path(Path::new("tracked"), 0).unwrap();
    ancestor.flags = (ancestor.flags & !0x3000) | 0x1000;
    ours.flags = (ours.flags & !0x3000) | 0x2000;
    theirs.flags = (theirs.flags & !0x3000) | 0x3000;
    index.remove_path(Path::new("tracked")).unwrap();
    index.add(&ancestor).unwrap();
    index.add(&ours).unwrap();
    index.add(&theirs).unwrap();
    index.write().unwrap();
    let before = std::fs::read(record.git_dir.join("index")).unwrap();
    let error = recovery::prepare(&record.scope, &primary, &record.id, false).await.err().unwrap();
    assert!(error.to_string().contains("Conflicted index"));
    assert_eq!(std::fs::read(record.git_dir.join("index")).unwrap(), before);
    assert!(record.directory.exists());
}

#[tokio::test]
async fn worktree_recovery_snapshot_revalidates_changes_and_holds_lock() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (_temp, primary, record) = fixture().await;
    let prepared = recovery::prepare(&record.scope, &primary, &record.id, false).await.unwrap();
    assert!(registry::lifecycle_lock(&primary).is_err());
    prepared.revalidate(&primary).await.unwrap();
    std::fs::write(record.directory.join("tracked"), "new edit").unwrap();
    assert!(prepared.revalidate(&primary).await.is_err());
    let reference = prepared.reference.clone();
    drop(prepared);
    assert!(registry::lifecycle_lock(&primary).is_ok());
    assert_eq!(
        blob(&git2::Repository::open(&primary).unwrap(), &reference, "worktree/tracked"),
        b"base"
    );
}

#[tokio::test]
async fn worktree_recovery_snapshot_scope_and_ref_cannot_be_injected() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (_temp, primary, record) = fixture().await;
    let other = ProjectScope::for_default_tenant(ProjectId::from("../../refs/heads/main")).unwrap();
    assert!(recovery::prepare(&other, &primary, &record.id, false).await.is_err());
    assert!(recovery::prepare(&record.scope, &primary, "../../heads/main", false).await.is_err());
    let prepared = recovery::prepare(&record.scope, &primary, &record.id, false).await.unwrap();
    let foreign = git2::Repository::open(&primary).unwrap();
    let head = foreign.head().unwrap().target().unwrap();
    foreign.reference(&prepared.reference, head, true, "simulate changed snapshot").unwrap();
    assert!(prepared.revalidate(&primary).await.is_err());
    assert!(record.directory.exists());
}

#[cfg(unix)]
#[tokio::test]
async fn worktree_recovery_snapshot_unreadable_entry_keeps_checkout() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (_temp, primary, record) = fixture().await;
    let fifo = record.directory.join("pipe");
    let path = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let error = recovery::prepare(&record.scope, &primary, &record.id, false).await.err().unwrap();
    assert!(error.to_string().contains("Special filesystem"));
    assert!(record.directory.join("tracked").exists());
    assert!(git(&primary, &["for-each-ref", "refs/shiju-worktree-recovery"]).is_empty());
}

#[tokio::test]
async fn worktree_recovery_snapshot_rejects_submodule_and_special_index() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (_temp, primary, record) = fixture().await;
    let repo = git2::Repository::open(&record.directory).unwrap();
    let mut index = repo.index().unwrap();
    let mut entry = index.get_path(Path::new("tracked"), 0).unwrap();
    entry.mode = 0o160000;
    entry.id = repo.head().unwrap().target().unwrap();
    index.add(&entry).unwrap();
    index.write().unwrap();
    let error = recovery::prepare(&record.scope, &primary, &record.id, false).await.err().unwrap();
    assert!(error.to_string().contains("Submodule"));
    entry.mode = 0o100644;
    entry.id = repo.blob(b"base").unwrap();
    entry.flags_extended = 0x4000; // skip-worktree: sparse index is not a complete checkout.
    index.add(&entry).unwrap();
    index.write().unwrap();
    let error = recovery::prepare(&record.scope, &primary, &record.id, false).await.err().unwrap();
    assert!(error.to_string().contains("Sparse or intent-to-add"));
    assert_eq!(std::fs::read(record.directory.join("tracked")).unwrap(), b"base");
}

#[tokio::test]
async fn worktree_recovery_snapshot_detached_head_and_staged_deletion() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (_temp, primary, record) = fixture().await;
    git(&record.directory, &["checkout", "--detach"]);
    git(&record.directory, &["rm", "deleted"]);
    git(&record.directory, &["config", "core.autocrlf", "true"]);
    std::fs::write(record.directory.join("tracked"), b"raw\r\nbytes\r\n").unwrap();
    let prepared = recovery::prepare(&record.scope, &primary, &record.id, false).await.unwrap();
    assert!(prepared.manifest.branch.is_none());
    let repo = git2::Repository::open(&primary).unwrap();
    assert!(repo.revparse_single(&format!("{}:index/deleted", prepared.reference)).is_err());
    assert_eq!(blob(&repo, &prepared.reference, "worktree/tracked"), b"raw\r\nbytes\r\n");
    assert_eq!(
        repo.find_reference(&prepared.reference)
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .parent_id(0)
            .unwrap()
            .to_string(),
        prepared.manifest.head
    );
}
