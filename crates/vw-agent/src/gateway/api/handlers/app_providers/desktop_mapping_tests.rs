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
            .merge(super::desktop_mapping::router_with_policy(Some(policy.clone())))
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

use vw_api_types::app_providers::desktop_mapping::*;
#[tokio::test]
async fn cc04_desktop_mapping_sdk_api_import_validates_and_never_activates() {
    let f = Fixture::new().await;
    let tenant = TenantScope::new("p", "a").unwrap();
    let settings = GatewaySettingsStore::new(f.db.clone()).unwrap();
    let source = json!({"name":"Fixture","settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"https://fixture.test","ANTHROPIC_AUTH_TOKEN":"fake-secret","ANTHROPIC_MODEL":"kimi-k2[1M]"}}});
    settings
        .upsert(&tenant, "app_providers.records", "\"claude\":one", &source.to_string())
        .unwrap();
    settings
        .upsert(
            &tenant,
            "app_providers.records",
            "\"claude\":official",
            &json!({"name":"Official","settingsConfig":{"env":{}}}).to_string(),
        )
        .unwrap();
    let before = std::fs::read(f.file()).unwrap();
    assert!(f.client.claude_desktop_import_claude(&f.target(), "bad-token").await.is_err());
    assert!(f.record("one").is_none());
    assert_eq!(
        f.client.claude_desktop_import_claude(&f.target(), TOKEN).await.unwrap().imported,
        1
    );
    assert_eq!(
        f.client.claude_desktop_import_claude(&f.target(), TOKEN).await.unwrap().imported,
        0
    );
    assert!(f.record("official").is_none());
    let row: Value = serde_json::from_str(&f.record("one").unwrap()).unwrap();
    assert_eq!(row["meta"]["claudeDesktopModelRoutes"]["claude-sonnet-5"]["model"], "kimi-k2");
    let defaults = f.client.claude_desktop_default_routes(&f.target(), TOKEN).await.unwrap();
    assert_eq!(defaults.len(), 4);
    let mapping = DesktopMapping {
        mode: DesktopMappingMode::Proxy,
        routes: [(
            "claude-opus-5".into(),
            DesktopModelRoute {
                model: "new-model".into(),
                label_override: None,
                supports_1m: false,
            },
        )]
        .into(),
    };
    let mut request = DesktopMappingWrite { target_host_id: "host".into(), mapping };
    let result = f.client.claude_desktop_mapping_save("one", &request, TOKEN).await.unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, ClaudeDesktopLiveResult::NotAttempted);
    let saved = f.record("one").unwrap();
    request.mapping.routes.values_mut().next().unwrap().model = "bad\nmodel".into();
    assert!(f.client.claude_desktop_mapping_save("one", &request, TOKEN).await.is_err());
    assert_eq!(f.record("one").unwrap(), saved);
    assert!(f.client.claude_desktop_provider_switch("one", &f.target(), TOKEN).await.is_err());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert!(!f.root.path().join("private/provider-live").exists());
}
#[tokio::test]
async fn cc04_desktop_mapping_imported_native_profile_reaches_transactional_files() {
    let f = Fixture::new().await;
    let tenant = TenantScope::new("p", "a").unwrap();
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    use vw_api_types::app_providers::claude::{ClaudeConfig, ClaudeProfile};
    store
        .claude_save(
            &tenant,
            "native",
            &ClaudeProfile {
                name: "Native".into(),
                config: ClaudeConfig::ThirdParty {
                    base_url: "https://fixture.test".into(),
                    api_key: "fake-native".into(),
                    api_key_header: false,
                },
                model: Some("claude-sonnet-5".into()),
            },
            None,
            true,
        )
        .unwrap();
    assert_eq!(
        f.client.claude_desktop_import_claude(&f.target(), TOKEN).await.unwrap().imported,
        1
    );
    let result =
        f.client.claude_desktop_provider_switch("native", &f.target(), TOKEN).await.unwrap();
    assert_eq!(result.live, ClaudeDesktopLiveResult::Applied);
    let profile = f.root.path().join(format!(
        "Library/Application Support/Claude-3p/configLibrary/{}.json",
        vw_agent_apps::providers::claude_desktop::PROFILE_ID
    ));
    let value: Value = serde_json::from_slice(&std::fs::read(profile).unwrap()).unwrap();
    assert_eq!(value["inferenceGatewayApiKey"], "fake-native");
    assert_eq!(value["inferenceGatewayBaseUrl"], "https://fixture.test");
    assert_eq!(f.state()["pointer"]["id"], "native");
    assert_eq!(
        store.list(&tenant, vw_api_types::agent_apps::AppId::ClaudeDesktop).unwrap().len(),
        1
    );
}
#[tokio::test]
async fn cc04_desktop_mapping_permissions_refuse_catalog_side_effects() {
    let f = Fixture::new().await;
    let settings = GatewaySettingsStore::new(f.db.clone()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    settings.upsert(&tenant,"app_providers.records","\"claude\":one",&json!({"name":"Fixture","settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"https://fixture.test","ANTHROPIC_AUTH_TOKEN":"fake","ANTHROPIC_MODEL":"claude-opus-5"}}}).to_string()).unwrap();
    let before = std::fs::read(f.file()).unwrap();
    let policy_path = f.root.path().join("access.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&policy_path).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(&policy_path, policy.to_string()).unwrap();
    assert_eq!(f.client.claude_desktop_default_routes(&f.target(), TOKEN).await.unwrap().len(), 4);
    let err = f.client.claude_desktop_import_claude(&f.target(), TOKEN).await.unwrap_err();
    assert!(matches!(err, vw_gateway_client::AgentAppsReadError::Http { status: 403, .. }));
    let request = DesktopMappingWrite {
        target_host_id: "host".into(),
        mapping: DesktopMapping { mode: DesktopMappingMode::Direct, routes: Default::default() },
    };
    assert!(f.client.claude_desktop_mapping_save("one", &request, TOKEN).await.is_err());
    assert!(f.record("one").is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert!(!f.root.path().join("private/provider-live").exists());
}

#[tokio::test]
async fn cc04_ui_desktop_form_read_is_redacted_and_permission_checked() {
    let f = Fixture::new().await;
    let write = ClaudeDesktopWrite {
        target_host_id: "host".into(),
        profile: ClaudeDesktopProfile {
            name: "中文直连".into(),
            config: ClaudeDesktopConfig::ThirdParty {
                base_url: "https://fixture.test".into(),
                auth_token: "fake-private-secret".into(),
            },
        },
        activation: ClaudeDesktopActivation::CatalogOnly,
    };
    f.client.claude_desktop_provider_save("form", &write, TOKEN).await.unwrap();
    let before = f.record("form").unwrap();
    let view = f.client.claude_desktop_form_read("form", &f.target(), TOKEN).await.unwrap();
    assert_eq!(view.name, "中文直连");
    assert!(view.direct_editable && view.mapping_editable);
    assert_eq!(view.mapping.mode, DesktopMappingMode::Direct);
    assert!(!serde_json::to_string(&view).unwrap().contains("fake-private-secret"));
    assert!(f.client.claude_desktop_form_read("form", &f.target(), "bad-token").await.is_err());
    assert_eq!(f.record("form").unwrap(), before);
    let mut unsafe_row: Value = serde_json::from_str(&before).unwrap();
    unsafe_row["settingsConfig"]["env"]["ANTHROPIC_BASE_URL"] =
        json!("https://fixture.test?api_key=private");
    GatewaySettingsStore::new(f.db.clone())
        .unwrap()
        .upsert(
            &TenantScope::new("p", "a").unwrap(),
            "app_providers.records",
            "\"claude-desktop\":form",
            &unsafe_row.to_string(),
        )
        .unwrap();
    assert!(matches!(
        f.client.claude_desktop_form_read("form", &f.target(), TOKEN).await.unwrap_err(),
        vw_gateway_client::AgentAppsReadError::Http { status: 422, .. }
    ));
    assert!(!f.root.path().join("private/provider-live").exists());
}
