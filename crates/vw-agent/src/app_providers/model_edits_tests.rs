use crate::{app_providers::AppProviderStore, global_database::GlobalDatabase};
use std::path::Path;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderWrite, ProviderProtocol},
    id::TenantScope,
    model_catalog::{
        ModelCatalogDefaultChange, ModelCatalogEdit, ModelCatalogEditRequest, ModelCatalogIdentity,
    },
};

fn open(root: &Path) -> AppProviderStore {
    AppProviderStore::new(GlobalDatabase::for_home(root).unwrap()).unwrap()
}

fn seed(store: &AppProviderStore, tenant: &TenantScope) -> ModelCatalogIdentity {
    let deployment = ModelCatalogIdentity {
        target_host_id: "mac".into(),
        app_id: AppId::Codex,
        provider_id: "fixture".into(),
        endpoint: "https://fixture.test/v1".into(),
    };
    store
        .save(
            tenant,
            deployment.app_id,
            &deployment.provider_id,
            &AppProviderWrite {
                name: "Fixture".into(),
                protocol: ProviderProtocol::OpenaiResponses,
                api_key: "test-only-value".into(),
                base_url: deployment.endpoint.clone(),
                model: "configured".into(),
                request_options: None,
            },
        )
        .unwrap();
    deployment
}

fn select(id: &str) -> ModelCatalogEdit {
    ModelCatalogEdit::Upsert {
        model_id: id.into(),
        display_alias: None,
        metadata: Default::default(),
    }
}

#[test]
fn catalog_edits_provider_transaction_keeps_invalid_batches_out_of_sqlite() {
    let root = tempfile::tempdir().unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let other = TenantScope::new("platform", "other").unwrap();
    let store = open(root.path());
    let deployment = seed(&store, &tenant);
    seed(&store, &other);
    let initial = store.read_model_catalog(&tenant, &deployment).unwrap();
    let mut request = ModelCatalogEditRequest {
        deployment: deployment.clone(),
        expected_revision: initial.revision,
        edits: vec![select("saved")],
        default_change: None,
    };
    assert!(store.edit_model_catalog(&tenant, &request).unwrap().changed);
    drop(store);
    let store = open(root.path());
    let baseline = store.read_model_catalog(&tenant, &deployment).unwrap();
    assert_eq!(baseline.items[0].identity.model_id.as_ref(), "saved");
    assert!(store.read_model_catalog(&other, &deployment).unwrap().items.is_empty());
    request.expected_revision = baseline.revision.clone();
    assert_eq!(store.edit_model_catalog(&other, &request).unwrap_err(), "model catalog changed");
    request.edits =
        vec![ModelCatalogEdit::Remove { model_id: "saved".into() }, select("valid"), select(" ")];
    assert_eq!(store.edit_model_catalog(&tenant, &request).unwrap_err(), "invalid model ID");
    drop(store);
    assert_eq!(open(root.path()).read_model_catalog(&tenant, &deployment).unwrap(), baseline);
}

#[test]
fn catalog_edits_provider_transaction_rejects_competing_writes_at_the_same_revision() {
    let root = tempfile::tempdir().unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let store = open(root.path());
    let deployment = seed(&store, &tenant);
    let revision = store.read_model_catalog(&tenant, &deployment).unwrap().revision;
    let first = ModelCatalogEditRequest {
        deployment: deployment.clone(),
        expected_revision: revision,
        edits: vec![select("first")],
        default_change: None,
    };
    let second = ModelCatalogEditRequest { edits: vec![select("second")], ..first.clone() };
    let other = open(root.path());
    let barrier = std::sync::Barrier::new(2);
    let (one, two) = std::thread::scope(|scope| {
        let one = scope.spawn(|| {
            barrier.wait();
            store.edit_model_catalog(&tenant, &first)
        });
        barrier.wait();
        let two = other.edit_model_catalog(&tenant, &second);
        (one.join().unwrap(), two)
    });
    let winner = match (one, two) {
        (Ok(saved), Err(error)) => {
            assert!(saved.changed);
            assert_eq!(error, "model catalog changed");
            "first"
        }
        (Err(error), Ok(saved)) => {
            assert_eq!(error, "model catalog changed");
            assert!(saved.changed);
            "second"
        }
        results => panic!("expected one successful commit and one conflict, got {results:?}"),
    };
    let catalog = open(root.path()).read_model_catalog(&tenant, &deployment).unwrap();
    assert_eq!(catalog.items.len(), 1);
    assert_eq!(catalog.items[0].identity.model_id.as_ref(), winner);
}

#[test]
fn catalog_defaults_provider_transaction_reopens_without_cross_scope_changes() {
    let root = tempfile::tempdir().unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let other = TenantScope::new("platform", "other").unwrap();
    let store = open(root.path());
    let deployment = seed(&store, &tenant);
    seed(&store, &other);
    let original = store.read_model_catalog(&tenant, &deployment).unwrap();
    let request = ModelCatalogEditRequest {
        deployment: deployment.clone(),
        expected_revision: original.revision,
        edits: vec![ModelCatalogEdit::Upsert {
            model_id: "Vendor/Model:free".into(),
            display_alias: Some("展示名".into()),
            metadata: Default::default(),
        }],
        default_change: Some(ModelCatalogDefaultChange::Set {
            model_id: "Vendor/Model:free".into(),
        }),
    };
    let saved = store.edit_model_catalog(&tenant, &request).unwrap();
    assert_eq!(store.edit_model_catalog(&other, &request).unwrap_err(), "model catalog changed");
    drop(store);

    let store = open(root.path());
    let catalog = store.read_model_catalog(&tenant, &deployment).unwrap();
    assert_eq!(catalog.revision, saved.revision);
    assert_eq!(catalog.default_model_id, Some("Vendor/Model:free".into()));
    assert_eq!(catalog.items[0].identity.scope, tenant);
    assert_eq!(catalog.items[0].identity.deployment, deployment);
    assert_eq!(catalog.items[0].identity.model_id.as_ref(), "Vendor/Model:free");
    assert_eq!(catalog.items[0].display_alias.as_deref(), Some("展示名"));
    let other_catalog = store.read_model_catalog(&other, &deployment).unwrap();
    assert!(other_catalog.items.is_empty());
    assert!(other_catalog.default_model_id.is_none());
}
