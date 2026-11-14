use super::{backup_create::*, backup_delete::*, safety_snapshot::*};
use crate::usage::source_settings::{initialize_source_settings, load_source_settings};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

fn scope(company: &str) -> SafetySnapshotScope {
    SafetySnapshotScope {
        project: ProjectScope::new(
            TenantScope::new("fixture-platform", company).unwrap(),
            ProjectId::from("fixture-project"),
        )
        .unwrap(),
        principal_id: "fixture-user".into(),
        host_id: "fixture-host".into(),
    }
}
fn create(root: &Path, scope: &SafetySnapshotScope) -> LocalBackupReceipt {
    let mut connection = rusqlite::Connection::open_in_memory().unwrap();
    initialize_source_settings(&connection).unwrap();
    let transaction = connection.transaction().unwrap();
    let target = SnapshotTarget {
        category: SnapshotCategory::Preference,
        object_id: "usage-source/claude".into(),
    };
    let receipt = create_local_backup(root, scope, &[target.clone()], "Anonymous", || {
        let state =
            load_source_settings(&transaction, &scope.project, &scope.host_id, "claude").unwrap();
        Ok(vec![OwnerPreimage {
            target,
            state: Some(PresentPreimage {
                revision: state.settings_revision.to_string(),
                bytes: serde_json::to_vec(&state.settings).unwrap(),
            }),
        }])
    })
    .unwrap()
    .unwrap();
    transaction.commit().unwrap();
    receipt
}

#[test]
fn cc_data_25_delete_removes_selected_backup_only() {
    let temp = tempfile::tempdir().unwrap();
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = temp.path().canonicalize().unwrap();
    let current = scope("fixture-company");
    let selected = create(&root, &current);
    let retained = create(&root, &current);
    let foreign = create(&root, &scope("foreign-company"));
    let retained_path = root.join(format!("{}.local-backup", retained.id()));
    let before = fs::read(&retained_path).unwrap();
    assert_eq!(delete_local_backup(&root, &current, foreign.id()),
        Err(BackupDeleteError::Snapshot(SafetySnapshotError::ScopeMismatch)));
    assert!(delete_local_backup(&root, &current, "../escape").is_err());
    symlink(&retained_path, root.join("linked.local-backup")).unwrap();
    assert!(delete_local_backup(&root, &current, "linked").is_err());
    delete_local_backup(&root, &current, selected.id()).unwrap();
    assert_eq!(delete_local_backup(&root, &current, selected.id()), Err(BackupDeleteError::NotFound));
    assert_eq!(fs::read(&retained_path).unwrap(), before);
    assert!(root.join(format!("{}.local-backup", foreign.id())).exists());
}

#[test]
fn cc_data_25_delete_rejects_protected_restore_source() {
    let temp = tempfile::tempdir().unwrap();
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = temp.path().canonicalize().unwrap();
    let current = scope("fixture-company");
    let source = create(&root, &current);
    let rollback = create(&root, &current);
    let source_guard = open_local_backup(&root, &source, &current).unwrap();
    let rollback_guard = open_local_backup(&root, &rollback, &current).unwrap();
    for receipt in [&source, &rollback] {
        assert_eq!(delete_local_backup(&root, &current, receipt.id()), Err(BackupDeleteError::Protected));
    }
    drop(source_guard);
    delete_local_backup(&root, &current, source.id()).unwrap();
    assert_eq!(delete_local_backup(&root, &current, rollback.id()), Err(BackupDeleteError::Protected));
    drop(rollback_guard);
    delete_local_backup(&root, &current, rollback.id()).unwrap();
}
