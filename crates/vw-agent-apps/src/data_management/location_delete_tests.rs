use super::{
    location_delete::*, locations::BackupLocationRoots, safety_snapshot::SafetySnapshotScope,
};
use std::{fs, os::unix::fs::symlink};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

fn scope() -> SafetySnapshotScope {
    SafetySnapshotScope {
        project: ProjectScope::new(
            TenantScope::new("fixture", "company").unwrap(),
            ProjectId::from("project"),
        )
        .unwrap(),
        principal_id: "user".into(),
        host_id: "host".into(),
    }
}

#[test]
fn cc_data_31_category_delete_removes_only_matching_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let scope = scope();
    let roots = BackupLocationRoots {
        scope: &scope,
        local_backups: &root,
        app: &root,
        device: &root,
        environment: &root,
    };
    fs::write(root.join("env-backup-fixture.json"), b"123").unwrap();
    fs::write(root.join("other.json"), b"keep").unwrap();
    fs::create_dir(root.join("env-backup-directory.json")).unwrap();
    let report = delete_location(&roots, &scope, "envVars").unwrap();
    assert_eq!(report.freed_bytes, 3);
    assert!(report.undeleted.is_empty());
    assert!(!root.join("env-backup-fixture.json").exists());
    assert_eq!(fs::read(root.join("other.json")).unwrap(), b"keep");
    assert!(root.join("env-backup-directory.json").is_dir());
}

#[test]
fn cc_data_31_category_delete_rejects_database() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let scope = scope();
    let roots = BackupLocationRoots {
        scope: &scope,
        local_backups: &root,
        app: &root,
        device: &root,
        environment: &root,
    };
    assert_eq!(
        delete_location(&roots, &scope, "database"),
        Err(LocationDeleteError::DatabaseNotDeletable)
    );
    assert_eq!(
        delete_location(&roots, &scope, "../elsewhere"),
        Err(LocationDeleteError::UnknownCategory)
    );
    assert_eq!(delete_location(&roots, &scope, "skills"), Err(LocationDeleteError::OwnerRequired));
    assert_eq!(
        delete_location(&roots, &scope, "codexHistoryUnify"),
        Err(LocationDeleteError::OwnerRequired)
    );
    let mut foreign = scope.clone();
    foreign.host_id = "foreign".into();
    assert!(delete_location(&roots, &foreign, "envVars").is_err());
}

#[test]
fn cc_data_31_category_delete_reports_protected_and_unsafe_entries() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let scope = scope();
    let roots = BackupLocationRoots {
        scope: &scope,
        local_backups: &root,
        app: &root,
        device: &root,
        environment: &root,
    };
    fs::write(root.join("outside"), b"keep").unwrap();
    fs::create_dir_all(root.join("toolsearch-backups/nested")).unwrap();
    fs::write(root.join("toolsearch-backups/nested/removed"), b"12345").unwrap();
    fs::write(root.join("toolsearch-backups/protected"), b"keep").unwrap();
    let protected = fs::File::open(root.join("toolsearch-backups/protected")).unwrap();
    super::backup_create_files::lock_backup(&protected, false).unwrap();
    symlink(root.join("outside"), root.join("toolsearch-backups/link")).unwrap();
    let report = delete_location(&roots, &scope, "legacyToolSearch").unwrap();
    assert_eq!(report.freed_bytes, 5);
    assert_eq!(report.undeleted.len(), 2);
    assert_eq!(fs::read(root.join("outside")).unwrap(), b"keep");
    assert!(root.join("toolsearch-backups/protected").exists());
    assert!(!root.join("toolsearch-backups/nested").exists());
}
