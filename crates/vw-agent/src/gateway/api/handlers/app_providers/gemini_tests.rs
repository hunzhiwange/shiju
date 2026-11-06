use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{app_providers::gemini::*, id::TenantScope};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-gemini-token-with-at-least-32-chars";
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
        std::fs::create_dir(root.path().join(".gemini")).unwrap();
        std::fs::write(
            root.path().join(".gemini/settings.json"),
            r#"{"mcpServers":{"fixture":{}},"ui":{"theme":"dark"}}"#,
        )
        .unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"gemini","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let router = super::gemini::router_with_policy(Some(policy.clone()))
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
        self.root.path().join(".gemini/settings.json")
    }
    fn request(&self, official: bool, activation: GeminiActivation) -> GeminiWrite {
        GeminiWrite {
            target_host_id: "host".into(),
            profile: GeminiProfile {
                name: "Fixture".into(),
                config: if official {
                    GeminiConfig::Official {}
                } else {
                    GeminiConfig::ThirdParty {
                        base_url: "https://fixture.test".into(),
                        api_key: "fake-secret".into(),
                    }
                },
                model: None,
            },
            activation,
        }
    }
    fn target(&self) -> GeminiTarget {
        GeminiTarget { target_host_id: "host".into() }
    }
    fn state(&self) -> Value {
        serde_json::from_slice(
            &std::fs::read(self.root.path().join("private/provider-live/aG9zdA/gemini.state.json"))
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
                &format!("\"gemini\":{id}"),
            )
            .unwrap()
    }
}

#[tokio::test]
async fn cc04_gemini_sdk_api_switch_first_and_edit_keep_user_settings() {
    let f = Fixture::new().await;
    let r = f
        .client
        .gemini_provider_save("third", &f.request(false, GeminiActivation::First), TOKEN)
        .await
        .unwrap();
    assert_eq!(r.live, GeminiLiveResult::Applied);
    assert!(r.catalog_saved);
    assert_eq!(f.state()["pointer"]["id"], "third");
    assert!(
        std::fs::read_to_string(f.root.path().join(".gemini/.env"))
            .unwrap()
            .contains("fake-secret")
    );
    let mut edit = f.request(false, GeminiActivation::IfCurrent);
    edit.profile.model = Some("edited".into());
    assert_eq!(
        f.client.gemini_provider_save("third", &edit, TOKEN).await.unwrap().live,
        GeminiLiveResult::Applied
    );
    let v: Value = serde_json::from_slice(&std::fs::read(f.file()).unwrap()).unwrap();
    assert_eq!(v["model"]["name"], "edited");
    assert_eq!(v["ui"]["theme"], "dark");
    assert_eq!(v["mcpServers"]["fixture"], json!({}));
    let r = f
        .client
        .gemini_provider_save("official", &f.request(true, GeminiActivation::CatalogOnly), TOKEN)
        .await
        .unwrap();
    assert_eq!(r.live, GeminiLiveResult::NotAttempted);
    assert_eq!(
        f.client.gemini_provider_switch("official", &f.target(), TOKEN).await.unwrap().live,
        GeminiLiveResult::Applied
    );
    assert!(std::fs::read(f.root.path().join(".gemini/.env")).unwrap().is_empty());
    let v: Value = serde_json::from_slice(&std::fs::read(f.file()).unwrap()).unwrap();
    assert_eq!(v["security"]["auth"]["selectedType"], "oauth-personal");
    assert_eq!(v["mcpServers"]["fixture"], json!({}));
    assert_eq!(f.state()["pointer"]["id"], "official");
    assert_eq!(
        f.client.gemini_provider_reapply("official", &f.target(), TOKEN).await.unwrap().live,
        GeminiLiveResult::Applied
    );
    assert!(!serde_json::to_string(&r).unwrap().contains("fake-secret"));
}
#[tokio::test]
async fn cc04_gemini_denial_and_bad_config_keep_files_and_records() {
    let f = Fixture::new().await;
    let before = std::fs::read(f.file()).unwrap();
    assert!(
        f.client
            .gemini_provider_save(
                "denied",
                &f.request(false, GeminiActivation::Switch),
                "bad-token"
            )
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
            .gemini_provider_save("denied", &f.request(false, GeminiActivation::Switch), TOKEN)
            .await
            .is_err()
    );
    assert!(f.record("denied").is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert!(!f.root.path().join(".gemini/.env").exists());
}

#[tokio::test]
async fn cc04_gemini_sdk_api_recovers_both_files_before_replanning() {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use std::os::unix::fs::MetadataExt;
    use vw_agent_apps::providers::live::commit::*;
    let f = Fixture::new().await;
    f.client
        .gemini_provider_save("one", &f.request(true, GeminiActivation::Switch), TOKEN)
        .await
        .unwrap();
    let directory = f.root.path().join(".gemini");
    let meta = std::fs::metadata(&directory).unwrap();
    let mut changes = Vec::new();
    for (index, name) in ["settings.json", ".env"].into_iter().enumerate() {
        let before = std::fs::read(directory.join(name)).unwrap();
        let staged = format!(".shiju-live-recovery-{index}");
        std::fs::write(directory.join(&staged), &before).unwrap();
        std::fs::set_permissions(directory.join(&staged), std::fs::Permissions::from_mode(0o600))
            .unwrap();
        let digest = URL_SAFE_NO_PAD.encode(Sha256::digest(&before));
        std::fs::remove_file(directory.join(name)).unwrap();
        changes.push(PendingFile {
            name: name.as_bytes().to_vec(),
            staged: staged.into_bytes(),
            pre: None,
            planned: Some(digest),
        });
    }
    let pending = Pending {
        version: 1,
        host: ExecutionHostId::new("host").unwrap(),
        app: vw_api_types::agent_apps::AppId::Gemini,
        tenant: TenantScope::new("p", "a").unwrap(),
        directory_device: meta.dev(),
        directory_inode: meta.ino(),
        target: PendingTarget { pointer: Some("one".into()), mode: None },
        files: changes,
        published: true,
    };
    let path = f.root.path().join("private/provider-live/aG9zdA/gemini.pending.json");
    std::fs::write(&path, serde_json::to_vec(&pending).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let result = f.client.gemini_provider_switch("one", &f.target(), TOKEN).await.unwrap();
    assert_eq!(result.live, GeminiLiveResult::RecoveryRequired);
    assert!(f.file().exists());
    assert!(directory.join(".env").exists());
    assert!(!path.exists());
    assert_eq!(f.state()["pointer"]["id"], "one");
    assert_eq!(
        f.client.gemini_provider_switch("one", &f.target(), TOKEN).await.unwrap().live,
        GeminiLiveResult::Applied
    );
}

#[tokio::test]
async fn cc04_gemini_activation_and_tenant_host_scope_are_enforced() {
    let f = Fixture::new().await;
    f.client
        .gemini_provider_save("one", &f.request(false, GeminiActivation::First), TOKEN)
        .await
        .unwrap();
    let before = std::fs::read(f.file()).unwrap();
    let r = f
        .client
        .gemini_provider_save("two", &f.request(true, GeminiActivation::First), TOKEN)
        .await
        .unwrap();
    assert_eq!(r.live, GeminiLiveResult::NotAttempted);
    let r = f
        .client
        .gemini_provider_save("two", &f.request(true, GeminiActivation::IfCurrent), TOKEN)
        .await
        .unwrap();
    assert_eq!(r.live, GeminiLiveResult::NotAttempted);
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    let mut other = f.request(true, GeminiActivation::Switch);
    other.target_host_id = "other-host".into();
    assert!(f.client.gemini_provider_save("foreign", &other, TOKEN).await.is_err());
    assert!(f.record("foreign").is_none());
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    let summaries = store
        .list(&TenantScope::new("p", "a").unwrap(), vw_api_types::agent_apps::AppId::Gemini)
        .unwrap();
    assert_eq!(summaries.len(), 2);
    assert!(
        store
            .list(&TenantScope::new("p", "other").unwrap(), vw_api_types::agent_apps::AppId::Gemini)
            .unwrap()
            .is_empty()
    );
    let path = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["company_id"] = json!("other");
    std::fs::write(path, policy.to_string()).unwrap();
    assert!(f.client.gemini_provider_switch("two", &f.target(), TOKEN).await.is_err());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
}
#[tokio::test]
async fn cc04_gemini_invalid_live_json_prevents_save_and_env_write() {
    let f = Fixture::new().await;
    std::fs::write(f.file(), b"invalid-json").unwrap();
    assert!(
        f.client
            .gemini_provider_save("one", &f.request(false, GeminiActivation::Switch), TOKEN)
            .await
            .is_err()
    );
    assert!(f.record("one").is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), b"invalid-json");
    assert!(!f.root.path().join(".gemini/.env").exists());
}

#[tokio::test]
async fn cc04_gemini_managed_row_is_refused_without_touching_live() {
    let f = Fixture::new().await;
    f.client
        .gemini_provider_save("one", &f.request(false, GeminiActivation::CatalogOnly), TOKEN)
        .await
        .unwrap();
    let before = std::fs::read(f.file()).unwrap();
    let mut row: Value = serde_json::from_str(&f.record("one").unwrap()).unwrap();
    row["meta"] = json!({"authBinding":{"accountId":"fake-unresolved-account"}});
    GatewaySettingsStore::new(f.db.clone())
        .unwrap()
        .upsert(
            &TenantScope::new("p", "a").unwrap(),
            "app_providers.records",
            "\"gemini\":one",
            &row.to_string(),
        )
        .unwrap();
    assert!(f.client.gemini_provider_switch("one", &f.target(), TOKEN).await.is_err());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert!(!f.root.path().join(".gemini/.env").exists());
}
