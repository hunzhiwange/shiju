use super::{AppProviderStore, GlobalDatabase, TenantScope};
use vw_agent_apps::{
    high_availability::queue::{Queue, QueueError},
    providers::failover::ProviderAuth,
};
use vw_api_types::{
    agent_apps::AppId::{Claude, Codex, Pi},
    app_providers::{AppProviderWrite, ProviderProtocol, codex_catalog::CodexProfile},
    id::ProjectScope,
};

#[test]
fn ha_queue_rejects_ineligible_members_from_saved_provider_snapshot() {
    let home = tempfile::tempdir().unwrap();
    let store = AppProviderStore::new(GlobalDatabase::for_home(home.path()).unwrap()).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let other = TenantScope::new("platform", "other-company").unwrap();
    let scope = ProjectScope::new(tenant.clone(), "project".into()).unwrap();
    let official = CodexProfile {
        name: "Same display name".into(),
        config: "model = \"fixture-model\"".into(),
        api_key: None,
        preserve_login: false,
    };
    let custom = CodexProfile {
        config: r#"
model = "fixture-model"
model_provider = "vendor"

[model_providers.vendor]
base_url = "https://vendor.example.test/v1"
wire_api = "responses"
"#
        .into(),
        api_key: Some("test-provider-key".into()),
        ..official.clone()
    };
    let unknown = CodexProfile { api_key: None, ..custom.clone() };
    store.codex_save(&tenant, "official", &official, None, None).unwrap();
    store.codex_save(&tenant, "custom", &custom, None, None).unwrap();
    store.codex_save(&tenant, "unknown", &unknown, None, None).unwrap();
    store.codex_save(&other, "other-tenant", &custom, None, None).unwrap();

    let mut legacy = AppProviderWrite {
        name: "Same display name".into(),
        protocol: ProviderProtocol::OpenaiResponses,
        api_key: "test-legacy-key".into(),
        base_url: "https://legacy.example.test/v1".into(),
        model: "fixture-model".into(),
        request_options: None,
    };
    store.save(&tenant, Codex, "legacy", &legacy).unwrap();
    let placeholder = AppProviderWrite { api_key: "PROXY_MANAGED".into(), ..legacy.clone() };
    store.save(&tenant, Codex, "placeholder", &placeholder).unwrap();
    legacy.protocol = ProviderProtocol::Anthropic;
    store.save(&tenant, Claude, "other-app", &legacy).unwrap();

    let providers = store.failover_providers(&scope, Codex).unwrap();
    assert_eq!(providers.len(), 5);
    for (id, auth) in [
        ("official", ProviderAuth::OfficialAccount),
        ("custom", ProviderAuth::ThirdParty),
        ("legacy", ProviderAuth::ThirdParty),
        ("unknown", ProviderAuth::Unknown),
        ("placeholder", ProviderAuth::Unknown),
    ] {
        let provider = providers.iter().find(|entry| entry.provider.id == id).unwrap();
        assert_eq!(provider.auth, auth);
        assert_eq!(provider.provider, store.get_summary(&tenant, Codex, id).unwrap());
    }
    let debug = format!("{providers:?}");
    assert!(!debug.contains("test-provider-key"));
    assert!(!debug.contains("test-legacy-key"));
    assert!(!debug.contains("example.test"));
    assert!(store.failover_providers(&scope, Pi).is_err());

    let mut queue = Queue::new(Codex, vec![]);
    assert_eq!(queue.add("official", &providers), Err(QueueError::IneligibleProvider));
    assert_eq!(queue.add("unknown", &providers), Err(QueueError::IneligibleProvider));
    assert_eq!(queue.add("placeholder", &providers), Err(QueueError::IneligibleProvider));
    assert_eq!(queue.add("other-app", &providers), Err(QueueError::ProviderNotFound));
    assert_eq!(queue.add("other-tenant", &providers), Err(QueueError::ProviderNotFound));
    assert_eq!(queue.add("custom", &providers), Ok(true));
    assert_eq!(queue.add("legacy", &providers), Ok(true));
    assert_eq!(queue.members(), ["custom", "legacy"]);
}
