use super::registry;
use std::path::{Path, PathBuf};
use std::process::Command;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

fn git(root: &Path, args: &[&str]) {
    assert!(Command::new("git").arg("-C").arg(root).args(args).output().unwrap().status.success());
}
fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, ProjectScope) {
    let temp = tempfile::tempdir().unwrap();
    let primary = temp.path().join("primary");
    std::fs::create_dir(&primary).unwrap();
    git(&primary, &["init", "-b", "main"]);
    git(
        &primary,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "--allow-empty",
            "-m",
            "init",
        ],
    );
    let secondary = temp.path().join("secondary");
    git(&primary, &["worktree", "add", "-b", "feature", secondary.to_str().unwrap()]);
    let scope = ProjectScope::for_default_tenant(ProjectId::from(uuid::Uuid::new_v4().to_string()))
        .unwrap();
    (temp, primary, secondary, scope)
}
#[tokio::test]
async fn worktree_registry_survives_restart() {
    if let Some(primary) = std::env::var_os("SHIJU_REGISTRY_RESTART_PRIMARY") {
        let scope: ProjectScope =
            serde_json::from_str(&std::env::var("SHIJU_REGISTRY_RESTART_SCOPE").unwrap()).unwrap();
        let records = registry::list(&scope, Path::new(&primary)).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, std::env::var("SHIJU_REGISTRY_RESTART_ID").unwrap());
        return;
    }
    let (_temp, primary, secondary, scope) = fixture();
    assert!(registry::list(&scope, &primary).await.unwrap().is_empty());
    let record = registry::register(&scope, &primary, &secondary).await.unwrap();
    let loaded = registry::list(&scope, &primary).await.unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(record.id, loaded[0].id);
    assert!(registry::authorize(&scope, &primary, &record.id).await.is_ok());
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "worktree::worktree::registry_tests::worktree_registry_survives_restart",
            "--nocapture",
        ])
        .env("SHIJU_REGISTRY_RESTART_PRIMARY", &primary)
        .env("SHIJU_REGISTRY_RESTART_SCOPE", serde_json::to_string(&scope).unwrap())
        .env("SHIJU_REGISTRY_RESTART_ID", &record.id)
        .output()
        .unwrap();
    assert!(child.status.success(), "{}", String::from_utf8_lossy(&child.stderr));
    assert!(String::from_utf8_lossy(&child.stdout).contains("1 passed"));
}
#[tokio::test]
async fn worktree_registry_rejects_external_or_primary_delete() {
    let (temp, primary, secondary, scope) = fixture();
    assert!(registry::register(&scope, &primary, &primary).await.is_err());
    assert!(registry::register(&scope, &secondary, &primary).await.is_err());
    assert!(registry::register(&scope, &primary, temp.path()).await.is_err());
    assert!(registry::authorize(&scope, &primary, "fake").await.is_err());
    let (_other_temp, _other_primary, other_checkout, _other_scope) = fixture();
    assert!(registry::register(&scope, &primary, &other_checkout).await.is_err());
    let record = registry::register(&scope, &primary, &secondary).await.unwrap();
    let other =
        ProjectScope::new(TenantScope::new("other", "other").unwrap(), scope.project_id().clone())
            .unwrap();
    assert!(registry::authorize(&other, &primary, &record.id).await.is_err());
    let other = ProjectScope::for_default_tenant(ProjectId::from("another-project")).unwrap();
    assert!(registry::authorize(&other, &primary, &record.id).await.is_err());
    assert!(primary.exists());
}
#[tokio::test]
async fn worktree_registry_rejects_replaced_checkout() {
    let (temp, primary, secondary, scope) = fixture();
    let record = registry::register(&scope, &primary, &secondary).await.unwrap();
    std::fs::rename(&secondary, temp.path().join("original")).unwrap();
    std::fs::create_dir(&secondary).unwrap();
    assert!(registry::authorize(&scope, &primary, &record.id).await.is_err());
    assert!(registry::list(&scope, &primary).await.unwrap().is_empty());
}
#[cfg(unix)]
#[tokio::test]
async fn worktree_registry_rejects_symlink_replacement() {
    let (temp, primary, secondary, scope) = fixture();
    let record = registry::register(&scope, &primary, &secondary).await.unwrap();
    let moved = temp.path().join("moved");
    std::fs::rename(&secondary, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &secondary).unwrap();
    assert!(registry::authorize(&scope, &primary, &record.id).await.is_err());
}

#[tokio::test]
async fn worktree_registry_managed_delete_rejects_foreign_owner_and_repeat() {
    let (_temp, primary, secondary, scope) = fixture();
    let record = registry::register(&scope, &primary, &secondary).await.unwrap();
    let other = ProjectScope::for_default_tenant(ProjectId::from("foreign-project")).unwrap();
    assert!(registry::remove_managed(&other, &primary, &record.id, true).await.is_err());
    assert!(secondary.exists());
    assert!(registry::remove_managed(&scope, &primary, &record.id, true).await.unwrap());
    assert!(!secondary.exists());
    assert!(registry::list(&scope, &primary).await.unwrap().is_empty());
    assert!(registry::remove_managed(&scope, &primary, &record.id, true).await.is_err());
    assert!(primary.exists());
}

#[tokio::test]
async fn worktree_registry_rejects_forged_or_missing_credential() {
    let (_temp, primary, secondary, scope) = fixture();
    let record = registry::register(&scope, &primary, &secondary).await.unwrap();
    let marker = record.git_dir.join("shiju-managed.json");
    let mut forged = record.clone();
    forged.id = "forged".into();
    std::fs::write(&marker, serde_json::to_vec(&forged).unwrap()).unwrap();
    assert!(registry::authorize(&scope, &primary, &record.id).await.is_err());
    std::fs::remove_file(marker).unwrap();
    assert!(registry::authorize(&scope, &primary, &record.id).await.is_err());
    assert!(secondary.exists());
}

#[test]
fn worktree_registry_lifecycle_lock_excludes_another_process() {
    if let Some(primary) = std::env::var_os("SHIJU_REGISTRY_LOCK_CHILD") {
        assert!(registry::lifecycle_lock(Path::new(&primary)).is_err());
        return;
    }
    let (_temp, primary, _secondary, _scope) = fixture();
    let lock = registry::lifecycle_lock(&primary).unwrap();
    assert!(registry::lifecycle_lock(&primary).is_err());
    let child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "worktree::worktree::registry_tests::worktree_registry_lifecycle_lock_excludes_another_process", "--nocapture"])
        .env("SHIJU_REGISTRY_LOCK_CHILD", &primary).output().unwrap();
    assert!(child.status.success(), "{}", String::from_utf8_lossy(&child.stderr));
    assert!(String::from_utf8_lossy(&child.stdout).contains("1 passed"));
    drop(lock);
    assert!(registry::lifecycle_lock(&primary).is_ok());
}

#[tokio::test]
async fn worktree_registry_rejects_replaced_git_metadata() {
    let (temp, primary, secondary, scope) = fixture();
    let record = registry::register(&scope, &primary, &secondary).await.unwrap();
    let moved = temp.path().join("old-admin");
    std::fs::rename(&record.git_dir, &moved).unwrap();
    std::fs::create_dir(&record.git_dir).unwrap();
    for entry in std::fs::read_dir(&moved).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            std::fs::copy(entry.path(), record.git_dir.join(entry.file_name())).unwrap();
        }
    }
    assert!(registry::authorize(&scope, &primary, &record.id).await.is_err());
    assert!(secondary.exists());
}
