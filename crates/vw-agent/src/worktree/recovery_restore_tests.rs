use super::{recovery, registry};
use std::path::{Path, PathBuf};
use std::process::Command;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

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
    let tenant = TenantScope::new("restore-tests", uuid::Uuid::new_v4().to_string()).unwrap();
    let (project, _) = crate::project::from_directory(&tenant, &primary).await.unwrap();
    let scope = ProjectScope::new(tenant.clone(), ProjectId::from(project.id)).unwrap();
    crate::config::update_global_for_tenant(
        &tenant,
        serde_json::json!({"worktree_settings": {"root": temp.path().join("restored")}}),
    )
    .await
    .unwrap();
    let record = registry::register(&scope, &primary, &secondary).await.unwrap();
    (temp, primary, record)
}

async fn deleted(primary: &Path, record: &registry::Record) -> String {
    let snapshot = recovery::prepare(&record.scope, primary, &record.id, false).await.unwrap();
    let id = snapshot.manifest.recovery_id.clone();
    drop(snapshot);
    git(primary, &["worktree", "remove", "--force", record.directory.to_str().unwrap()]);
    registry::mark_deleted(&record.scope, &record.id).await.unwrap();
    id
}

#[tokio::test]
async fn worktree_recovery_restore_round_trips_staged_and_unstaged_changes() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (_temp, primary, record) = fixture().await;
    std::fs::write(record.directory.join("tracked"), "staged").unwrap();
    git(&record.directory, &["add", "tracked"]);
    std::fs::write(record.directory.join("tracked"), b"unstaged\r\n").unwrap();
    std::fs::remove_file(record.directory.join("deleted")).unwrap();
    std::fs::write(record.directory.join("中文 binary"), [0, 255, 13, 10]).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        symlink("tracked", record.directory.join("link")).unwrap();
        use std::os::unix::ffi::OsStrExt;
        symlink(std::ffi::OsStr::from_bytes(b"target\xff"), record.directory.join("raw-link"))
            .unwrap();
        std::fs::write(record.directory.join("executable"), b"run").unwrap();
        std::fs::set_permissions(
            record.directory.join("executable"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }
    let staged = git(&record.directory, &["diff", "--cached", "--binary"]);
    let unstaged = git(&record.directory, &["diff", "--binary"]);
    let id = deleted(&primary, &record).await;
    let future = recovery::restore(&record.scope, &primary, &id);
    fn assert_send<T: Send>(_: &T) {}
    assert_send(&future);
    let restored = future.await.unwrap();
    assert_ne!(restored.record.directory, record.directory);
    assert_eq!(restored.original_id, record.id);
    assert_eq!(restored.branch, "feature");
    assert!(!restored.branch_changed);
    assert_eq!(git(&restored.record.directory, &["diff", "--cached", "--binary"]), staged);
    assert_eq!(git(&restored.record.directory, &["diff", "--binary"]), unstaged);
    assert_eq!(
        std::fs::read(restored.record.directory.join("中文 binary")).unwrap(),
        [0, 255, 13, 10]
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::read_link(restored.record.directory.join("link")).unwrap(),
            Path::new("tracked")
        );
        use std::os::unix::ffi::OsStrExt;
        assert_eq!(
            std::fs::read_link(restored.record.directory.join("raw-link"))
                .unwrap()
                .as_os_str()
                .as_bytes(),
            b"target\xff"
        );
        assert_ne!(
            std::fs::metadata(restored.record.directory.join("executable"))
                .unwrap()
                .permissions()
                .mode()
                & 0o111,
            0
        );
    }
    registry::authorize(&record.scope, &primary, &restored.record.id).await.unwrap();
}

#[tokio::test]
async fn worktree_recovery_restore_rejects_other_tenant() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (_temp, primary, record) = fixture().await;
    let id = deleted(&primary, &record).await;
    let other = ProjectScope::new(
        vw_api_types::id::TenantScope::new("other", "company").unwrap(),
        record.scope.project_id().clone(),
    )
    .unwrap();
    assert!(recovery::restore(&other, &primary, &id).await.is_err());
    assert!(recovery::restore(&record.scope, &primary, &id).await.is_ok());
}

#[tokio::test]
async fn worktree_recovery_restore_handles_path_conflict() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (_temp, primary, record) = fixture().await;
    let id = deleted(&primary, &record).await;
    std::fs::create_dir(&record.directory).unwrap();
    std::fs::write(record.directory.join("sentinel"), "untouched").unwrap();
    // A foreign checkout now owns the original branch. Never move it.
    let foreign = primary.parent().unwrap().join("foreign");
    git(&primary, &["worktree", "add", foreign.to_str().unwrap(), "feature"]);
    let restored = recovery::restore(&record.scope, &primary, &id).await.unwrap();
    assert!(restored.branch_changed);
    assert_ne!(restored.branch, "feature");
    assert_eq!(std::fs::read(record.directory.join("sentinel")).unwrap(), b"untouched");
    assert!(recovery::restore(&record.scope, &primary, &id).await.is_err());
}

#[tokio::test]
async fn worktree_recovery_restore_failure_can_retry_and_preserves_snapshot() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (temp, primary, record) = fixture().await;
    let id = deleted(&primary, &record).await;
    let root = temp.path().join("restored");
    std::fs::write(&root, b"blocker").unwrap();
    assert!(recovery::restore(&record.scope, &primary, &id).await.is_err());
    assert_eq!(std::fs::read(&root).unwrap(), b"blocker");
    std::fs::remove_file(&root).unwrap();
    git(&primary, &["branch", "-D", "feature"]);
    let restored = recovery::restore(&record.scope, &primary, &id).await.unwrap();
    assert!(restored.branch_changed);
    assert_eq!(restored.record.directory.parent().unwrap(), std::fs::canonicalize(root).unwrap());
    assert!(!git(&primary, &["for-each-ref", "refs/shiju-worktree-recovery"]).is_empty());
}

#[tokio::test]
async fn worktree_recovery_restore_survives_process_restart() {
    if let Some(primary) = std::env::var_os("SHIJU_RESTORE_CHILD") {
        let scope: ProjectScope =
            serde_json::from_str(&std::env::var("SHIJU_RESTORE_SCOPE").unwrap()).unwrap();
        let id = std::env::var("SHIJU_RESTORE_ID").unwrap();
        let restored = recovery::restore(&scope, Path::new(&primary), &id).await.unwrap();
        assert_eq!(std::fs::read(restored.record.directory.join("tracked")).unwrap(), b"base");
        return;
    }
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (_temp, primary, record) = fixture().await;
    let id = deleted(&primary, &record).await;
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "worktree::worktree::recovery_restore_tests::worktree_recovery_restore_survives_process_restart", "--nocapture"])
        .env("SHIJU_RESTORE_CHILD", &primary)
        .env("SHIJU_RESTORE_SCOPE", serde_json::to_string(&record.scope).unwrap())
        .env("SHIJU_RESTORE_ID", &id).output().unwrap();
    assert!(
        child.status.success(),
        "{} {}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    assert!(String::from_utf8_lossy(&child.stdout).contains("1 passed"));
    // The receipt is durable as well: a new process's successful restore cannot repeat.
    assert!(recovery::restore(&record.scope, &primary, &id).await.is_err());
}

#[tokio::test]
async fn worktree_recovery_restore_rejects_foreign_repository_and_busy_operation() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (_temp, primary, record) = fixture().await;
    let id = deleted(&primary, &record).await;
    let lock = registry::lifecycle_lock(&primary).unwrap();
    assert!(recovery::restore(&record.scope, &primary, &id).await.is_err());
    drop(lock);
    let repo = git2::Repository::open(&primary).unwrap();
    let (_foreign_temp, foreign, _) = fixture().await;
    let other = git2::Repository::open(&foreign).unwrap();
    // Copy every object and the ref; matching object IDs do not grant repository ownership.
    let odb = repo.odb().unwrap();
    let target = other.odb().unwrap();
    odb.foreach(|oid| {
        let obj = odb.read(*oid).unwrap();
        target.write(obj.kind(), obj.data()).unwrap();
        true
    })
    .unwrap();
    let reference =
        repo.references_glob("refs/shiju-worktree-recovery/*/*").unwrap().next().unwrap().unwrap();
    other.reference(reference.name().unwrap(), reference.target().unwrap(), false, "test").unwrap();
    let error = recovery::restore(&record.scope, &foreign, &id).await.unwrap_err();
    assert!(error.to_string().contains("identity"));
    assert!(recovery::restore(&record.scope, &primary, &id).await.is_ok());
}

#[tokio::test]
async fn worktree_recovery_restore_rejects_unsafe_tree_before_writing() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (_temp, primary, record) = fixture().await;
    let id = deleted(&primary, &record).await;
    let repo = git2::Repository::open(&primary).unwrap();
    let reference_name = repo
        .references_glob("refs/shiju-worktree-recovery/*/*")
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .name()
        .unwrap()
        .to_owned();
    let commit = repo.find_reference(&reference_name).unwrap().peel_to_commit().unwrap();
    let mut malicious = repo.treebuilder(None).unwrap();
    malicious.insert("..\\escape", repo.blob(b"unsafe").unwrap(), 0o100644).unwrap();
    let original_tree = commit.tree().unwrap();
    let mut root = repo.treebuilder(Some(&original_tree)).unwrap();
    root.insert("worktree", malicious.write().unwrap(), 0o040000).unwrap();
    let bad_tree = repo.find_tree(root.write().unwrap()).unwrap();
    let signature = git2::Signature::now("Test", "test@example.com").unwrap();
    let malicious_commit = repo
        .commit(
            None,
            &signature,
            &signature,
            "malicious snapshot",
            &bad_tree,
            &[&commit.parent(0).unwrap()],
        )
        .unwrap();
    repo.reference(&reference_name, malicious_commit, true, "test malicious tree").unwrap();
    let error = recovery::restore(&record.scope, &primary, &id).await.unwrap_err();
    assert!(error.to_string().contains("Unsafe recovery"));
    assert!(!primary.parent().unwrap().join("restored").exists());
    repo.reference(&reference_name, commit.id(), true, "retry valid snapshot").unwrap();
    assert!(recovery::restore(&record.scope, &primary, &id).await.is_ok());
}

#[tokio::test]
async fn worktree_recovery_restore_registration_failure_rolls_back_only_new_resources() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (temp, primary, record) = fixture().await;
    let id = deleted(&primary, &record).await;
    git(&primary, &["branch", "-D", "feature"]);
    let before = git(&primary, &["branch", "--list"]);
    let encode = |s: &str| s.as_bytes().iter().map(|b| format!("{b:02x}")).collect::<String>();
    let key = format!(
        "{}-{}-{}",
        encode(record.scope.platform_id().as_ref()),
        encode(record.scope.company_id().as_ref()),
        encode(record.scope.project_id().as_ref())
    );
    let lock_path = vw_config_types::paths::home_config_dir(&crate::global::paths().home)
        .join("storage/managed-worktrees")
        .join(format!("{key}.lock"));
    let lock = std::fs::OpenOptions::new().read(true).write(true).open(lock_path).unwrap();
    lock.try_lock().unwrap();
    let error = recovery::restore(&record.scope, &primary, &id).await.unwrap_err();
    assert!(error.to_string().contains("registry is busy"));
    assert_eq!(git(&primary, &["branch", "--list"]), before);
    assert_eq!(
        git(&primary, &["worktree", "list", "--porcelain"])
            .windows(9)
            .filter(|s| *s == b"worktree ")
            .count(),
        1
    );
    assert!(std::fs::read_dir(temp.path().join("restored")).unwrap().next().is_none());
    drop(lock);
    assert!(recovery::restore(&record.scope, &primary, &id).await.is_ok());
}

#[tokio::test]
async fn worktree_recovery_restore_does_not_interpret_branch_names_as_options() {
    let _serial = GIT_PROCESS_TEST_LOCK.lock().await;
    let (_temp, primary, record) = fixture().await;
    git(&primary, &["update-ref", "refs/heads/--force", "HEAD"]);
    git(&record.directory, &["symbolic-ref", "HEAD", "refs/heads/--force"]);
    let before = git(&primary, &["rev-parse", "refs/heads/--force"]);
    let id = deleted(&primary, &record).await;
    let restored = recovery::restore(&record.scope, &primary, &id).await.unwrap();
    assert!(restored.branch_changed);
    assert_eq!(
        git(&restored.record.directory, &["symbolic-ref", "HEAD"]),
        format!("refs/heads/{}\n", restored.branch).as_bytes()
    );
    assert_eq!(git(&primary, &["rev-parse", "refs/heads/--force"]), before);
}
