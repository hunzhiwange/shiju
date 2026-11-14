//! macOS acceptance prerequisites using anonymous copies and the real provider owner.
//! These do not claim seven-category or SDK/Iced acceptance.
use crate::{app_providers::AppProviderStore, global_database::GlobalDatabase};
use serde_json::json;
use std::{cell::Cell, fs, os::unix::fs::PermissionsExt};
use vw_agent_apps::data_management::{
    export::{export_portable_package, verify_portable_package},
    preview::Decision,
    safety_snapshot::SafetySnapshotScope,
    source::{SourceScope, read_selected_source},
};
use vw_api_types::{
    agent_apps::{AppId, data_management::DataCategory},
    id::{ProjectScope, TenantScope},
};

struct Fixture {
    root: tempfile::TempDir,
    source_path: std::path::PathBuf,
    bytes: Vec<u8>,
    scope: SourceScope,
    store: AppProviderStore,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let source_path = root.path().canonicalize().unwrap().join("匿名迁入副本.json");
        let bytes = json!({"version":2,"claude":{"current":"fixture","providers":{
            "fixture":{"name":"Anonymous","settingsConfig":{"env":{"ANTHROPIC_MODEL":"fixture-model"}}}
        }}}).to_string().into_bytes();
        fs::write(&source_path, &bytes).unwrap();
        fs::set_permissions(&source_path, fs::Permissions::from_mode(0o600)).unwrap();
        let tenant = TenantScope::new("acceptance", "selected").unwrap();
        let scope = SourceScope::new(
            ProjectScope::new(tenant, "project".into()).unwrap(),
            "host".into(), "principal".into(),
        ).unwrap();
        let db = GlobalDatabase::new(root.path().join("isolated.sqlite")).unwrap();
        let store = AppProviderStore::new(db).unwrap();
        Self { root, source_path, bytes, scope, store }
    }
}

#[test]
fn cc_data_45_macos_migration_round_trip_is_idempotent() {
    let f = Fixture::new();
    let root = f.root.path().canonicalize().unwrap();
    let source = read_selected_source(&f.scope, &root, &f.source_path).unwrap();
    let before = fs::metadata(&f.source_path).unwrap().modified().unwrap();
    let preview = f.store.preview_cc_providers(&f.scope, &source).unwrap();
    assert_eq!(f.store.import_cc_providers(&f.scope, &source, &preview, &[], &root).unwrap().0, 1);
    let rows = f.store.list(f.scope.project().tenant_scope(), AppId::Claude).unwrap();
    let expected = serde_json::to_value(&rows).unwrap();
    // A fresh preview exercises semantic re-import, rather than only request replay.
    let preview = f.store.preview_cc_providers(&f.scope, &source).unwrap();
    let decisions: Vec<_> = preview.entries().iter().map(|entry| (entry.id, Decision::Skip)).collect();
    assert_eq!(f.store.import_cc_providers(&f.scope, &source, &preview, &decisions, &root).unwrap().0, 0);
    assert_eq!(serde_json::to_value(f.store.list(f.scope.project().tenant_scope(), AppId::Claude).unwrap()).unwrap(), expected);
    let receipt = export_portable_package(&root, &SafetySnapshotScope {
        project: f.scope.project().clone(), host_id: "host".into(), principal_id: "principal".into(),
    }, &[DataCategory::Providers], |_, _| Ok(expected.as_array().unwrap().clone())).unwrap();
    let bytes = fs::read(root.join(format!("{}.shiju-package", receipt.id))).unwrap();
    let records = verify_portable_package(&bytes, &receipt.digest).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].items.len(), 1);
    assert_eq!(fs::read(&f.source_path).unwrap(), f.bytes);
    assert_eq!(fs::metadata(&f.source_path).unwrap().modified().unwrap(), before);
    let foreign = TenantScope::new("acceptance", "foreign").unwrap();
    assert!(f.store.list(&foreign, AppId::Claude).unwrap().is_empty());
}

#[test]
fn cc_data_45_macos_late_failure_restores_target() {
    let f = Fixture::new();
    let root = f.root.path().canonicalize().unwrap();
    let source = read_selected_source(&f.scope, &root, &f.source_path).unwrap();
    let preview = f.store.preview_cc_providers(&f.scope, &source).unwrap();
    let reached = Cell::new(false);
    let result = f.store.import_cc_providers_recorded(&f.scope, &source, &preview, &[], &root,
        |_, count, receipt| {
            assert_eq!(count, 1);
            assert!(receipt.is_some());
            reached.set(true);
            Err("designated terminal persistence failure".into())
        });
    assert!(reached.get());
    assert!(result.is_err());
    assert!(f.store.list(f.scope.project().tenant_scope(), AppId::Claude).unwrap().is_empty());
    assert_eq!(fs::read(&f.source_path).unwrap(), f.bytes);
    // A fresh owner attempt can commit after the rolled-back transaction.
    assert_eq!(f.store.import_cc_providers(&f.scope, &source, &preview, &[], &root).unwrap().0, 1);
}
