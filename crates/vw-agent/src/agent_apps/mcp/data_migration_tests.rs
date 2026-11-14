use super::McpCatalogStore;
use crate::global_database::GlobalDatabase;
use serde_json::json;
use vw_agent_apps::data_management::{
    preview::Decision,
    source::{SourceScope, read_selected_source},
};
use vw_api_types::{
    id::{ProjectScope, TenantScope},
    mcp_management::{McpCatalogScope, McpFailureCode},
};

#[test]
fn cc_data_10_owner_add_skip_replace_and_repeat_without_duplicate_rows() {
    let home = tempfile::tempdir().unwrap();
    let sources = tempfile::tempdir().unwrap();
    let snapshots = tempfile::tempdir().unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(snapshots.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let project = ProjectScope::new(
        TenantScope::new("fixture-platform", "fixture-company").unwrap(),
        "fixture-project".into(),
    )
    .unwrap();
    let scope =
        SourceScope::new(project.clone(), "fixture-host".into(), "fixture-user".into()).unwrap();
    let catalog_scope = McpCatalogScope::Project { scope: project };
    let store = McpCatalogStore::new(GlobalDatabase::for_home(home.path()).unwrap()).unwrap();
    let path = sources.path().join("source.json");
    let write_source = |name: &str| {
        std::fs::write(&path, serde_json::to_vec(&json!({"version":2,"mcp":{"servers":{"fixture":{"id":"fixture","name":name,"server":{"command":"fixture-command"},"apps":{}}}}})).unwrap()).unwrap()
    };
    write_source("Initial");
    let source = read_selected_source(&scope, sources.path(), &path).unwrap();
    let prepared = store.preview_data_migration(&scope, &source, &[]).unwrap();
    let result =
        store.commit_data_migration(&scope, &source, &prepared, &[], snapshots.path()).unwrap();
    assert_eq!(result.outcome, Ok(1));
    assert!(result.snapshot.is_some());
    let prepared = store.preview_data_migration(&scope, &source, &[]).unwrap();
    assert_eq!(
        store
            .commit_data_migration(&scope, &source, &prepared, &[], snapshots.path())
            .unwrap()
            .outcome,
        Ok(0)
    );
    write_source("Changed");
    let source = read_selected_source(&scope, sources.path(), &path).unwrap();
    let prepared = store.preview_data_migration(&scope, &source, &[]).unwrap();
    let id = prepared.preview().entries()[0].id;
    assert_eq!(
        store
            .commit_data_migration(
                &scope,
                &source,
                &prepared,
                &[(id, Decision::Skip)],
                snapshots.path()
            )
            .unwrap()
            .outcome,
        Ok(0)
    );
    assert_eq!(store.list(&catalog_scope).unwrap()[0].name, "Initial");
    assert_eq!(
        store
            .commit_data_migration(
                &scope,
                &source,
                &prepared,
                &[(id, Decision::Replace)],
                snapshots.path()
            )
            .unwrap()
            .outcome,
        Ok(1)
    );
    let records = store.list(&catalog_scope).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].name, "Changed");
    assert_eq!(records[0].record.revision, 2);
    let error = store
        .commit_data_migration(
            &scope,
            &source,
            &prepared,
            &[(id, Decision::Replace)],
            snapshots.path(),
        )
        .unwrap()
        .outcome
        .unwrap_err();
    // A stale CAS is retained with its snapshot receipt as an outcome.
    assert_eq!(error, McpFailureCode::StaleRevision);
}
