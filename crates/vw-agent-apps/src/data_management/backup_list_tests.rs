use super::{backup_create::*, backup_list::*, safety_snapshot::*};
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
fn cc_data_23_backup_list_returns_visible_metadata() {
    let temp = tempfile::tempdir().unwrap();
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = temp.path().canonicalize().unwrap();
    let scope = scope("fixture-company");
    let receipt = create(&root, &scope);
    let path = root.join(format!("{}.local-backup", receipt.id()));
    let original = fs::read(&path).unwrap();
    let entries = list_local_backups(&root, &scope).unwrap();
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.id, receipt.id());
    assert_eq!(
        entry.kind,
        vw_api_types::agent_apps::data_management::DataArtifactKind::LocalRollback
    );
    assert_eq!(entry.format_version, 1);
    assert_eq!(entry.size_bytes, original.len() as u64);
    assert_eq!(
        entry.created_at_ms,
        open_local_backup(&root, &receipt, &scope).unwrap().created_at_ms()
    );
    assert_eq!(entry.restore_status, BackupRestoreStatus::RequiresOwnerValidation);
    assert_eq!(fs::read(&path).unwrap(), original);
    let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
    value["format_version"] = 99.into();
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(
        list_local_backups(&root, &scope).unwrap()[0].restore_status,
        BackupRestoreStatus::VersionUnsupported
    );
    value["format_version"] = 1.into();
    value["preimages"] = serde_json::json!([]);
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert_eq!(
        list_local_backups(&root, &scope).unwrap()[0].restore_status,
        BackupRestoreStatus::Corrupt
    );
    fs::write(&path, b"unidentifiable corruption").unwrap();
    assert!(list_local_backups(&root, &scope).unwrap().is_empty());
}

#[test]
fn cc_data_23_backup_list_hides_other_tenant() {
    let temp = tempfile::tempdir().unwrap();
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = temp.path().canonicalize().unwrap();
    let current = scope("fixture-company");
    let receipt = create(&root, &current);
    create(&root, &scope("foreign-company"));
    for foreign in [
        SafetySnapshotScope { host_id: "foreign-host".into(), ..current.clone() },
        SafetySnapshotScope { principal_id: "foreign-user".into(), ..current.clone() },
        SafetySnapshotScope {
            project: ProjectScope::new(
                current.project.tenant_scope().clone(),
                ProjectId::from("foreign-project"),
            )
            .unwrap(),
            ..current.clone()
        },
    ] {
        create(&root, &foreign);
    }
    let path = root.join(format!("{}.local-backup", receipt.id()));
    symlink(&path, root.join("linked.local-backup")).unwrap();
    let entries = list_local_backups(&root, &current).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, receipt.id());
    fs::hard_link(&path, root.join("hardlinked.local-backup")).unwrap();
    assert!(list_local_backups(&root, &current).unwrap().is_empty());
}
