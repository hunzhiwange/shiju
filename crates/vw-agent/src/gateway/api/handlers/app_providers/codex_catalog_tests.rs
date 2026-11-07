use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{
    app_providers::{claude::ClaudeActivation, codex_catalog::*},
    id::TenantScope,
};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-codex-token-with-at-least-32-chars";
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
        std::fs::create_dir(root.path().join(".codex")).unwrap();
        std::fs::write(root.path().join(".codex/config.toml"), "# fixture comment\n").unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"codex","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let router = super::codex_catalog::router_with_policy(Some(policy.clone()))
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
        self.root.path().join(".codex/config.toml")
    }
    fn request(&self, catalog: bool, activation: ClaudeActivation) -> CodexWrite {
        CodexWrite {target_host_id:"host".into(),profile:CodexProfile{name:"Fixture".into(),config:"model_provider='vendor'\nmodel='saved'\n[model_providers.vendor]\nbase_url='https://fixture.test/v1'\nwire_api='responses'\n".into(),api_key:Some("fake-route-secret".into()),preserve_login:false},catalog:if catalog {Some(vw_api_types::model_catalog::SavedCodexCatalog {models:vec![vw_api_types::model_catalog::CodexNativeModel{slug:"saved".into(),display_name:"Saved".into(),description:"fixture".into(),base_instructions:"fixture instruction".into(),supports_reasoning_summaries:false,supports_parallel_tool_calls:false,context_window:None}]})}else{None},activation}
    }
    fn target(&self) -> CodexTarget {
        CodexTarget { target_host_id: "host".into() }
    }
    fn state(&self) -> Value {
        serde_json::from_slice(
            &std::fs::read(self.root.path().join("private/provider-live/aG9zdA/codex.state.json"))
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
                &format!("\"codex\":{id}"),
            )
            .unwrap()
    }
}

#[tokio::test]
async fn cc04_codex_catalog_sdk_api_commits_saved_directory_and_scoped_pointer() {
    let f = Fixture::new().await;
    let r = f
        .client
        .codex_provider_save("one", &f.request(true, ClaudeActivation::First), TOKEN)
        .await
        .unwrap();
    assert!(r.catalog_saved);
    assert_eq!(r.live, CodexLiveResult::Applied);
    assert_eq!(f.state()["pointer"]["id"], "one");
    let config = std::fs::read_to_string(f.file()).unwrap();
    assert!(config.contains("shiju-model-catalog.json"));
    assert!(!config.contains("modelCatalog"));
    let catalog = f.root.path().join(".codex/shiju-model-catalog.json");
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(&catalog).unwrap()).unwrap()["models"][0]["slug"],
        "saved"
    );
    assert!(!serde_json::to_string(&r).unwrap().contains("fake-route-secret"));
    let r = f
        .client
        .codex_provider_save("two", &f.request(false, ClaudeActivation::CatalogOnly), TOKEN)
        .await
        .unwrap();
    assert_eq!(r.live, CodexLiveResult::NotAttempted);
    assert_eq!(
        f.client.codex_provider_switch("two", &f.target(), TOKEN).await.unwrap().live,
        CodexLiveResult::Applied
    );
    assert_eq!(f.state()["pointer"]["id"], "two");
    assert!(!catalog.exists());
    assert!(!std::fs::read_to_string(f.file()).unwrap().contains("model_catalog_json"));
    assert_eq!(
        f.client.codex_provider_switch("one", &f.target(), TOKEN).await.unwrap().live,
        CodexLiveResult::Applied
    );
    assert!(catalog.exists());
}
#[tokio::test]
async fn cc04_codex_catalog_sdk_api_denial_and_profile_failure_keep_native_files() {
    let f = Fixture::new().await;
    let before = std::fs::read(f.file()).unwrap();
    assert!(
        f.client
            .codex_provider_save(
                "denied",
                &f.request(true, ClaudeActivation::Switch),
                "invalid-token"
            )
            .await
            .is_err()
    );
    assert!(f.record("denied").is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    f.client
        .codex_provider_save("one", &f.request(true, ClaudeActivation::CatalogOnly), TOKEN)
        .await
        .unwrap();
    std::fs::write(f.file(), "profile='active'\n[profiles.active]\nmodel_provider='foreign'\n")
        .unwrap();
    let before = std::fs::read(f.file()).unwrap();
    assert!(f.client.codex_provider_switch("one", &f.target(), TOKEN).await.is_err());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert!(!f.root.path().join(".codex/shiju-model-catalog.json").exists());
}

#[test]
fn cc04_codex_catalog_saved_revisions_are_deployment_and_tenant_scoped() {
    use crate::model_catalog::ModelCatalogStore;
    use vw_api_types::model_catalog::*;
    let root = tempfile::tempdir().unwrap();
    let store = ModelCatalogStore::new(GlobalDatabase::for_home(root.path()).unwrap()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let identity = ModelCatalogIdentity {
        target_host_id: "host".into(),
        app_id: vw_api_types::agent_apps::AppId::Codex,
        provider_id: "one".into(),
        endpoint: "https://fixture.test/v1".into(),
    };
    let mut catalog = SavedCodexCatalog {
        models: vec![CodexNativeModel {
            slug: "first".into(),
            display_name: "First".into(),
            description: "fixture".into(),
            base_instructions: "fixture".into(),
            supports_reasoning_summaries: false,
            supports_parallel_tool_calls: false,
            context_window: None,
        }],
    };
    assert!(store.read(&tenant, &identity, None).unwrap().is_none());
    let first = store.save(&tenant, &identity, Some(&catalog)).unwrap().unwrap();
    catalog.models[0].slug = "second".into();
    let second = store.save(&tenant, &identity, Some(&catalog)).unwrap().unwrap();
    assert_ne!(first, second);
    assert_eq!(
        store.read(&tenant, &identity, Some(&first)).unwrap().unwrap().models[0].slug,
        "first"
    );
    assert!(store.read(&TenantScope::new("p", "b").unwrap(), &identity, Some(&first)).is_err());
    for dimension in 0..3 {
        let mut other = identity.clone();
        match dimension {
            0 => other.target_host_id = "other".into(),
            1 => other.provider_id = "other".into(),
            _ => other.endpoint = "https://other.test/v1".into(),
        };
        assert!(store.read(&tenant, &other, Some(&first)).is_err());
    }
    assert!(store.save(&tenant, &identity, None).unwrap().is_none());
    assert_eq!(store.read(&tenant, &identity, Some(&second)).unwrap().unwrap(), catalog);
    catalog.models[0].context_window = Some(0);
    assert!(store.save(&tenant, &identity, Some(&catalog)).is_err());
}
#[tokio::test]
async fn cc04_codex_catalog_sdk_api_auth_stash_preservation_and_failure() {
    let f = Fixture::new().await;
    let auth = json!({"auth_mode":"chatgpt","tokens":{"id_token":"eyJhbGciOiJub25lIn0.eyJzdWIiOiJhbGljZSJ9.","access_token":"fake-access","refresh_token":"fake-refresh","account_id":"team"}});
    let auth_path = f.root.path().join(".codex/auth.json");
    std::fs::write(&auth_path, auth.to_string()).unwrap();
    let mut request = f.request(true, ClaudeActivation::Switch);
    request.profile.preserve_login = true;
    assert_eq!(
        f.client.codex_provider_save("one", &request, TOKEN).await.unwrap().live,
        CodexLiveResult::Applied
    );
    assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&auth_path).unwrap()).unwrap(), auth);
    request.profile.preserve_login = false;
    assert_eq!(
        f.client.codex_provider_save("one", &request, TOKEN).await.unwrap().live,
        CodexLiveResult::Applied
    );
    assert!(!auth_path.exists());
    let stash: Value = serde_json::from_slice(
        &std::fs::read(f.root.path().join(".codex/shiju-login-stash.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(stash["logins"]["sub:alice|account:team"], auth);
    let before = std::fs::read(f.file()).unwrap();
    std::fs::write(&auth_path, b"invalid-json").unwrap();
    let result = f
        .client
        .codex_provider_save("two", &f.request(false, ClaudeActivation::Switch), TOKEN)
        .await
        .unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, CodexLiveResult::Failed);
    assert_eq!(f.state()["pointer"]["id"], "one");
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert_eq!(std::fs::read(&auth_path).unwrap(), b"invalid-json");
}
#[tokio::test]
async fn cc04_codex_catalog_sdk_api_recovers_marked_catalog_and_pointer_before_replanning() {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use std::os::unix::fs::MetadataExt;
    use vw_agent_apps::providers::live::commit::*;
    let f = Fixture::new().await;
    f.client
        .codex_provider_save("one", &f.request(true, ClaudeActivation::Switch), TOKEN)
        .await
        .unwrap();
    let directory = f.root.path().join(".codex");
    let meta = std::fs::metadata(&directory).unwrap();
    let mut changes = Vec::new();
    for (index, name) in ["config.toml", "shiju-model-catalog.json"].into_iter().enumerate() {
        let before = std::fs::read(directory.join(name)).unwrap();
        let staged = format!(".shiju-live-recovery-{index}");
        std::fs::write(directory.join(&staged), &before).unwrap();
        std::fs::set_permissions(directory.join(&staged), std::fs::Permissions::from_mode(0o600))
            .unwrap();
        let digest = URL_SAFE_NO_PAD.encode(Sha256::digest(&before));
        std::fs::remove_file(directory.join(name)).unwrap();
        changes.push(PendingFile {
            directory: 0,
            name: name.as_bytes().to_vec(),
            staged: staged.into_bytes(),
            pre: None,
            planned: Some(digest),
        });
    }
    let pending = Pending {
        version: 1,
        host: ExecutionHostId::new("host").unwrap(),
        app: vw_api_types::agent_apps::AppId::Codex,
        tenant: TenantScope::new("p", "a").unwrap(),
        directory_device: meta.dev(),
        directory_inode: meta.ino(),
        target: PendingTarget { pointer: Some("one".into()), mode: None, ..Default::default() },
        files: changes,
        published: true,
    };
    let path = f.root.path().join("private/provider-live/aG9zdA/codex.pending.json");
    std::fs::write(&path, serde_json::to_vec(&pending).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let result = f.client.codex_provider_switch("one", &f.target(), TOKEN).await.unwrap();
    assert_eq!(result.live, CodexLiveResult::RecoveryRequired);
    assert!(f.file().exists());
    assert!(directory.join("shiju-model-catalog.json").exists());
    assert!(!path.exists());
    assert_eq!(f.state()["pointer"]["id"], "one");
    assert_eq!(
        f.client.codex_provider_switch("one", &f.target(), TOKEN).await.unwrap().live,
        CodexLiveResult::Applied
    );
}

#[tokio::test]
async fn cc04_codex_catalog_missing_write_grant_and_managed_rows_are_rejected() {
    let f = Fixture::new().await;
    let path = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(&path, policy.to_string()).unwrap();
    let before = std::fs::read(f.file()).unwrap();
    assert!(
        f.client
            .codex_provider_save("denied", &f.request(true, ClaudeActivation::Switch), TOKEN)
            .await
            .is_err()
    );
    assert!(f.record("denied").is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert_eq!(std::fs::read_dir(f.root.path().join(".codex")).unwrap().count(), 1);
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(true);
    std::fs::write(&path, policy.to_string()).unwrap();
    f.client
        .codex_provider_save("one", &f.request(true, ClaudeActivation::CatalogOnly), TOKEN)
        .await
        .unwrap();
    let settings = GatewaySettingsStore::new(f.db.clone()).unwrap();
    let mut row: Value = serde_json::from_str(&f.record("one").unwrap()).unwrap();
    row["meta"] = json!({"authBinding":{"accountId":"fake-unresolved-account"}});
    settings
        .upsert(
            &TenantScope::new("p", "a").unwrap(),
            "app_providers.records",
            "\"codex\":one",
            &row.to_string(),
        )
        .unwrap();
    assert!(f.client.codex_provider_switch("one", &f.target(), TOKEN).await.is_err());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert_eq!(std::fs::read_dir(f.root.path().join(".codex")).unwrap().count(), 1);
}
