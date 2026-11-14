use super::{backup_create::*, backup_restore::*, retention::RetentionPolicy, safety_snapshot::*};
use crate::usage::source_settings::{initialize_source_settings, load_source_settings, save_source_settings, UsageSourceSettings};
use rusqlite::Connection;
use std::{cell::RefCell, fs, os::unix::fs::PermissionsExt};
use vw_api_types::{agent_apps::DataManagementTarget, id::{ProjectId, ProjectScope, TenantScope}};

fn scope() -> SafetySnapshotScope {
    SafetySnapshotScope {
        project: ProjectScope::new(TenantScope::new("fixture-platform", "fixture-company").unwrap(), ProjectId::from("fixture-project")).unwrap(),
        principal_id: "fixture-user".into(), host_id: "fixture-host".into(),
    }
}
fn target() -> SnapshotTarget {
    SnapshotTarget { category: SnapshotCategory::Preference, object_id: "usage-source/claude".into() }
}
fn capture(db: &Connection, scope: &SafetySnapshotScope) -> Vec<OwnerPreimage> {
    let state = load_source_settings(db, &scope.project, &scope.host_id, "claude").unwrap();
    vec![OwnerPreimage { target: target(), state: Some(PresentPreimage {
        revision: state.settings_revision.to_string(), bytes: serde_json::to_vec(&state.settings).unwrap(),
    }) }]
}

#[test]
fn cc_data_26_restore_retain_one_keeps_safety_point() {
    let root = tempfile::tempdir().unwrap();
    let safety = tempfile::tempdir().unwrap();
    for path in [root.path(), safety.path()] { fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap(); }
    let root = root.path().canonicalize().unwrap();
    let safety = safety.path().canonicalize().unwrap();
    let scope = scope();
    let db = RefCell::new(Connection::open_in_memory().unwrap());
    initialize_source_settings(&db.borrow()).unwrap();
    let original = UsageSourceSettings { scan_enabled: false, show_usage: true, refresh_seconds: Some(90) };
    save_source_settings(&mut db.borrow_mut(), &scope.project, &scope.host_id, "claude", 0, original.clone()).unwrap();
    let source = create_local_backup(&root, &scope, &[target()], "fixture", || Ok(capture(&db.borrow(), &scope))).unwrap().unwrap();
    save_source_settings(&mut db.borrow_mut(), &scope.project, &scope.host_id, "claude", 1, UsageSourceSettings::default()).unwrap();
    let before = capture(&db.borrow(), &scope);
    let target_scope = DataManagementTarget { scope: scope.project.clone(), principal_id: scope.principal_id.clone(), target_host_id: scope.host_id.clone() };
    let report = restore_local_backup(&root, &safety, &source, &target_scope, RetentionPolicy::new(1).unwrap(),
        |images| {
            for image in images { serde_json::from_slice::<UsageSourceSettings>(&image.state.as_ref().unwrap().bytes).map_err(|_| SafetySnapshotError::Corrupt)?; }
            Ok(())
        },
        || Ok(capture(&db.borrow(), &scope)), |_| Ok(()),
        |images| {
            let settings = serde_json::from_slice::<UsageSourceSettings>(&images[0].state.as_ref().unwrap().bytes).unwrap();
            save_source_settings(&mut db.borrow_mut(), &scope.project, &scope.host_id, "claude", 2, settings).map(|_| ())
        }, |_, _| Ok(())).unwrap();
    assert!(report.restored.is_ok());
    assert!(report.retention.unwrap().is_ok());
    assert!(open_local_backup(&root, &source, &scope).is_ok());
    let snapshot = open_safety_snapshot(&safety, &report.safety_point, &scope, &[target()]).unwrap();
    assert!(snapshot.preimages() == before);
    assert_eq!(load_source_settings(&db.borrow(), &scope.project, &scope.host_id, "claude").unwrap().settings, original);
}

#[test]
fn cc_data_26_restore_rejects_corrupt_before_write() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.path().canonicalize().unwrap();
    let scope = scope();
    let db = Connection::open_in_memory().unwrap();
    initialize_source_settings(&db).unwrap();
    let source = create_local_backup(&path, &scope, &[target()], "fixture", || Ok(capture(&db, &scope))).unwrap().unwrap();
    fs::write(path.join(format!("{}.local-backup", source.id())), b"corrupt").unwrap();
    let target_scope = DataManagementTarget { scope: scope.project.clone(), principal_id: scope.principal_id.clone(), target_host_id: scope.host_id.clone() };
    let result = restore_local_backup::<()>(&path, &path, &source, &target_scope, RetentionPolicy::default(),
        |_| panic!("validation reached"), || panic!("capture reached"), |_| panic!("persist reached"),
        |_| panic!("write reached"), |_, _| panic!("projection reached"));
    assert!(matches!(result, Err(SafetySnapshotError::Corrupt)));
    assert_eq!(fs::read_dir(path).unwrap().count(), 1);
}
