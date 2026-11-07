use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{app_providers::kimi::*, id::TenantScope};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-kimi-token-with-at-least-32-chars";
struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    port: u16,
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
        std::fs::create_dir_all(root.path().join(".kimi-code")).unwrap();
        std::fs::write(
            root.path().join(".kimi-code/config.toml"),
            "# keep\nfuture = true\ndefault_model = 'native'\n[providers.other]\ntype = 'openai'\napi_key = 'fake-native'\n[models.native]\nprovider = 'other'\nmodel = 'native'\nmax_context_size = 32000\n",
        )
        .unwrap();
        std::fs::write(root.path().join(".kimi-code/other.yml"), b"defaults-fixture").unwrap();
        std::fs::write(root.path().join(".kimi-code/agent.db"), b"login-fixture").unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"kimi","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let router = super::kimi::router_with_policy(Some(policy.clone()))
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
        Self { root, db, port, client, server }
    }
    fn file(&self) -> PathBuf {
        self.root.path().join(".kimi-code/config.toml")
    }
    fn request(&self, activation: KimiActivation) -> KimiWrite {
        KimiWrite {
            target_host_id: "host".into(),
            profile: KimiProfile {
                name: "Fixture".into(),
                declaration: json!({"provider":{"type":"openai","api_key":"fake-secret"},"models":{"alias":{"model":"fixture","max_context_size":32000}}}),
            },
            activation,
        }
    }
    fn target(&self) -> KimiTarget {
        KimiTarget { target_host_id: "host".into() }
    }
    fn record(&self, id: &str) -> Option<Value> {
        crate::app_providers::AppProviderStore::new(self.db.clone())
            .unwrap()
            .kimi_record(&TenantScope::new("p", "a").unwrap(), id)
            .unwrap()
    }
    fn bytes(&self) -> Vec<u8> {
        assert_eq!(
            std::fs::read(self.root.path().join(".kimi-code/other.yml")).unwrap(),
            b"defaults-fixture"
        );
        assert_eq!(
            std::fs::read(self.root.path().join(".kimi-code/agent.db")).unwrap(),
            b"login-fixture"
        );
        std::fs::read(self.file()).unwrap()
    }
}
#[tokio::test]
async fn cc04_kimi_sdk_api_member_edit_remove_and_catalog_delete() {
    let f = Fixture::new().await;
    assert_eq!(
        f.client
            .kimi_provider_save("one", &f.request(KimiActivation::Add), TOKEN)
            .await
            .unwrap()
            .live,
        KimiLiveResult::Applied
    );
    assert_eq!(
        f.client.kimi_provider_remove_live("one", &f.target(), TOKEN).await.unwrap().live,
        KimiLiveResult::Applied
    );
    assert_eq!(f.client.kimi_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    assert_eq!(
        f.client.kimi_provider_add("one", &f.target(), TOKEN).await.unwrap().live,
        KimiLiveResult::Applied
    );
    let mut request = f.request(KimiActivation::IfMember);
    request.profile.declaration["provider"]["api_key"] = json!("fake-edited");
    f.client.kimi_provider_save("one", &request, TOKEN).await.unwrap();
    assert!(String::from_utf8_lossy(&f.bytes()).contains("fake-edited"));
    let state = f.client.kimi_state(&f.target(), TOKEN).await.unwrap();
    assert_eq!(state.members, vec!["one", "other"]);
    assert!(String::from_utf8_lossy(&f.bytes()).contains("# keep\nfuture = true"));
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    assert!(
        store.delete(&tenant, vw_api_types::agent_apps::AppId::Kimi, "one", f.root.path()).is_err()
    );
    assert_eq!(
        f.client.kimi_provider_remove_live("one", &f.target(), TOKEN).await.unwrap().live,
        KimiLiveResult::Applied
    );
    assert!(f.record("one").is_some());
    assert_eq!(f.client.kimi_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    store.delete(&tenant, vw_api_types::agent_apps::AppId::Kimi, "one", f.root.path()).unwrap();
    assert!(f.record("one").is_none());
    let state: Value = serde_json::from_slice(
        &std::fs::read(f.root.path().join("private/provider-live/aG9zdA/kimi.state.json")).unwrap(),
    )
    .unwrap();
    assert!(state["pointer"].is_null());
}
#[tokio::test]
async fn cc04_kimi_import_is_read_only_and_does_not_overwrite_catalog() {
    let f = Fixture::new().await;
    let before = f.bytes();
    let mut permissions = std::fs::metadata(f.file()).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(f.file(), permissions).unwrap();
    assert_eq!(f.client.kimi_import(&f.target(), TOKEN).await.unwrap().imported, 1);
    assert_eq!(f.client.kimi_import(&f.target(), TOKEN).await.unwrap().imported, 0);
    assert_eq!(f.bytes(), before);
    assert!(f.record("other").is_some());
    assert!(!f.root.path().join("private/provider-live").exists());
    f.client
        .kimi_provider_save("draft", &f.request(KimiActivation::CatalogOnly), TOKEN)
        .await
        .unwrap();
    assert_eq!(f.bytes(), before);
}
#[tokio::test]
async fn cc04_kimi_permission_tenant_and_host_denials_leave_no_changes() {
    let f = Fixture::new().await;
    let before = f.bytes();
    assert!(
        f.client
            .kimi_provider_save("one", &f.request(KimiActivation::Add), "bad-token")
            .await
            .is_err()
    );
    let mut bad = f.request(KimiActivation::Add);
    bad.target_host_id = "foreign".into();
    assert!(f.client.kimi_provider_save("one", &bad, TOKEN).await.is_err());
    let foreign = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", f.port)
            .with_tenant_scope(TenantScope::new("p", "b").unwrap()),
    )
    .unwrap();
    assert!(
        foreign.kimi_provider_save("one", &f.request(KimiActivation::Add), TOKEN).await.is_err()
    );
    let path = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(path, policy.to_string()).unwrap();
    assert!(
        f.client.kimi_provider_save("one", &f.request(KimiActivation::Add), TOKEN).await.is_err()
    );
    assert_eq!(f.client.kimi_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    assert_eq!(f.bytes(), before);
    assert!(f.record("one").is_none());
    assert!(!f.root.path().join("private/provider-live").exists());
}
#[tokio::test]
async fn cc04_kimi_first_activation_and_reference_denials_are_atomic() {
    let f = Fixture::new().await;
    std::fs::write(f.file(), "# first\n[thinking]\neffort='high'\n").unwrap();
    let before = f.bytes();
    let mut bad = f.request(KimiActivation::Add);
    bad.profile.declaration["models"]["alias"]["provider"] = json!("missing");
    assert!(f.client.kimi_provider_save("one", &bad, TOKEN).await.is_err());
    assert!(f.record("one").is_none());
    assert_eq!(f.bytes(), before);
    assert_eq!(
        f.client
            .kimi_provider_save("one", &f.request(KimiActivation::Add), TOKEN)
            .await
            .unwrap()
            .live,
        KimiLiveResult::Applied
    );
    let doc: Value = toml::from_str(std::str::from_utf8(&f.bytes()).unwrap()).unwrap();
    assert_eq!(doc["default_model"], "alias");
    assert_eq!(doc["models"]["alias"]["provider"], "one");
    let record = f.record("one");
    let bytes = f.bytes();
    assert!(f.client.kimi_provider_remove_live("one", &f.target(), TOKEN).await.is_err());
    assert_eq!(f.bytes(), bytes);
    assert_eq!(f.record("one"), record);
}
#[tokio::test]
async fn cc04_kimi_existing_official_connection_is_consumed_and_never_forged() {
    let f = Fixture::new().await;
    let native = "# official\ndefault_model='native'\n[providers.'managed:kimi-code']\ntype='kimi'\nbase_url='https://api.kimi.com/coding/v1'\n[providers.'managed:kimi-code'.oauth]\nkey='oauth/kimi-code'\nstorage='file'\n[models.native]\nprovider='managed:kimi-code'\nmodel='native'\nmax_context_size=32000\n";
    std::fs::write(f.file(), native).unwrap();
    assert_eq!(f.client.kimi_import(&f.target(), TOKEN).await.unwrap().imported, 1);
    let mut malformed = f.request(KimiActivation::IfMember);
    malformed.profile.declaration = json!("invalid");
    assert!(f.client.kimi_provider_save("managed:kimi-code", &malformed, TOKEN).await.is_err());
    assert_eq!(f.bytes(), native.as_bytes());
    let mut request = f.request(KimiActivation::IfMember);
    request.profile.declaration["models"] =
        json!({"native":{"model":"edited","max_context_size":64000}});
    assert_eq!(
        f.client.kimi_provider_save("managed:kimi-code", &request, TOKEN).await.unwrap().live,
        KimiLiveResult::Applied
    );
    let doc: Value = toml::from_str(std::str::from_utf8(&f.bytes()).unwrap()).unwrap();
    assert_eq!(doc["providers"]["managed:kimi-code"]["oauth"]["key"], "oauth/kimi-code");
    assert_eq!(doc["providers"]["managed:kimi-code"]["base_url"], "https://api.kimi.com/coding/v1");
    assert_eq!(
        f.record("managed:kimi-code").unwrap()["kimi"]["declaration"]["provider"],
        doc["providers"]["managed:kimi-code"]
    );
    let bytes = f.bytes();
    let mut forged = f.request(KimiActivation::CatalogOnly);
    forged.profile.declaration["provider"] = doc["providers"]["managed:kimi-code"].clone();
    assert!(f.client.kimi_provider_save("forged", &forged, TOKEN).await.is_err());
    assert!(f.record("forged").is_none());
    assert_eq!(bytes, f.bytes());
    assert!(!f.root.path().join(".kimi-code/credentials").exists());
}
#[tokio::test]
async fn cc04_kimi_editor_is_checked_by_server_and_import_never_overwrites() {
    let f = Fixture::new().await;
    let before = f.bytes();
    let mut body = KimiEditorWrite {
        target_host_id: "host".into(),
        name: "Fixture".into(),
        toml: "default_model='changed'\n[providers.one]\n".into(),
        activation: KimiActivation::Add,
    };
    assert!(f.client.kimi_provider_editor_save("one", &body, TOKEN).await.is_err());
    assert_eq!(f.bytes(), before);
    assert!(f.record("one").is_none());
    let req = f.request(KimiActivation::Add);
    body.toml=toml::to_string(&json!({"providers":{"one":req.profile.declaration["provider"]},"models":req.profile.declaration["models"]})).unwrap();
    assert_eq!(
        f.client.kimi_provider_editor_save("one", &body, TOKEN).await.unwrap().live,
        KimiLiveResult::Applied
    );
    assert!(String::from_utf8_lossy(&f.bytes()).contains("fake-secret"));
    let record = f.record("one");
    let bytes = f.bytes();
    f.client.kimi_import(&f.target(), TOKEN).await.unwrap();
    assert_eq!(f.record("one"), record);
    assert_eq!(f.bytes(), bytes);
}

#[tokio::test]
async fn cc04_ui_kimi_form_roundtrip_preserves_credentials_and_other_nodes() {
    use vw_api_types::app_providers::{basic::ProviderBasic, create::ProviderSecretWrite};
    let f = Fixture::new().await;
    let mut seed = f.request(KimiActivation::Add);
    seed.profile.declaration["provider"]["base_url"] = json!("https://fixture.example.test/v1");
    seed.profile.declaration["provider"]["custom_headers"] = json!({"X-Test":"fake-header"});
    f.client.kimi_provider_save("one", &seed, TOKEN).await.unwrap();
    let view = f.client.kimi_form_read("one", &f.target(), TOKEN).await.unwrap();
    let config = view.config.unwrap();
    assert!(!config.declaration.contains("fake-secret"));
    assert!(!config.declaration.contains("fake-header"));
    let mut node: Value = toml::from_str(&config.declaration).unwrap();
    node["models"]["alias"]["display_name"] = json!("中文动态模型");
    let body = KimiFormWrite {
        target_host_id: "host".into(),
        fields: ProviderBasic {
            name: "中文供应商".into(),
            notes: Some("中文备注".into()),
            category: None,
            website_url: None,
        },
        declaration: toml::to_string(&node).unwrap(),
        api_key: ProviderSecretWrite::Keep,
        activation: KimiActivation::IfMember,
        revision: Some(config.revision),
    };
    assert_eq!(
        f.client.kimi_form_update("one", &body, TOKEN).await.unwrap().live,
        KimiLiveResult::Applied
    );
    let native: Value = toml::from_str(std::str::from_utf8(&f.bytes()).unwrap()).unwrap();
    assert_eq!(native["providers"]["one"]["api_key"], "fake-secret");
    assert_eq!(native["providers"]["one"]["custom_headers"]["X-Test"], "fake-header");
    assert_eq!(native["providers"]["other"]["api_key"], "fake-native");
    assert_eq!(native["models"]["alias"]["display_name"], "中文动态模型");
    assert_eq!(native["default_model"], "native");
    assert!(String::from_utf8_lossy(&f.bytes()).contains("# keep"));
    assert_eq!(f.record("one").unwrap()["notes"], "中文备注");
    let before = f.bytes();
    let record = f.record("one");
    assert!(matches!(
        f.client.kimi_form_update("one", &body, TOKEN).await,
        Err(vw_gateway_client::AgentAppsReadError::Http { status: 409, .. })
    ));
    assert_eq!(f.bytes(), before);
    assert_eq!(f.record("one"), record);
    f.client.kimi_provider_remove_live("one", &f.target(), TOKEN).await.unwrap();
    assert_eq!(f.record("one").unwrap()["name"], "中文供应商");
}

#[tokio::test]
async fn cc04_ui_kimi_form_invalid_denied_and_redirect_leave_no_side_effects() {
    use vw_api_types::app_providers::create::ProviderSecretWrite;
    let f = Fixture::new().await;
    let mut seed = f.request(KimiActivation::Add);
    seed.profile.declaration["provider"]["base_url"] = json!("https://fixture.example.test/v1");
    f.client.kimi_provider_save("one", &seed, TOKEN).await.unwrap();
    let view = f.client.kimi_form_read("one", &f.target(), TOKEN).await.unwrap();
    let config = view.config.unwrap();
    let mut body = KimiFormWrite {
        target_host_id: "host".into(),
        fields: view.fields,
        declaration: config.declaration.clone(),
        api_key: ProviderSecretWrite::Keep,
        activation: KimiActivation::IfMember,
        revision: Some(config.revision),
    };
    let before = f.bytes();
    let record = f.record("one");
    assert!(f.client.kimi_form_update("one", &body, "invalid-token").await.is_err());
    body.declaration = "[provider".into();
    assert!(matches!(
        f.client.kimi_form_update("one", &body, TOKEN).await,
        Err(vw_gateway_client::AgentAppsReadError::Http { status: 422, .. })
    ));
    body.declaration = config
        .declaration
        .replace("https://fixture.example.test/v1", "https://other.example.test/v1");
    assert!(f.client.kimi_form_update("one", &body, TOKEN).await.is_err());
    assert_eq!(f.bytes(), before);
    assert_eq!(f.record("one"), record);
    body.declaration = config.declaration;
    body.activation = KimiActivation::CatalogOnly;
    body.revision = None;
    body.api_key = ProviderSecretWrite::Replace { value: "fake-new".into() };
    assert!(f.client.kimi_form_create("new", &body, TOKEN).await.unwrap().catalog_saved);
    assert_eq!(f.bytes(), before);
    assert_eq!(f.record("new").unwrap()["kimi"]["declaration"]["provider"]["api_key"], "fake-new");
}
