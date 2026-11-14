use super::{backup_create::*, retention::*, safety_snapshot::*};
use crate::usage::source_settings::{initialize_source_settings, load_source_settings};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
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
fn cc_data_28_retention_keeps_protected_snapshots() {
    let temp = tempfile::tempdir().unwrap();
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = temp.path().canonicalize().unwrap();
    let current = scope("fixture-company");
    let source = create(&root, &current);
    let rollback = create(&root, &current);
    let foreign = create(&root, &scope("foreign-company"));
    let source_guard = open_local_backup(&root, &source, &current).unwrap();
    let rollback_guard = open_local_backup(&root, &rollback, &current).unwrap();
    // Same inode under a case-equivalent path remains locked on a
    // case-insensitive volume. Case-sensitive volumes do not have this alias.
    let alias = root.join(format!("{}.LOCAL-BACKUP", source.id().to_uppercase()));
    if alias.exists() {
        use super::backup_delete::{BackupDeleteError, delete_local_backup};
        assert_eq!(delete_local_backup(&root, &current, source.id()), Err(BackupDeleteError::Protected));
    }
    let removable = create(&root, &current);
    let foreign_path = root.join(format!("{}.local-backup", foreign.id()));
    let before = fs::read(&foreign_path).unwrap();
    let report = cleanup_local_backups(&root, &current, RetentionPolicy::new(1).unwrap()).unwrap();
    assert_eq!(report.deleted, vec![removable.id().to_owned()]);
    assert_eq!(report.protected.len(), 2);
    assert_eq!(fs::read(foreign_path).unwrap(), before);
    drop((source_guard, rollback_guard));
    let report = cleanup_local_backups(&root, &current, RetentionPolicy::new(0).unwrap()).unwrap();
    assert_eq!(report.deleted.len(), 1);
    assert_eq!(super::backup_list::list_local_backups(&root, &current).unwrap().len(), 1);
}

#[test]
fn cc_data_28_retention_rejects_invalid_policy() {
    assert_eq!(RetentionPolicy::new(-1), Err(SafetySnapshotError::InvalidSelection));
    assert_eq!(RetentionPolicy::new(i64::from(u32::MAX) + 1), Err(SafetySnapshotError::InvalidSelection));
    assert_eq!(RetentionPolicy::new(0), RetentionPolicy::new(1));
    assert_eq!(RetentionPolicy::default(), RetentionPolicy::new(10).unwrap());
}

#[test]
fn cc_data_28_retention_orders_equal_timestamps_by_id() {
    let temp = tempfile::tempdir().unwrap();
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = temp.path().canonicalize().unwrap();
    let current = scope("fixture-company");
    let a = create(&root, &current);
    let b = create(&root, &current);
    for receipt in [&a, &b] {
        let path = root.join(format!("{}.local-backup", receipt.id()));
        let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["created_at_ms"] = 1.into();
        fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    }
    let report = cleanup_local_backups(&root, &current, RetentionPolicy::new(1).unwrap()).unwrap();
    assert_eq!(report.deleted, vec![a.id().min(b.id()).to_owned()]);
}
