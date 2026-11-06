use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{app_providers::grok::*, id::TenantScope};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-grok-token-with-at-least-32-chars";
struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    client: GatewayClient,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let private = root.path().join("private");
        std::fs::create_dir(&private).unwrap();
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::create_dir(root.path().join(".grok")).unwrap();
        std::fs::write(
            root.path().join(".grok/config.toml"),
            "# keep\n[ui]\ntheme = 'dark'\n[model.mine]\nmodel = 'user'\n[mcp_servers.fixture]\ncommand = 'fixture'\n",
        )
        .unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"grok","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let router = super::grok::router_with_policy(Some(policy.clone()))
            .layer(Extension(db.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            axum::serve(listener, Router::new().nest("/v1", router)).await.unwrap()
        });
        let client = GatewayClient::new(
            GatewayEndpoint::new("127.0.0.1", port)
                .with_tenant_scope(TenantScope::new("p", "a").unwrap()),
        )
        .unwrap();
        Self { root, db, client, server }
    }
    fn file(&self) -> PathBuf {
        self.root.path().join(".grok/config.toml")
    }
    fn request(&self, official: bool, activation: GrokActivation) -> GrokWrite {
        GrokWrite {
            target_host_id: "host".into(),
            profile: GrokProfile {
                name: "Fixture".into(),
                config: if official {
                    GrokConfig::Official {}
                } else {
                    GrokConfig::ThirdParty { toml: "[models]\ndefault = 'old'\n[model.old]\nmodel = 'remote'\nname = 'Fixture'\nbase_url = 'https://fixture.test'\napi_key = 'fake-secret'\napi_backend = 'responses'\ncontext_window = 1000\n".into() }
                },
            },
            activation,
        }
    }
    fn target(&self) -> GrokTarget {
        GrokTarget { target_host_id: "host".into() }
    }
    fn state(&self) -> Value {
        serde_json::from_slice(
            &std::fs::read(
                self.root.path().join("private/provider-live/aG9zdA/grokbuild.state.json"),
            )
            .unwrap(),
        )
        .unwrap()
    }
    fn record(&self, id: &str) -> Option<String> {
        GatewaySettingsStore::new(self.db.clone())
            .unwrap()
            .get(
                &TenantScope::new("p", "a").unwrap(),
                "app_providers.records",
                &format!("\"grokbuild\":{id}"),
            )
            .unwrap()
    }
}

#[tokio::test]
async fn cc04_grok_sdk_api_switch_edit_and_reapply_use_written_record() {
    let f = Fixture::new().await;
    let r = f
        .client
        .grok_provider_save("one", &f.request(false, GrokActivation::First), TOKEN)
        .await
        .unwrap();
    assert!(r.catalog_saved);
    assert_eq!(r.live, GrokLiveResult::Applied);
    let mut edit = f.request(false, GrokActivation::IfCurrent);
    if let GrokConfig::ThirdParty { toml } = &mut edit.profile.config {
        *toml = toml.replace("old", "new");
    }
    let r = f.client.grok_provider_save("one", &edit, TOKEN).await.unwrap();
    assert_eq!(r.live, GrokLiveResult::Applied);
    let text = std::fs::read_to_string(f.file()).unwrap();
    assert!(!text.contains("[model.old]"), "old table remains after editing current row");
    assert!(text.contains("model = 'user'"));
    assert!(text.contains("[mcp_servers.fixture]"));
    assert_eq!(f.state()["pointer"]["id"], "one");
    // Save a renamed catalog without applying; reapply must still retire the committed table.
    let mut draft = edit.clone();
    draft.activation = GrokActivation::CatalogOnly;
    if let GrokConfig::ThirdParty { toml } = &mut draft.profile.config {
        *toml = toml.replace("new", "final");
    }
    assert_eq!(
        f.client.grok_provider_save("one", &draft, TOKEN).await.unwrap().live,
        GrokLiveResult::NotAttempted
    );
    assert_eq!(
        f.client.grok_provider_reapply("one", &f.target(), TOKEN).await.unwrap().live,
        GrokLiveResult::Applied
    );
    let text = std::fs::read_to_string(f.file()).unwrap();
    assert!(!text.contains("[model.new]"));
    f.client
        .grok_provider_save("official", &f.request(true, GrokActivation::CatalogOnly), TOKEN)
        .await
        .unwrap();
    assert_eq!(
        f.client.grok_provider_switch("official", &f.target(), TOKEN).await.unwrap().live,
        GrokLiveResult::Applied
    );
    let text = std::fs::read_to_string(f.file()).unwrap();
    assert!(!text.contains("[model.final]"));
    assert!(!text.contains("fake-secret"));
    assert_eq!(f.state()["pointer"]["id"], "official");
}
#[tokio::test]
async fn cc04_grok_denial_has_no_file_or_record_side_effects() {
    let f = Fixture::new().await;
    let before = std::fs::read(f.file()).unwrap();
    assert!(
        f.client
            .grok_provider_save("denied", &f.request(false, GrokActivation::Switch), "bad-token")
            .await
            .is_err()
    );
    assert!(f.record("denied").is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    let path = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(path, policy.to_string()).unwrap();
    assert!(
        f.client
            .grok_provider_save("denied", &f.request(false, GrokActivation::Switch), TOKEN)
            .await
            .is_err()
    );
    assert!(f.record("denied").is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
}
#[tokio::test]
async fn cc04_grok_activation_scope_and_invalid_live_are_checked_before_save() {
    let f = Fixture::new().await;
    f.client
        .grok_provider_save("one", &f.request(false, GrokActivation::First), TOKEN)
        .await
        .unwrap();
    let before = std::fs::read(f.file()).unwrap();
    for activation in [GrokActivation::First, GrokActivation::IfCurrent] {
        assert_eq!(
            f.client
                .grok_provider_save("two", &f.request(true, activation), TOKEN)
                .await
                .unwrap()
                .live,
            GrokLiveResult::NotAttempted
        );
        assert_eq!(std::fs::read(f.file()).unwrap(), before);
    }
    let mut bad = f.request(true, GrokActivation::Switch);
    bad.target_host_id = "other".into();
    assert!(f.client.grok_provider_save("foreign", &bad, TOKEN).await.is_err());
    assert!(f.record("foreign").is_none());
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    assert_eq!(
        store
            .list(&TenantScope::new("p", "a").unwrap(), vw_api_types::agent_apps::AppId::GrokBuild)
            .unwrap()
            .len(),
        2
    );
    assert!(
        store
            .list(
                &TenantScope::new("p", "other").unwrap(),
                vw_api_types::agent_apps::AppId::GrokBuild
            )
            .unwrap()
            .is_empty()
    );
    std::fs::write(f.file(), b"invalid TOML").unwrap();
    assert!(
        f.client
            .grok_provider_save("invalid", &f.request(false, GrokActivation::Switch), TOKEN)
            .await
            .is_err()
    );
    assert!(f.record("invalid").is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), b"invalid TOML");
}
#[tokio::test]
async fn cc04_grok_managed_rows_and_missing_write_records_are_refused() {
    let f = Fixture::new().await;
    f.client
        .grok_provider_save("one", &f.request(false, GrokActivation::CatalogOnly), TOKEN)
        .await
        .unwrap();
    let before = std::fs::read(f.file()).unwrap();
    let mut row: Value = serde_json::from_str(&f.record("one").unwrap()).unwrap();
    row["meta"] = json!({"authBinding":{"accountId":"fake-unresolved"}});
    GatewaySettingsStore::new(f.db.clone())
        .unwrap()
        .upsert(
            &TenantScope::new("p", "a").unwrap(),
            "app_providers.records",
            "\"grokbuild\":one",
            &row.to_string(),
        )
        .unwrap();
    assert!(f.client.grok_provider_switch("one", &f.target(), TOKEN).await.is_err());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    f.client
        .grok_provider_save("two", &f.request(false, GrokActivation::Switch), TOKEN)
        .await
        .unwrap();
    let before = std::fs::read(f.file()).unwrap();
    let path = f.root.path().join("private/provider-live/aG9zdA/grokbuild.state.json");
    let mut state = f.state();
    state["mode"]["contract"] = Value::Null;
    std::fs::write(path, state.to_string()).unwrap();
    assert!(
        f.client
            .grok_provider_save("two", &f.request(true, GrokActivation::IfCurrent), TOKEN)
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert_eq!(
        serde_json::from_str::<Value>(&f.record("two").unwrap()).unwrap()["grok"]["config"]["kind"],
        "third_party"
    );
}
