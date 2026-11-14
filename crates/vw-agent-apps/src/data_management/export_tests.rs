use super::{export::*, safety_snapshot::*};
use std::{fs, os::unix::fs::PermissionsExt};
use serde_json::json;
use vw_api_types::{agent_apps::data_management::DataCategory, id::{ProjectScope, ProjectId, TenantScope}};
fn scope() -> SafetySnapshotScope {
    SafetySnapshotScope {
        project: ProjectScope::new(TenantScope::new("fixture", "fixture").unwrap(), ProjectId::from("fixture")).unwrap(),
        principal_id: "fixture-user".into(), host_id: "fixture-host".into(),
    }
}
#[test]
fn cc_data_20_export_publishes_verified_portable_package() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.path().canonicalize().unwrap();
    let scope = scope();
    let receipt = export_portable_package(&path, &scope, &[DataCategory::Providers], |observed, category| {
        assert_eq!(observed, &scope);
        assert_eq!(category, DataCategory::Providers);
        Ok(vec![json!({"name":"Anonymous", "model":"fixture", "apiKey":"fixture-secret", "path":"/fixture/private"})])
    }).unwrap();
    let bytes = fs::read(path.join(format!("{}.shiju-package", receipt.id))).unwrap();
    let records = verify_portable_package(&bytes, &receipt.digest).unwrap();
    assert_eq!(records[0].items, vec![json!({"name":"Anonymous", "model":"fixture"})]);
    assert_eq!(receipt.excluded_fields, 2);
    assert_eq!(fs::read_dir(&path).unwrap().count(), 1);
    let mut corrupt = bytes;
    corrupt[0] ^= 1;
    assert!(verify_portable_package(&corrupt, &receipt.digest).is_err());
}
#[test]
fn cc_data_20_export_failure_leaves_no_visible_artifact() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = root.path().canonicalize().unwrap();
    assert!(export_portable_package(&path, &scope(), &[DataCategory::Rules], |_, _| Err(SafetySnapshotError::CaptureFailed)).is_err());
    assert_eq!(fs::read_dir(&path).unwrap().count(), 0);
    let directory = super::backup_create_files::open_root(&path).unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    assert!(super::export_files::publish(&directory, &id, b"fixture", |_| Err(SafetySnapshotError::Corrupt)).is_err());
    assert_eq!(fs::read_dir(&path).unwrap().count(), 0);
    super::export_files::publish(&directory, &id, b"first", |_| Ok(())).unwrap();
    assert!(super::export_files::publish(&directory, &id, b"second", |_| Ok(())).is_err());
    assert_eq!(fs::read(path.join(format!("{id}.shiju-package"))).unwrap(), b"first");
    assert_eq!(fs::read_dir(&path).unwrap().count(), 1);
}
