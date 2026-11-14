use std::collections::BTreeMap;
use serde_json::json;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use super::{providers::*, preview::{Decision, PreviewError}, source::*};
fn fixture() -> (SourceScope, SourceSnapshot) {
    let scope = SourceScope::new(ProjectScope::new(TenantScope::new("fixture-platform", "fixture-company").unwrap(), ProjectId::from("fixture-project")).unwrap(), "fixture-host".into(), "fixture-user".into()).unwrap();
    let source = SourceSnapshot {
        scope: scope.clone(),
        summary: SourceSummary { format: SourceFormat::Sqlite, version: 24, main_sha256: [9;32], wal_sha256: None, counts: BTreeMap::new() },
        records: vec![SourceRecord { category: SourceCategory::Providers, origin: "providers".into(), fields: json!({"id":"fixture","app_type":"claude","name":"Anonymous","settings_config":{"env":{"ANTHROPIC_MODEL":"fixture-model"}},"is_current":1}) }],
    };
    (scope, source)
}
#[test]
fn cc_data_09_providers_import_is_idempotent() {
    let (scope, source) = fixture();
    let preview = prepare(&scope, &source, &[]).unwrap();
    let selected = preview.resolve(&scope, &source, &[], &[]).unwrap();
    assert_eq!(selected.len(),1);
    assert_eq!(selected[0].value["claude"]["model"], "fixture-model");
    assert!(selected[0].value.get("is_current").is_none());
    let existing = vec![ProviderRow { app: selected[0].app, id: selected[0].id.clone(), value: selected[0].value.clone() }];
    let next = prepare(&scope, &source, &existing).unwrap();
    assert!(next.resolve(&scope, &source, &existing, &[]).unwrap().is_empty());
}
#[test]
fn cc_data_09_providers_skip_preserves_existing() {
    let (scope, source) = fixture();
    let first = prepare(&scope, &source, &[]).unwrap();
    let selected = first.resolve(&scope, &source, &[], &[]).unwrap();
    let mut value = selected[0].value.clone();
    value["name"] = json!("Existing");
    let existing = vec![ProviderRow { app: selected[0].app, id: selected[0].id.clone(), value }];
    let preview = prepare(&scope, &source, &existing).unwrap();
    assert!(preview.resolve(&scope, &source, &existing, &[(preview.entries()[0].id,Decision::Skip)]).unwrap().is_empty());
    assert_eq!(existing[0].value["name"],"Existing");
}
#[test]
fn cc_data_09_providers_rejects_stale_scope_and_invalid_batch() {
    let (scope, mut source) = fixture();
    let preview = prepare(&scope, &source, &[]).unwrap();
    let foreign = SourceScope::new(scope.project().clone(), "another-host".into(), "fixture-user".into()).unwrap();
    assert!(matches!(preview.resolve(&foreign, &source, &[], &[]),Err(PreviewError::Forbidden)));
    source.records.push(SourceRecord { category: SourceCategory::Providers, origin:"providers".into(), fields:json!({"id":"invalid space"}) });
    assert!(prepare(&scope, &source, &[]).is_err());
}
#[test]
fn cc_data_09_providers_does_not_copy_source_credentials() {
    let (scope, mut source) = fixture();
    source.records[0].fields["settings_config"]["env"]["ANTHROPIC_BASE_URL"] = json!("https://fixture.invalid");
    source.records[0].fields["settings_config"]["env"]["ANTHROPIC_AUTH_TOKEN"] = json!("anonymous-source-secret");
    // No account-owned target credential exists: reject the whole preparation.
    assert!(prepare(&scope, &source, &[]).is_err());
}
