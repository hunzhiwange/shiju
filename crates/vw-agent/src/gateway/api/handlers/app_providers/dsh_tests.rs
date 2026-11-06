use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{app_providers::dsh::*, id::TenantScope};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-dsh-token-with-at-least-32-chars";
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
        std::fs::create_dir_all(root.path().join(".dsh/profiles/web")).unwrap();
        std::fs::write(
            root.path().join(".dsh/profiles/web/cordis.patch.yml"),
            "# keep\n- id: native-plugin\n  config: {value: native}\n",
        )
        .unwrap();
        std::fs::write(root.path().join(".dsh/other.yml"), b"defaults-fixture").unwrap();
        std::fs::write(root.path().join(".dsh/agent.db"), b"login-fixture").unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"dsh","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let router = super::dsh::router_with_policy(Some(policy.clone()))
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
        self.root.path().join(".dsh/profiles/web/cordis.patch.yml")
    }
    fn request(&self, activation: DshActivation) -> DshWrite {
        DshWrite {
            target_host_id: "host".into(),
            profile: DshProfile {
                name: "Fixture".into(),
                declaration: json!({"options":{"baseURL":"https://example.com","apiKey":"fake-secret"},"api":"openai-completions","models":{"one":{}}}),
            },
            activation,
        }
    }
    fn target(&self) -> DshTarget {
        DshTarget { target_host_id: "host".into() }
    }
    fn record(&self, id: &str) -> Option<Value> {
        crate::app_providers::AppProviderStore::new(self.db.clone())
            .unwrap()
            .dsh_record(&TenantScope::new("p", "a").unwrap(), id)
            .unwrap()
    }
    fn bytes(&self) -> Vec<u8> {
        assert_eq!(
            std::fs::read(self.root.path().join(".dsh/other.yml")).unwrap(),
            b"defaults-fixture"
        );
        assert_eq!(
            std::fs::read(self.root.path().join(".dsh/agent.db")).unwrap(),
            b"login-fixture"
        );
        std::fs::read(self.file()).unwrap()
    }
}
#[tokio::test]
async fn cc04_dsh_sdk_api_updates_profiles_and_keeps_native_account() {
    let f = Fixture::new().await;
    let result =
        f.client.dsh_provider_save("one", &f.request(DshActivation::Add), TOKEN).await.unwrap();
    assert_eq!(result.live, DshLiveResult::Applied);
    assert!(String::from_utf8_lossy(&f.bytes()).contains("SHIJU_DSH_ONE"));
    assert_eq!(f.client.dsh_state(&f.target(), TOKEN).await.unwrap().members, vec!["one"]);
}
#[tokio::test]
async fn cc04_dsh_binding_errors_and_denials_have_no_side_effects() {
    let f = Fixture::new().await;
    let before = f.bytes();
    let mut request = f.request(DshActivation::Add);
    request.profile.declaration["options"]["apiKey"] = json!("");
    request.profile.declaration["apiKeyEnv"] = json!("ABSENT");
    let error = f.client.dsh_provider_save("one", &request, TOKEN).await.unwrap_err();
    match error {
        vw_gateway_client::AgentAppsReadError::Http { status, body } => {
            assert_eq!(status, 422);
            assert_eq!(body, json!({"code":"invalid_binding"}));
        }
        _ => panic!("expected typed binding error"),
    }
    assert!(f.record("one").is_none());
    assert_eq!(f.bytes(), before);
    let stored = json!({"name":"Bound","meta":{"authBinding":{"accountId":"missing"}}});
    GatewaySettingsStore::new(f.db.clone())
        .unwrap()
        .upsert(
            &TenantScope::new("p", "a").unwrap(),
            "app_providers.records",
            "\"dsh\":bound",
            &stored.to_string(),
        )
        .unwrap();
    let error = f
        .client
        .dsh_provider_save("bound", &f.request(DshActivation::Add), TOKEN)
        .await
        .unwrap_err();
    match error {
        vw_gateway_client::AgentAppsReadError::Http { status, body } => {
            assert_eq!(status, 422);
            assert_eq!(body, json!({"code":"invalid_binding"}));
        }
        _ => panic!("expected typed binding error"),
    }
    assert_eq!(f.record("bound"), Some(stored));
    assert_eq!(f.bytes(), before);
    for (company, host, token) in
        [("b", "host", TOKEN), ("a", "other", TOKEN), ("a", "host", "fake-invalid")]
    {
        let mut body = f.request(DshActivation::Add);
        body.target_host_id = host.into();
        let response = reqwest::Client::new()
            .put(format!("http://127.0.0.1:{}/v1/app-providers/dsh/one/config", f.port))
            .header("x-platform-id", "p")
            .header("x-company-id", company)
            .header("x-agent-apps-token", token)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert!(!response.status().is_success());
        assert!(f.record("one").is_none());
        assert_eq!(f.bytes(), before);
    }
    let mut policy: Value =
        serde_json::from_slice(&std::fs::read(f.root.path().join("access.json")).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(f.root.path().join("access.json"), policy.to_string()).unwrap();
    assert!(
        f.client.dsh_provider_save("one", &f.request(DshActivation::Add), TOKEN).await.is_err()
    );
    assert!(f.record("one").is_none());
    assert_eq!(f.bytes(), before);
}
#[tokio::test]
async fn cc04_dsh_catalog_first_add_and_current_edit_use_all_profiles() {
    let f = Fixture::new().await;
    let credentials = f.root.path().join(".dsh/.credentials.yaml");
    let native = "version: 1\nrefs: {OTHER: fake-other}\nrecords:\n  deepseek-account-platform/default:\n    kind: grant\n    payload: {version: 1, token: fake-native, issuer: 'https://platform.deepseek.com'}\n";
    std::fs::write(&credentials, native).unwrap();
    let before = f.bytes();
    let result = f
        .client
        .dsh_provider_save("one", &f.request(DshActivation::CatalogOnly), TOKEN)
        .await
        .unwrap();
    assert_eq!(result.live, DshLiveResult::NotAttempted);
    assert_eq!(f.bytes(), before);
    assert_eq!(
        f.client.dsh_provider_add("one", &f.target(), TOKEN).await.unwrap().live,
        DshLiveResult::Applied
    );
    let mut edited = f.request(DshActivation::IfMember);
    edited.profile.declaration["options"]["apiKey"] = json!("fake-rotated");
    assert_eq!(
        f.client.dsh_provider_save("one", &edited, TOKEN).await.unwrap().live,
        DshLiveResult::Applied
    );
    for name in ["web", "desktop"] {
        let text = std::fs::read_to_string(
            f.root.path().join(format!(".dsh/profiles/{name}/cordis.patch.yml")),
        )
        .unwrap();
        assert!(text.contains("SHIJU_DSH_ONE"));
    }
    let creds: serde_yaml::Value =
        serde_yaml::from_slice(&std::fs::read(credentials).unwrap()).unwrap();
    let original: serde_yaml::Value = serde_yaml::from_str(native).unwrap();
    assert_eq!(creds["records"], original["records"]);
    assert_eq!(creds["refs"]["OTHER"], "fake-other");
    assert_eq!(creds["refs"]["SHIJU_DSH_ONE"], "fake-rotated");
    let before = f.bytes();
    let result = f
        .client
        .dsh_provider_save("parked", &f.request(DshActivation::IfMember), TOKEN)
        .await
        .unwrap();
    assert_eq!(result.live, DshLiveResult::NotAttempted);
    assert!(f.record("parked").is_some());
    assert_eq!(f.bytes(), before);
}
