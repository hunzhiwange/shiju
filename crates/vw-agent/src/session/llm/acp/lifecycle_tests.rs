use super::*;
use vw_api_types::acp_model_source::AcpModelSource;

#[test]
fn acp_model_source_session_controls_failed_commit_preserves_recovery_anchor() {
    let root = tempfile::tempdir().unwrap();
    let database = GlobalDatabase::for_home(root.path()).unwrap();
    let store = Store {
        store: GatewaySettingsStore::new(database).unwrap(),
        tenant: TenantScope::default(),
        key: session_identity("session", "/workspace"),
    };
    let previous = Selection {
        agent: "codex".into(),
        model: AcpSessionModelSelection {
            source: AcpModelSource::SessionModel,
            provider_id: Some("selected-provider".into()),
            model: Some("nested/model".into()),
        },
        generation: 0,
        record_id: "previous-anchor".into(),
    };
    store.commit(&previous).unwrap();
    // A failed replacement has no authority to publish a new selection.
    let failed =
        Store { store: store.store.clone(), tenant: store.tenant.clone(), key: String::new() };
    let mut replacement = previous.clone();
    replacement.record_id = "uncommitted-anchor".into();
    replacement.model.model = Some("replacement".into());
    assert!(failed.commit(&replacement).is_err());
    let recovered = store.read().unwrap().unwrap();
    assert_eq!(recovered.record_id, "previous-anchor");
    assert_eq!(recovered.model, previous.model);
    let other = Store {
        store: store.store.clone(),
        tenant: TenantScope::new("platform", "other").unwrap(),
        key: store.key.clone(),
    };
    assert!(other.read().unwrap().is_none());
    let raw = store.store.get(&store.tenant, NAMESPACE, &store.key).unwrap().unwrap();
    assert!(!raw.contains("token"));
}
