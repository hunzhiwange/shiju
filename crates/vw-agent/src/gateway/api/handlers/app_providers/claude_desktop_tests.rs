use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{app_providers::claude_desktop::*, id::TenantScope};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-claude_desktop-token-with-at-least-32-chars";
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
        drop(
            vw_agent_apps::providers::claude_desktop::ClaudeDesktopFiles::open(root.path())
                .unwrap(),
        );
        std::fs::write(
            root.path().join("Library/Application Support/Claude/claude_desktop_config.json"),
            b"{\"mcpServers\":{\"mine\":{}}}",
        )
        .unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"claude_desktop","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let router = super::claude_desktop::router_with_policy(Some(policy.clone()))
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
        self.root.path().join("Library/Application Support/Claude/claude_desktop_config.json")
    }
    fn request(&self, official: bool, activation: ClaudeDesktopActivation) -> ClaudeDesktopWrite {
        ClaudeDesktopWrite {
            target_host_id: "host".into(),
            profile: ClaudeDesktopProfile {
                name: "Fixture".into(),
                config: if official {
                    ClaudeDesktopConfig::Official {}
                } else {
                    ClaudeDesktopConfig::ThirdParty {
                        base_url: "https://fixture.test".into(),
                        auth_token: "fake-secret".into(),
                    }
                },
            },
            activation,
        }
    }
    fn target(&self) -> ClaudeDesktopTarget {
        ClaudeDesktopTarget { target_host_id: "host".into() }
    }
    fn state(&self) -> Value {
        serde_json::from_slice(
            &std::fs::read(
                self.root.path().join("private/provider-live/aG9zdA/claude-desktop.state.json"),
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
                &format!("\"claude-desktop\":{id}"),
            )
            .unwrap()
    }
}

#[tokio::test]
async fn cc04_claude_desktop_sdk_api_switch_edit_reapply_and_official() {
    let f = Fixture::new().await;
    let r = f
        .client
        .claude_desktop_provider_save(
            "one",
            &f.request(false, ClaudeDesktopActivation::First),
            TOKEN,
        )
        .await
        .unwrap();
    assert!(r.catalog_saved);
    assert_eq!(r.live, ClaudeDesktopLiveResult::Applied);
    let profile = f.root.path().join(format!(
        "Library/Application Support/Claude-3p/configLibrary/{}.json",
        vw_agent_apps::providers::claude_desktop::PROFILE_ID
    ));
    let read = || serde_json::from_slice::<Value>(&std::fs::read(&profile).unwrap()).unwrap();
    let mut user = read();
    user["userAutoMode"] = json!(true);
    std::fs::write(&profile, user.to_string()).unwrap();
    let mut edit = f.request(false, ClaudeDesktopActivation::IfCurrent);
    if let ClaudeDesktopConfig::ThirdParty { auth_token, .. } = &mut edit.profile.config {
        *auth_token = "fake-edited".into();
    }
    assert_eq!(
        f.client.claude_desktop_provider_save("one", &edit, TOKEN).await.unwrap().live,
        ClaudeDesktopLiveResult::Applied
    );
    assert_eq!(read()["userAutoMode"], true);
    assert_eq!(read()["inferenceGatewayApiKey"], "fake-edited");
    edit.activation = ClaudeDesktopActivation::CatalogOnly;
    if let ClaudeDesktopConfig::ThirdParty { auth_token, .. } = &mut edit.profile.config {
        *auth_token = "fake-final".into();
    }
    assert_eq!(
        f.client.claude_desktop_provider_save("one", &edit, TOKEN).await.unwrap().live,
        ClaudeDesktopLiveResult::NotAttempted
    );
    assert_eq!(read()["inferenceGatewayApiKey"], "fake-edited");
    assert_eq!(
        f.client.claude_desktop_provider_reapply("one", &f.target(), TOKEN).await.unwrap().live,
        ClaudeDesktopLiveResult::Applied
    );
    assert_eq!(read()["inferenceGatewayApiKey"], "fake-final");
    f.client
        .claude_desktop_provider_save(
            "official",
            &f.request(true, ClaudeDesktopActivation::CatalogOnly),
            TOKEN,
        )
        .await
        .unwrap();
    assert_eq!(
        f.client.claude_desktop_provider_switch("official", &f.target(), TOKEN).await.unwrap().live,
        ClaudeDesktopLiveResult::Applied
    );
    assert_eq!(read()["userAutoMode"], true);
    assert!(read().get("inferenceGatewayApiKey").is_none());
    assert_eq!(f.state()["pointer"]["id"], "official");
    let normal: Value = serde_json::from_slice(&std::fs::read(f.file()).unwrap()).unwrap();
    assert_eq!(normal["deploymentMode"], "1p");
    assert_eq!(normal["mcpServers"], json!({"mine":{}}));
}
#[tokio::test]
async fn cc04_claude_desktop_denial_and_generic_import_leave_files_and_catalog_untouched() {
    let f = Fixture::new().await;
    let before = std::fs::read(f.file()).unwrap();
    assert!(
        f.client
            .claude_desktop_provider_save(
                "denied",
                &f.request(false, ClaudeDesktopActivation::Switch),
                "bad-token"
            )
            .await
            .is_err()
    );
    assert!(f.record("denied").is_none());
    let error = f.client.claude_desktop_provider_import_live(&f.target(), TOKEN).await.unwrap_err();
    assert!(matches!(error, vw_gateway_client::AgentAppsReadError::Http { status: 422, .. }));
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert!(f.record("live").is_none());
    let path = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(path, policy.to_string()).unwrap();
    assert!(
        f.client
            .claude_desktop_provider_save(
                "denied",
                &f.request(false, ClaudeDesktopActivation::Switch),
                TOKEN
            )
            .await
            .is_err()
    );
    assert!(f.record("denied").is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
}
#[tokio::test]
async fn cc04_claude_desktop_activation_and_host_scope_are_enforced() {
    let f = Fixture::new().await;
    f.client
        .claude_desktop_provider_save(
            "one",
            &f.request(false, ClaudeDesktopActivation::First),
            TOKEN,
        )
        .await
        .unwrap();
    let before = std::fs::read(f.file()).unwrap();
    for activation in [ClaudeDesktopActivation::First, ClaudeDesktopActivation::IfCurrent] {
        assert_eq!(
            f.client
                .claude_desktop_provider_save("two", &f.request(true, activation), TOKEN)
                .await
                .unwrap()
                .live,
            ClaudeDesktopLiveResult::NotAttempted
        );
        assert_eq!(std::fs::read(f.file()).unwrap(), before);
    }
    let mut bad = f.request(true, ClaudeDesktopActivation::Switch);
    bad.target_host_id = "other".into();
    assert!(f.client.claude_desktop_provider_save("foreign", &bad, TOKEN).await.is_err());
    assert!(f.record("foreign").is_none());
    std::fs::write(f.file(), b"[]").unwrap();
    assert!(
        f.client
            .claude_desktop_provider_save(
                "invalid",
                &f.request(false, ClaudeDesktopActivation::Switch),
                TOKEN
            )
            .await
            .is_err()
    );
    assert!(f.record("invalid").is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), b"[]");
}
#[tokio::test]
async fn cc04_claude_desktop_mapping_and_managed_rows_require_routing_without_writes() {
    let f = Fixture::new().await;
    f.client
        .claude_desktop_provider_save(
            "one",
            &f.request(false, ClaudeDesktopActivation::CatalogOnly),
            TOKEN,
        )
        .await
        .unwrap();
    let before = std::fs::read(f.file()).unwrap();
    let original: Value = serde_json::from_str(&f.record("one").unwrap()).unwrap();
    for meta in [
        json!({"authBinding":{"accountId":"fake"}}),
        json!({"claudeDesktopMode":"proxy"}),
        json!({"apiFormat":"openai_chat"}),
        json!({"claudeDesktopModelRoutes":{"claude-sonnet-5":{"model":"other"}}}),
    ] {
        let mut row = original.clone();
        row["meta"] = meta;
        GatewaySettingsStore::new(f.db.clone())
            .unwrap()
            .upsert(
                &TenantScope::new("p", "a").unwrap(),
                "app_providers.records",
                "\"claude-desktop\":one",
                &row.to_string(),
            )
            .unwrap();
        assert!(f.client.claude_desktop_provider_switch("one", &f.target(), TOKEN).await.is_err());
        assert_eq!(std::fs::read(f.file()).unwrap(), before);
        assert_eq!(serde_json::from_str::<Value>(&f.record("one").unwrap()).unwrap(), row);
    }
}
