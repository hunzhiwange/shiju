use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{app_providers::pi::*, id::TenantScope};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-pi-token-with-at-least-32-chars";
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
        std::fs::create_dir_all(root.path().join(".pi/agent")).unwrap();
        std::fs::write(
            root.path().join(".pi/agent/models.json"),
            r#"{// keep
"future":true,"providers":{"other":{"models":[]}}}"#,
        )
        .unwrap();
        std::fs::write(root.path().join(".pi/agent/settings.json"), b"defaults-fixture").unwrap();
        std::fs::write(root.path().join(".pi/agent/auth.json"), b"login-fixture").unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"pi","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let router = super::pi::router_with_policy(Some(policy.clone()))
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
        self.root.path().join(".pi/agent/models.json")
    }
    fn request(&self, activation: PiActivation) -> PiWrite {
        PiWrite {
            target_host_id: "host".into(),
            profile: PiProfile {
                name: "Fixture".into(),
                declaration: json!({"api":"openai-completions","apiKey":"fake-secret","models":[{"id":"one"}]}),
            },
            activation,
        }
    }
    fn target(&self) -> PiTarget {
        PiTarget { target_host_id: "host".into() }
    }
    fn record(&self, id: &str) -> Option<Value> {
        crate::app_providers::AppProviderStore::new(self.db.clone())
            .unwrap()
            .pi_record(&TenantScope::new("p", "a").unwrap(), id)
            .unwrap()
    }
    fn bytes(&self) -> Vec<u8> {
        assert_eq!(
            std::fs::read(self.root.path().join(".pi/agent/settings.json")).unwrap(),
            b"defaults-fixture"
        );
        assert_eq!(
            std::fs::read(self.root.path().join(".pi/agent/auth.json")).unwrap(),
            b"login-fixture"
        );
        std::fs::read(self.file()).unwrap()
    }
}
#[tokio::test]
async fn cc04_pi_sdk_api_member_edit_remove_and_catalog_delete() {
    let f = Fixture::new().await;
    assert_eq!(
        f.client.pi_provider_save("one", &f.request(PiActivation::Add), TOKEN).await.unwrap().live,
        PiLiveResult::Applied
    );
    assert_eq!(
        f.client.pi_provider_remove_live("one", &f.target(), TOKEN).await.unwrap().live,
        PiLiveResult::Applied
    );
    assert_eq!(f.client.pi_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    assert_eq!(
        f.client.pi_provider_add("one", &f.target(), TOKEN).await.unwrap().live,
        PiLiveResult::Applied
    );
    let mut request = f.request(PiActivation::IfMember);
    request.profile.declaration["apiKey"] = json!("fake-edited");
    f.client.pi_provider_save("one", &request, TOKEN).await.unwrap();
    assert!(String::from_utf8_lossy(&f.bytes()).contains("fake-edited"));
    let state = f.client.pi_state(&f.target(), TOKEN).await.unwrap();
    assert_eq!(state.members, vec!["one", "other"]);
    assert!(String::from_utf8_lossy(&f.bytes()).contains("// keep\n\"future\":true"));
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    assert!(
        store.delete(&tenant, vw_api_types::agent_apps::AppId::Pi, "one", f.root.path()).is_err()
    );
    assert_eq!(
        f.client.pi_provider_remove_live("one", &f.target(), TOKEN).await.unwrap().live,
        PiLiveResult::Applied
    );
    assert!(f.record("one").is_some());
    assert_eq!(f.client.pi_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    store.delete(&tenant, vw_api_types::agent_apps::AppId::Pi, "one", f.root.path()).unwrap();
    assert!(f.record("one").is_none());
    let state: Value = serde_json::from_slice(
        &std::fs::read(f.root.path().join("private/provider-live/aG9zdA/pi.state.json")).unwrap(),
    )
    .unwrap();
    assert!(state["pointer"].is_null());
}
#[tokio::test]
async fn cc04_pi_import_is_read_only_and_does_not_overwrite_catalog() {
    let f = Fixture::new().await;
    let before = f.bytes();
    let mut permissions = std::fs::metadata(f.file()).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(f.file(), permissions).unwrap();
    assert_eq!(f.client.pi_import(&f.target(), TOKEN).await.unwrap().imported, 1);
    assert_eq!(f.client.pi_import(&f.target(), TOKEN).await.unwrap().imported, 0);
    assert_eq!(f.bytes(), before);
    assert!(f.record("other").is_some());
    assert!(!f.root.path().join("private/provider-live").exists());
    f.client.pi_provider_save("draft", &f.request(PiActivation::CatalogOnly), TOKEN).await.unwrap();
    assert_eq!(f.bytes(), before);
}
#[tokio::test]
async fn cc04_pi_permission_tenant_and_host_denials_leave_no_changes() {
    let f = Fixture::new().await;
    let before = f.bytes();
    assert!(
        f.client.pi_provider_save("one", &f.request(PiActivation::Add), "bad-token").await.is_err()
    );
    let mut bad = f.request(PiActivation::Add);
    bad.target_host_id = "foreign".into();
    assert!(f.client.pi_provider_save("one", &bad, TOKEN).await.is_err());
    let foreign = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", f.port)
            .with_tenant_scope(TenantScope::new("p", "b").unwrap()),
    )
    .unwrap();
    assert!(foreign.pi_provider_save("one", &f.request(PiActivation::Add), TOKEN).await.is_err());
    let path = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(path, policy.to_string()).unwrap();
    assert!(f.client.pi_provider_save("one", &f.request(PiActivation::Add), TOKEN).await.is_err());
    assert_eq!(f.client.pi_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    assert_eq!(f.bytes(), before);
    assert!(f.record("one").is_none());
    assert!(!f.root.path().join("private/provider-live").exists());
}

#[tokio::test]
async fn cc04_pi_collision_and_rename_requests_cannot_overwrite() {
    let f = Fixture::new().await;
    let before = f.bytes();
    for activation in [PiActivation::Add, PiActivation::CatalogOnly] {
        assert!(f.client.pi_provider_save("other", &f.request(activation), TOKEN).await.is_err());
        assert!(f.record("other").is_none());
        assert_eq!(f.bytes(), before);
    }
    f.client.pi_import(&f.target(), TOKEN).await.unwrap();
    let record = f.record("other").unwrap();
    assert!(
        f.client.pi_provider_save("other", &f.request(PiActivation::Add), TOKEN).await.is_err()
    );
    assert_eq!(f.record("other").unwrap(), record);
    assert_eq!(f.bytes(), before);
    let mut body = serde_json::to_value(f.request(PiActivation::IfMember)).unwrap();
    body["originalId"] = json!("other");
    let response = reqwest::Client::new()
        .put(format!("http://127.0.0.1:{}/v1/app-providers/pi/renamed/config", f.port))
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
async fn cc04_pi_disabled_edits_metadata_and_import_preserve_external_changes() {
    let f = Fixture::new().await;
    f.client.pi_provider_save("one", &f.request(PiActivation::IfMember), TOKEN).await.unwrap();
    assert_eq!(f.client.pi_state(&f.target(), TOKEN).await.unwrap().members, vec!["other"]);
    f.client.pi_provider_add("one", &f.target(), TOKEN).await.unwrap();
    let mut native = json!({"future":true,"providers":{"other":{"models":[]}}});
    // Native Pi changed credentials after the catalog was saved.
    native["providers"]["one"] = json!({"apiKey":"fake-rotated-outside","future":{"keep":true}});
    std::fs::write(f.file(), native.to_string()).unwrap();
    let before = f.bytes();
    let usage:PiUsageWrite=serde_json::from_value(json!({"targetHostId":"host","usageScript":{"enabled":true,"language":"javascript","code":"throw new Error('must not execute')","autoQueryInterval":10,"apiKey":"fake-usage-secret"}})).unwrap();
    let result = f.client.pi_usage_save("one", &usage, TOKEN).await.unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, PiLiveResult::NotAttempted);
    assert_eq!(f.bytes(), before);
    let record = f.record("one").unwrap();
    assert_eq!(record["meta"]["usageScript"]["code"], usage.usage_script.code);
    f.client.pi_import(&f.target(), TOKEN).await.unwrap();
    assert_eq!(f.record("one").unwrap(), record);
    assert_eq!(f.bytes(), before);
    f.client.pi_provider_remove_live("one", &f.target(), TOKEN).await.unwrap();
    let record = f.record("one").unwrap();
    assert_eq!(record["pi"]["declaration"], native["providers"]["one"]);
    assert_eq!(record["meta"]["usageScript"]["apiKey"], "fake-usage-secret");
    f.client.pi_provider_add("one", &f.target(), TOKEN).await.unwrap();
    assert!(String::from_utf8_lossy(&f.bytes()).contains("fake-rotated-outside"));
}
