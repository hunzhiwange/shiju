use super::{
    backup_create::create_local_backup,
    recovery::{TargetHealth, diagnose_recovery},
    safety_snapshot::*,
};
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

#[test]
fn cc_data_27_recovery_lists_backups_with_corrupt_target() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = root.path().canonicalize().unwrap();
    let scope = SafetySnapshotScope {
        project: ProjectScope::new(
            TenantScope::new("fixture-platform", "fixture-company").unwrap(),
            ProjectId::from("fixture-project"),
        ).unwrap(),
        principal_id: "fixture-user".into(),
        host_id: "fixture-host".into(),
    };
    let target = SnapshotTarget {
        category: SnapshotCategory::Preference,
        object_id: "fixture-preference".into(),
    };
    let receipt = create_local_backup(&root, &scope, &[target.clone()], "fixture", || {
        Ok(vec![OwnerPreimage { target, state: None }])
    }).unwrap().unwrap();
    let damaged = tempfile::tempdir().unwrap();
    let database = damaged.path().join("target.db");
    fs::write(&database, b"damaged database evidence").unwrap();
    let diagnosis = diagnose_recovery(&root, &scope, || {
        let connection = rusqlite::Connection::open_with_flags(
            &database, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ).unwrap();
        assert!(connection.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0)).is_err());
        TargetHealth::Corrupt
    }).unwrap();
    assert_eq!(diagnosis.target, TargetHealth::Corrupt);
    let backups = diagnosis.backups.unwrap();
    assert_eq!(backups.len(), 1);
    assert_eq!(backups[0].id, receipt.id());
    assert_eq!(fs::read(database).unwrap(), b"damaged database evidence");
}

#[test]
fn cc_data_27_recovery_failure_preserves_original_evidence() {
    use super::{backup_restore::restore_local_backup, retention::RetentionPolicy};
    use vw_api_types::agent_apps::DataManagementTarget;
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = root.path().canonicalize().unwrap();
    let scope = SafetySnapshotScope {
        project: ProjectScope::new(
            TenantScope::new("fixture-platform", "fixture-company").unwrap(),
            ProjectId::from("fixture-project"),
        ).unwrap(),
        principal_id: "fixture-user".into(), host_id: "fixture-host".into(),
    };
    let target = SnapshotTarget { category: SnapshotCategory::Preference, object_id: "fixture".into() };
    let receipt = create_local_backup(&root, &scope, &[target.clone()], "fixture", || {
        Ok(vec![OwnerPreimage { target, state: None }])
    }).unwrap().unwrap();
    let evidence = tempfile::tempdir().unwrap();
    let database = evidence.path().join("target.db");
    fs::write(&database, b"corrupt original").unwrap();
    let target = DataManagementTarget {
        scope: scope.project.clone(), principal_id: scope.principal_id.clone(), target_host_id: scope.host_id.clone(),
    };
    let result = restore_local_backup::<()>(
        &root, &root, &receipt, &target, RetentionPolicy::default(),
        |_| Ok(()), || Err(SafetySnapshotError::CaptureFailed),
        |_| panic!("persist must not run"), |_| panic!("write must not run"),
        |_, _| panic!("projection must not run"),
    );
    assert!(matches!(result, Err(SafetySnapshotError::CaptureFailed)));
    assert_eq!(fs::read(database).unwrap(), b"corrupt original");
    assert!(super::backup_create::open_local_backup(&root, &receipt, &scope).is_ok());
}
