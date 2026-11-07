use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{app_providers::mcode::*, id::TenantScope};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-mcode-token-with-at-least-32-chars";
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
        std::fs::create_dir_all(root.path().join(".minimax")).unwrap();
        std::fs::write(
            root.path().join(".minimax/config.yaml"),
            "# keep\nfuture: true\ncustom_provider:\n  other:\n    baseUrl: https://other.example\n",
        )
        .unwrap();
        std::fs::write(root.path().join(".minimax/other.yml"), b"defaults-fixture").unwrap();
        std::fs::write(root.path().join(".minimax/agent.db"), b"login-fixture").unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"mcode","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let router = super::mcode::router_with_policy(Some(policy.clone()))
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
        self.root.path().join(".minimax/config.yaml")
    }
    fn request(&self, activation: McodeActivation) -> McodeWrite {
        McodeWrite {
            target_host_id: "host".into(),
            profile: McodeProfile {
                name: "Fixture".into(),
                declaration: json!({"options":{"baseURL":"https://example.com","apiKey":"fake-secret"},"api":"openai-completions","models":{"one":{}}}),
            },
            activation,
        }
    }
    fn target(&self) -> McodeTarget {
        McodeTarget { target_host_id: "host".into() }
    }
    fn record(&self, id: &str) -> Option<Value> {
        crate::app_providers::AppProviderStore::new(self.db.clone())
            .unwrap()
            .mcode_record(&TenantScope::new("p", "a").unwrap(), id)
            .unwrap()
    }
    fn bytes(&self) -> Vec<u8> {
        assert_eq!(
            std::fs::read(self.root.path().join(".minimax/other.yml")).unwrap(),
            b"defaults-fixture"
        );
        assert_eq!(
            std::fs::read(self.root.path().join(".minimax/agent.db")).unwrap(),
            b"login-fixture"
        );
        std::fs::read(self.file()).unwrap()
    }
}
#[tokio::test]
async fn cc04_mcode_sdk_api_member_edit_remove_and_catalog_delete() {
    let f = Fixture::new().await;
    assert_eq!(
        f.client
            .mcode_provider_save("one", &f.request(McodeActivation::Add), TOKEN)
            .await
            .unwrap()
            .live,
        McodeLiveResult::Applied
    );
    assert_eq!(
        f.client.mcode_provider_remove_live("one", &f.target(), TOKEN).await.unwrap().live,
        McodeLiveResult::Applied
    );
    assert_eq!(f.client.mcode_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    assert_eq!(
        f.client.mcode_provider_add("one", &f.target(), TOKEN).await.unwrap().live,
        McodeLiveResult::Applied
    );
    let mut request = f.request(McodeActivation::IfMember);
    request.profile.declaration["options"]["apiKey"] = json!("fake-edited");
    f.client.mcode_provider_save("one", &request, TOKEN).await.unwrap();
    assert!(String::from_utf8_lossy(&f.bytes()).contains("fake-edited"));
    let state = f.client.mcode_state(&f.target(), TOKEN).await.unwrap();
    assert_eq!(state.members, vec!["one", "other"]);
    assert!(String::from_utf8_lossy(&f.bytes()).contains("# keep\nfuture: true"));
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    assert!(
        store
            .delete(&tenant, vw_api_types::agent_apps::AppId::Mcode, "one", f.root.path())
            .is_err()
    );
    assert_eq!(
        f.client.mcode_provider_remove_live("one", &f.target(), TOKEN).await.unwrap().live,
        McodeLiveResult::Applied
    );
    assert!(f.record("one").is_some());
    assert_eq!(f.client.mcode_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    store.delete(&tenant, vw_api_types::agent_apps::AppId::Mcode, "one", f.root.path()).unwrap();
    assert!(f.record("one").is_none());
    let state: Value = serde_json::from_slice(
        &std::fs::read(f.root.path().join("private/provider-live/aG9zdA/mcode.state.json"))
            .unwrap(),
    )
    .unwrap();
    assert!(state["pointer"].is_null());
}
#[tokio::test]
async fn cc04_mcode_import_is_read_only_and_does_not_overwrite_catalog() {
    let f = Fixture::new().await;
    let before = f.bytes();
    let mut permissions = std::fs::metadata(f.file()).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(f.file(), permissions).unwrap();
    assert_eq!(f.client.mcode_import(&f.target(), TOKEN).await.unwrap().imported, 1);
    assert_eq!(f.client.mcode_import(&f.target(), TOKEN).await.unwrap().imported, 0);
    assert_eq!(f.bytes(), before);
    assert!(f.record("other").is_some());
    assert!(!f.root.path().join("private/provider-live").exists());
    f.client
        .mcode_provider_save("draft", &f.request(McodeActivation::CatalogOnly), TOKEN)
        .await
        .unwrap();
    assert_eq!(f.bytes(), before);
}
#[tokio::test]
async fn cc04_mcode_permission_tenant_and_host_denials_leave_no_changes() {
    let f = Fixture::new().await;
    let before = f.bytes();
    assert!(
        f.client
            .mcode_provider_save("one", &f.request(McodeActivation::Add), "bad-token")
            .await
            .is_err()
    );
    let mut bad = f.request(McodeActivation::Add);
    bad.target_host_id = "foreign".into();
    assert!(f.client.mcode_provider_save("one", &bad, TOKEN).await.is_err());
    let foreign = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", f.port)
            .with_tenant_scope(TenantScope::new("p", "b").unwrap()),
    )
    .unwrap();
    assert!(
        foreign.mcode_provider_save("one", &f.request(McodeActivation::Add), TOKEN).await.is_err()
    );
    let path = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(path, policy.to_string()).unwrap();
    assert!(
        f.client.mcode_provider_save("one", &f.request(McodeActivation::Add), TOKEN).await.is_err()
    );
    assert_eq!(f.client.mcode_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    assert_eq!(f.bytes(), before);
    assert!(f.record("one").is_none());
    assert!(!f.root.path().join("private/provider-live").exists());
}

#[tokio::test]
async fn cc04_mcode_collision_and_rename_requests_cannot_overwrite() {
    let f = Fixture::new().await;
    let before = f.bytes();
    for activation in [McodeActivation::Add, McodeActivation::CatalogOnly] {
        assert!(
            f.client.mcode_provider_save("other", &f.request(activation), TOKEN).await.is_err()
        );
        assert!(f.record("other").is_none());
        assert_eq!(f.bytes(), before);
    }
    f.client.mcode_import(&f.target(), TOKEN).await.unwrap();
    let record = f.record("other").unwrap();
    assert!(
        f.client
            .mcode_provider_save("other", &f.request(McodeActivation::Add), TOKEN)
            .await
            .is_err()
    );
    assert_eq!(f.record("other").unwrap(), record);
    assert_eq!(f.bytes(), before);
    let mut body = serde_json::to_value(f.request(McodeActivation::IfMember)).unwrap();
    body["originalId"] = json!("other");
    let response = reqwest::Client::new()
        .put(format!("http://127.0.0.1:{}/v1/app-providers/mcode/renamed/config", f.port))
        .header("x-agent-apps-token", TOKEN)
        .header("x-platform-id", "p")
        .header("x-company-id", "a")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 400);
    assert!(f.record("renamed").is_none());
    assert_eq!(f.bytes(), before);
}

#[tokio::test]
async fn cc04_mcode_editor_scope_is_checked_by_server() {
    let f = Fixture::new().await;
    let before = f.bytes();
    let mut body = McodeEditorWrite {
        target_host_id: "host".into(),
        name: "Fixture".into(),
        activation: McodeActivation::Add,
        yaml: "custom_provider:\n  one: {}\ndefaultModel: changed\n".into(),
    };
    assert!(f.client.mcode_provider_editor_save("one", &body, TOKEN).await.is_err());
    assert_eq!(f.bytes(), before);
    assert!(f.record("one").is_none());
    body.yaml=serde_json::json!({"custom_provider":{"one":f.request(McodeActivation::Add).profile.declaration}}).to_string();
    assert_eq!(
        f.client.mcode_provider_editor_save("one", &body, TOKEN).await.unwrap().live,
        McodeLiveResult::Applied
    );
    assert!(String::from_utf8_lossy(&f.bytes()).contains("fake-secret"));
}
#[tokio::test]
async fn cc04_mcode_native_edit_remove_and_selected_model_guard() {
    let f = Fixture::new().await;
    f.client.mcode_provider_save("one", &f.request(McodeActivation::Add), TOKEN).await.unwrap();
    let mut doc: Value = serde_yaml::from_slice(&f.bytes()).unwrap();
    doc["defaultLightModel"] = json!("custom_provider:one/one");
    std::fs::write(f.file(), serde_yaml::to_string(&doc).unwrap()).unwrap();
    let before = f.bytes();
    let record = f.record("one");
    assert!(f.client.mcode_provider_remove_live("one", &f.target(), TOKEN).await.is_err());
    let mut request = f.request(McodeActivation::IfMember);
    request.profile.declaration["models"] = json!({"new":{}});
    assert!(f.client.mcode_provider_save("one", &request, TOKEN).await.is_err());
    assert_eq!(f.bytes(), before);
    assert_eq!(f.record("one"), record);
    doc.as_object_mut().unwrap().remove("defaultLightModel");
    doc["custom_provider"]["one"]["options"]["apiKey"] = json!("fake-rotated-native");
    doc["custom_provider"]["one"]["future"] = json!({"keep":true});
    std::fs::write(f.file(), serde_yaml::to_string(&doc).unwrap()).unwrap();
    assert_eq!(
        f.client.mcode_provider_remove_live("one", &f.target(), TOKEN).await.unwrap().live,
        McodeLiveResult::Applied
    );
    assert_eq!(f.record("one").unwrap()["mcode"]["declaration"]["future"], json!({"keep":true}));
    f.client.mcode_provider_add("one", &f.target(), TOKEN).await.unwrap();
    assert!(String::from_utf8_lossy(&f.bytes()).contains("fake-rotated-native"));
    let bytes = f.bytes();
    let mut policy: Value =
        serde_json::from_slice(&std::fs::read(f.root.path().join("access.json")).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(f.root.path().join("access.json"), policy.to_string()).unwrap();
    let body = McodeEditorWrite {
        target_host_id: "host".into(),
        name: "Fixture".into(),
        yaml: serde_yaml::to_string(
            &json!({"custom_provider":{"one":f.request(McodeActivation::Add).profile.declaration}}),
        )
        .unwrap(),
        activation: McodeActivation::IfMember,
    };
    assert!(f.client.mcode_provider_editor_save("one", &body, TOKEN).await.is_err());
    assert_eq!(f.bytes(), bytes);
}
#[tokio::test]
async fn cc04_mcode_native_account_and_lock_denials_leave_no_catalog_changes() {
    let f = Fixture::new().await;
    let bytes=b"defaultModel: minimax/native\ncustom_provider:\n  account:\n    kind: minimax\n    options: {apiKey: fake-native-account}\n";
    std::fs::write(f.file(), bytes).unwrap();
    for activation in [McodeActivation::Add, McodeActivation::CatalogOnly] {
        assert!(
            f.client.mcode_provider_save("account", &f.request(activation), TOKEN).await.is_err()
        );
        assert!(f.record("account").is_none());
        assert_eq!(f.bytes(), bytes);
    }
    assert!(f.client.mcode_state(&f.target(), TOKEN).await.unwrap().members.is_empty());
    assert_eq!(f.client.mcode_import(&f.target(), TOKEN).await.unwrap().imported, 0);
    let lock = f.file().with_extension("yaml.lock");
    std::fs::create_dir(&lock).unwrap();
    assert!(
        f.client.mcode_provider_save("one", &f.request(McodeActivation::Add), TOKEN).await.is_err()
    );
    assert!(lock.exists());
    assert!(f.record("one").is_none());
    assert_eq!(f.bytes(), bytes);
    std::fs::remove_dir(lock).unwrap();
    std::fs::write(f.file(), "custom_provider: [broken\n").unwrap();
    assert!(f.client.mcode_state(&f.target(), TOKEN).await.is_err());
    assert!(f.client.mcode_import(&f.target(), TOKEN).await.is_err());
    assert!(f.record("one").is_none());
}

#[tokio::test]
async fn cc04_ui_mcode_form_roundtrip_preserves_secrets_and_other_nodes() {
    use vw_api_types::app_providers::create::ProviderSecretWrite;
    let f = Fixture::new().await;
    let mut initial = f.request(McodeActivation::Add);
    initial.profile.declaration["options"]["headers"] = json!({"Authorization":"private-fixture"});
    f.client.mcode_provider_save("fixture", &initial, TOKEN).await.unwrap();
    let before = f.bytes();
    let view = f.client.mcode_form_read("fixture", &f.target(), TOKEN).await.unwrap();
    let config = view.config.unwrap();
    assert!(!config.declaration.contains("fake-secret"));
    assert!(!config.declaration.contains("private-fixture"));
    let mut node: Value = serde_yaml::from_str(&config.declaration).unwrap();
    node["models"]["one"]["name"] = json!("中文模型");
    let mut body = McodeFormWrite {
        target_host_id: "host".into(),
        fields: view.fields,
        declaration: serde_yaml::to_string(&node).unwrap(),
        api_key: ProviderSecretWrite::Keep,
        activation: McodeActivation::IfMember,
        revision: Some(config.revision),
    };
    body.fields.name = "中文供应商".into();
    let result = f.client.mcode_form_update("fixture", &body, TOKEN).await.unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, McodeLiveResult::Applied);
    let after = f.bytes();
    assert_ne!(before, after);
    let yaml: Value = serde_yaml::from_slice(&after).unwrap();
    assert_eq!(yaml["custom_provider"]["fixture"]["options"]["apiKey"], "fake-secret");
    assert_eq!(
        yaml["custom_provider"]["fixture"]["options"]["headers"]["Authorization"],
        "private-fixture"
    );
    assert_eq!(yaml["custom_provider"]["fixture"]["models"]["one"]["name"], "中文模型");
    assert_eq!(yaml["custom_provider"]["other"]["baseUrl"], "https://other.example");
    assert!(String::from_utf8(after.clone()).unwrap().contains("# keep"));
    assert!(matches!(
        f.client.mcode_form_update("fixture", &body, TOKEN).await.unwrap_err(),
        vw_gateway_client::AgentAppsReadError::Http { status: 409, .. }
    ));
    assert_eq!(f.bytes(), after);
}

#[tokio::test]
async fn cc04_ui_mcode_form_rejects_invalid_yaml_and_unauthorized_writes() {
    use vw_api_types::app_providers::{basic::ProviderBasic, create::ProviderSecretWrite};
    let f = Fixture::new().await;
    let before = f.bytes();
    let mut body = McodeFormWrite {
        target_host_id: "host".into(),
        fields: ProviderBasic {
            name: "中文".into(),
            category: None,
            notes: None,
            website_url: None,
        },
        declaration: "models: [".into(),
        api_key: ProviderSecretWrite::Replace { value: "fake-key".into() },
        activation: McodeActivation::Add,
        revision: None,
    };
    let err = f.client.mcode_form_create("fixture", &body, TOKEN).await.unwrap_err();
    assert!(matches!(err, vw_gateway_client::AgentAppsReadError::Http { status: 422, .. }));
    assert_eq!(f.bytes(), before);
    assert!(f.record("fixture").is_none());
    body.declaration =
        "kind: custom\noptions:\n  baseURL: https://example.com\nmodels:\n  one: {}".into();
    assert!(f.client.mcode_form_create("fixture", &body, "wrong-token").await.is_err());
    assert_eq!(f.bytes(), before);
    assert!(f.record("fixture").is_none());
    let result = f.client.mcode_form_create("fixture", &body, TOKEN).await.unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, McodeLiveResult::Applied);
}
