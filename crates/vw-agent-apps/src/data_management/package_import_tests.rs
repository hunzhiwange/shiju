use super::{export::export_portable_package, package_import::stage_portable_package,
    preview::{build_preview, Difference, PreviewCandidate}, safety_snapshot::*, source::*};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::{agent_apps::data_management::DataCategory,
    id::{ProjectId, ProjectScope, TenantScope}};

fn fixture() -> (tempfile::TempDir, Vec<u8>, [u8; 32], SourceScope) {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let scope = SafetySnapshotScope {
        project: ProjectScope::new(TenantScope::new("fixture", "fixture").unwrap(), ProjectId::from("fixture")).unwrap(),
        principal_id: "fixture-user".into(), host_id: "fixture-host".into(),
    };
    let categories = [DataCategory::Providers, DataCategory::Mcp, DataCategory::Skills,
        DataCategory::Rules, DataCategory::Profiles, DataCategory::Preferences, DataCategory::Usage];
    let receipt = export_portable_package(&root.path().canonicalize().unwrap(), &scope, &categories,
        |_, _| Ok(vec![json!({"name":"Anonymous", "apiKey":"excluded"})])).unwrap();
    let bytes = fs::read(root.path().join(format!("{}.shiju-package", receipt.id))).unwrap();
    let target = SourceScope::new(scope.project, "isolated-target".into(), scope.principal_id).unwrap();
    (root, bytes, receipt.digest, target)
}

#[test]
fn cc_data_21_package_round_trip_preserves_semantics() {
    let (_root, bytes, digest, target) = fixture();
    let staged = stage_portable_package(&target, &bytes, &digest).unwrap();
    assert_eq!(staged.summary(&target).unwrap().format, SourceFormat::PortablePackage);
    let categories = [SourceCategory::Providers, SourceCategory::Mcp, SourceCategory::Skills,
        SourceCategory::Rules, SourceCategory::Profiles, SourceCategory::Preferences, SourceCategory::Usage];
    assert_eq!(staged.records(&target).unwrap().len(), 7);
    let preview = build_preview(&target, &staged, &categories, "revision".into(), |_, records| {
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].fields, json!({"name":"Anonymous"}));
        Ok(records.iter().map(|record| PreviewCandidate {
            incoming: record.fields.clone(), existing: None, references_valid: true,
        }).collect())
    }).unwrap();
    assert_eq!(preview.entries().len(), 7);
    assert!(preview.entries().iter().all(|entry| entry.difference == Difference::Added));
    assert_eq!(preview.resolve(&target, &staged, "revision", &[]).unwrap().len(), 7);
    let other = SourceScope::new(target.project().clone(), "another-host".into(), "fixture-user".into()).unwrap();
    assert!(staged.records(&other).is_err());
}

#[test]
fn cc_data_21_package_rejects_hash_mismatch() {
    let (_root, bytes, mut digest, scope) = fixture();
    digest[0] ^= 1;
    assert!(matches!(stage_portable_package(&scope, &bytes, &digest), Err(SafetySnapshotError::Corrupt)));
}
