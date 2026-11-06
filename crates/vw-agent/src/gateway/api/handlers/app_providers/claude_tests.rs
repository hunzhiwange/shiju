use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware, routing::post};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};
use vw_api_types::{agent_apps::AppId, app_providers::claude::*, id::TenantScope};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-claude-token-with-at-least-32-chars";
struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    client: GatewayClient,
    server: tokio::task::JoinHandle<()>,
    policy: PathBuf,
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
        std::fs::create_dir(root.path().join(".claude")).unwrap();
        std::fs::write(root.path().join(".claude/settings.json"),br#"{ "hooks": {"x":true}, "env":{"USER_FLAG":"yes","AWS_SECRET_ACCESS_KEY":"old-secret","ANTHROPIC_AUTH_TOKEN":"old-key"}}"#).unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"claude","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let router = Router::new()
            .merge(super::claude::router_with_policy(Some(policy.clone())))
            .route(
                "/app-providers/{app}/{id}",
                post(super::create::create).put(super::save).patch(super::update::update),
            )
            .layer(Extension(super::claude::Policy(Some(policy.clone()))))
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
        Self { root, db, client, server, policy }
    }
    fn file(&self) -> PathBuf {
        self.root.path().join(".claude/settings.json")
    }
    fn live(&self) -> Value {
        serde_json::from_slice(&std::fs::read(self.file()).unwrap()).unwrap()
    }
    fn request(&self, config: ClaudeConfig, activation: ClaudeActivation) -> ClaudeWrite {
        ClaudeWrite {
            target_host_id: "host".into(),
            claude: ClaudeProfile { name: "Fixture".into(), config, model: None },
            activation,
        }
    }
    fn target(&self) -> ClaudeTarget {
        ClaudeTarget { target_host_id: "host".into() }
    }
    fn state(&self) -> Value {
        serde_json::from_slice(
            &std::fs::read(self.root.path().join("private/provider-live/aG9zdA/claude.state.json"))
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
                &format!("\"claude\":{id}"),
            )
            .unwrap()
    }
}
#[tokio::test]
async fn cc04_claude_sdk_api_switches_four_variants_and_cleans_credentials() {
    let f = Fixture::new().await;
    let variants = [
        (
            "third",
            ClaudeConfig::ThirdParty {
                base_url: "https://fixture.example".into(),
                api_key: "fake-third-key".into(),
                api_key_header: false,
            },
        ),
        (
            "bedrock",
            ClaudeConfig::Bedrock {
                region: "us-west-2".into(),
                auth: BedrockAuth::Keys {
                    access_key_id: "fake-id".into(),
                    secret_access_key: "fake-secret".into(),
                    session_token: Some("fake-session".into()),
                },
            },
        ),
        (
            "vertex",
            ClaudeConfig::Vertex {
                project: "fixture-project".into(),
                region: "us-east5".into(),
                credentials_file: Some("/fake/never-opened.json".into()),
            },
        ),
        ("official", ClaudeConfig::Official {}),
    ];
    for (index, (id, config)) in variants.into_iter().enumerate() {
        let activation =
            if index == 0 { ClaudeActivation::First } else { ClaudeActivation::CatalogOnly };
        let result = f
            .client
            .claude_provider_create(id, &f.request(config.clone(), activation), TOKEN)
            .await
            .unwrap();
        assert!(result.catalog_saved);
        if index > 0 {
            assert_eq!(result.live, ClaudeLiveResult::NotAttempted);
            assert_eq!(
                f.client.claude_provider_switch(id, &f.target(), TOKEN).await.unwrap().live,
                ClaudeLiveResult::Applied
            );
        } else {
            assert_eq!(result.live, ClaudeLiveResult::Applied);
        }
        let expected = vw_agent_apps::providers::claude::settings(&ClaudeProfile {
            name: "Fixture".into(),
            config,
            model: None,
        })
        .unwrap();
        assert_eq!(
            f.live()["env"]
                .as_object()
                .unwrap()
                .iter()
                .filter(|(k, _)| k.as_str() != "USER_FLAG")
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<serde_json::Map<_, _>>(),
            expected["env"].as_object().unwrap().clone()
        );
        assert_eq!(f.state()["pointer"]["id"], id);
        assert_eq!(f.live()["hooks"], json!({"x":true}));
        assert_eq!(std::fs::metadata(f.file()).unwrap().permissions().mode() & 0o777, 0o600);
    }
    assert!(
        crate::app_providers::AppProviderStore::new(f.db.clone())
            .unwrap()
            .list(&TenantScope::new("p", "a").unwrap(), AppId::Claude)
            .unwrap()
            .len()
            == 4
    );
}
#[tokio::test]
async fn cc04_claude_current_edit_reapply_import_and_inactive_edit_share_adapter() {
    let f = Fixture::new().await;
    let official = f.request(ClaudeConfig::Official {}, ClaudeActivation::First);
    assert_eq!(
        f.client.claude_provider_create("saved", &official, TOKEN).await.unwrap().live,
        ClaudeLiveResult::Applied
    );
    let third = f.request(
        ClaudeConfig::ThirdParty {
            base_url: "https://new.example".into(),
            api_key: "fake-new".into(),
            api_key_header: true,
        },
        ClaudeActivation::IfCurrent,
    );
    assert_eq!(
        f.client.claude_provider_update("saved", &third, TOKEN).await.unwrap().live,
        ClaudeLiveResult::Applied
    );
    assert_eq!(f.live()["env"]["ANTHROPIC_API_KEY"], "fake-new");
    let bytes = std::fs::read(f.file()).unwrap();
    assert_eq!(
        f.client.claude_provider_save("inactive", &third, TOKEN).await.unwrap().live,
        ClaudeLiveResult::NotAttempted
    );
    assert_eq!(std::fs::read(f.file()).unwrap(), bytes);
    let imported = f
        .client
        .claude_provider_import(
            "imported",
            &ClaudeImport { target_host_id: "host".into(), name: "Imported".into() },
            TOKEN,
        )
        .await
        .unwrap();
    assert_eq!(imported.live, ClaudeLiveResult::NotAttempted);
    assert_eq!(std::fs::read(f.file()).unwrap(), bytes);
    assert!(f.record("imported").is_some());
    std::fs::write(f.file(), br#"{"outside":true,"env":{"AWS_SECRET_ACCESS_KEY":"stray"}}"#)
        .unwrap();
    assert_eq!(
        f.client.claude_provider_reapply("saved", &f.target(), TOKEN).await.unwrap().live,
        ClaudeLiveResult::Applied
    );
    assert_eq!(
        f.live()["env"],
        json!({"ANTHROPIC_BASE_URL":"https://new.example","ANTHROPIC_API_KEY":"fake-new"})
    );
    assert_eq!(f.live()["outside"], true);
}
#[tokio::test]
async fn cc04_claude_denied_and_invalid_requests_leave_database_and_files_untouched() {
    let f = Fixture::new().await;
    let before = std::fs::read(f.file()).unwrap();
    let request = f.request(ClaudeConfig::Official {}, ClaudeActivation::First);
    assert!(f.client.claude_provider_create("denied", &request, "wrong-token").await.is_err());
    assert!(f.record("denied").is_none());
    let mut invalid = request.clone();
    invalid.claude.name.clear();
    assert!(f.client.claude_provider_create("invalid", &invalid, TOKEN).await.is_err());
    assert!(f.record("invalid").is_none());
    let mut wrong_host = request.clone();
    wrong_host.target_host_id = "other-host".into();
    assert!(f.client.claude_provider_create("wrong-host", &wrong_host, TOKEN).await.is_err());
    assert!(f.record("wrong-host").is_none());
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(&f.policy, policy.to_string()).unwrap();
    assert!(f.client.claude_provider_create("denied", &request, TOKEN).await.is_err());
    assert!(f.record("denied").is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert!(!f.root.path().join("private/provider-live").exists());
}
#[tokio::test]
async fn cc04_claude_sync_projects_existing_universal_owner_and_applies_current_child() {
    use vw_api_types::app_providers::{create::ProviderSecretWrite, universal::*};
    let f = Fixture::new().await;
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    let tenant = TenantScope::new("p", "a").unwrap();
    let mut write = UniversalProviderWrite {
        config: UniversalConfig {
            name: "Parent".into(),
            provider_type: "custom".into(),
            apps: UniversalApps { claude: true, ..Default::default() },
            base_url: "https://parent.example".into(),
            models: Default::default(),
            website_url: None,
            notes: None,
            icon: None,
            icon_color: None,
            sort_index: None,
        },
        api_key: ProviderSecretWrite::Replace { value: "fake-parent-key".into() },
    };
    store.universal_save(&tenant, "parent", &write).unwrap();
    let mut request =
        ClaudeSync { target_host_id: "host".into(), activation: ClaudeActivation::CatalogOnly };
    let before = std::fs::read(f.file()).unwrap();
    assert_eq!(
        f.client.claude_provider_sync("parent", &request, TOKEN).await.unwrap().live,
        ClaudeLiveResult::NotAttempted
    );
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert_eq!(
        f.client
            .claude_provider_switch("universal-claude-parent", &f.target(), TOKEN)
            .await
            .unwrap()
            .live,
        ClaudeLiveResult::Applied
    );
    let edit = f.request(
        ClaudeConfig::Bedrock {
            region: "us-west-2".into(),
            auth: BedrockAuth::Bearer { token: "fake-child-cloud".into() },
        },
        ClaudeActivation::IfCurrent,
    );
    assert_eq!(
        f.client
            .claude_provider_update("universal-claude-parent", &edit, TOKEN)
            .await
            .unwrap()
            .live,
        ClaudeLiveResult::Applied
    );
    write.api_key = ProviderSecretWrite::Replace { value: "fake-updated-key".into() };
    store.universal_save(&tenant, "parent", &write).unwrap();
    request.activation = ClaudeActivation::IfCurrent;
    let result = f.client.claude_provider_sync("parent", &request, TOKEN).await.unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, ClaudeLiveResult::Applied);
    assert_eq!(f.live()["env"]["ANTHROPIC_AUTH_TOKEN"], "fake-updated-key");
    assert!(f.live()["env"].get("AWS_BEARER_TOKEN_BEDROCK").is_none());
    assert!(f.live()["env"].get("CLAUDE_CODE_USE_BEDROCK").is_none());
    assert_eq!(f.state()["pointer"]["id"], "universal-claude-parent");
    assert!(f.record("universal-codex-parent").is_none());
}
#[tokio::test]
async fn cc04_claude_live_failure_reports_saved_catalog_and_reapply_repairs_it() {
    let f = Fixture::new().await;
    let request = f.request(ClaudeConfig::Official {}, ClaudeActivation::First);
    assert_eq!(
        f.client.claude_provider_create("saved", &request, TOKEN).await.unwrap().live,
        ClaudeLiveResult::Applied
    );
    std::fs::write(f.file(), b"invalid configuration").unwrap();
    let request = f.request(
        ClaudeConfig::ThirdParty {
            base_url: "https://retry.example".into(),
            api_key: "fake-retry".into(),
            api_key_header: false,
        },
        ClaudeActivation::IfCurrent,
    );
    let result = f.client.claude_provider_update("saved", &request, TOKEN).await.unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, ClaudeLiveResult::Failed);
    assert_eq!(std::fs::read(f.file()).unwrap(), b"invalid configuration");
    assert_eq!(f.state()["pointer"]["id"], "saved");
    std::fs::write(f.file(), b"{}").unwrap();
    assert_eq!(
        f.client.claude_provider_reapply("saved", &f.target(), TOKEN).await.unwrap().live,
        ClaudeLiveResult::Applied
    );
    assert_eq!(f.live()["env"]["ANTHROPIC_AUTH_TOKEN"], "fake-retry");
}
#[tokio::test]
async fn cc04_claude_legacy_path_first_activation_and_symlink_rejection() {
    let f = Fixture::new().await;
    std::fs::rename(f.file(), f.root.path().join(".claude/claude.json")).unwrap();
    let request = f.request(ClaudeConfig::Official {}, ClaudeActivation::First);
    assert_eq!(
        f.client.claude_provider_create("legacy", &request, TOKEN).await.unwrap().live,
        ClaudeLiveResult::Applied
    );
    assert!(!f.file().exists());
    assert!(f.root.path().join(".claude/claude.json").exists());
    let original = f.root.path().join(".claude/claude.json");
    let before = std::fs::read(&original).unwrap();
    std::os::unix::fs::symlink(&original, f.file()).unwrap();
    assert!(f.client.claude_provider_switch("legacy", &f.target(), TOKEN).await.is_err());
    assert_eq!(std::fs::read(original).unwrap(), before);
}
#[tokio::test]
async fn cc04_claude_registered_route_requires_gateway_authentication() {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    let app = crate::gateway::router::handler_router()
        .layer(Extension(TenantScope::new("p", "a").unwrap()))
        .layer(middleware::from_fn(|req, next| async {
            crate::gateway::middleware::basic_auth_middleware_with_credentials(
                req,
                next,
                Some("secret"),
                "test",
            )
            .await
        }));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/app-providers/claude/saved/switch")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"targetHostId":"host"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
}
#[tokio::test]
async fn cc04_claude_first_activation_creates_missing_config_and_catalog_defaults_do_not() {
    let f = Fixture::new().await;
    std::fs::remove_dir_all(f.root.path().join(".claude")).unwrap();
    assert!(f.client.claude_provider_switch("missing", &f.target(), TOKEN).await.is_err());
    assert!(!f.root.path().join(".claude").exists());
    assert!(!f.root.path().join("private/provider-live").exists());
    let mut request = f.request(
        ClaudeConfig::Bedrock {
            region: "us-west-2".into(),
            auth: BedrockAuth::Bearer { token: "fake-bearer".into() },
        },
        ClaudeActivation::CatalogOnly,
    );
    assert_eq!(
        f.client.claude_provider_create("saved", &request, TOKEN).await.unwrap().live,
        ClaudeLiveResult::NotAttempted
    );
    assert!(!f.file().exists());
    assert!(!f.root.path().join("private/provider-live").exists());
    request.activation = ClaudeActivation::First;
    assert_eq!(
        f.client.claude_provider_save("saved", &request, TOKEN).await.unwrap().live,
        ClaudeLiveResult::Applied
    );
    assert_eq!(f.live()["env"]["AWS_BEARER_TOKEN_BEDROCK"], "fake-bearer");
}
#[tokio::test]
async fn cc04_claude_committed_exclusives_survive_catalog_edits_until_switch_cleans_them() {
    let f = Fixture::new().await;
    let tenant = TenantScope::new("p", "a").unwrap();
    let store = GatewaySettingsStore::new(f.db.clone()).unwrap();
    // Existing saved rows are consumed by the same projection adapter, not a second storage owner.
    store.upsert(&tenant,"app_providers.records","\"claude\":legacy",&json!({"name":"Legacy","settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"https://legacy.example","ANTHROPIC_AUTH_TOKEN":"fake-legacy","CLAUDE_CODE_DISABLE_ARTIFACT":"1"}}}).to_string()).unwrap();
    assert_eq!(
        f.client.claude_provider_switch("legacy", &f.target(), TOKEN).await.unwrap().live,
        ClaudeLiveResult::Applied
    );
    assert_eq!(f.live()["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"], "1");
    let request = f.request(ClaudeConfig::Official {}, ClaudeActivation::CatalogOnly);
    f.client.claude_provider_save("legacy", &request, TOKEN).await.unwrap();
    assert_eq!(
        f.client.claude_provider_reapply("legacy", &f.target(), TOKEN).await.unwrap().live,
        ClaudeLiveResult::Applied
    );
    assert!(f.live()["env"].get("CLAUDE_CODE_DISABLE_ARTIFACT").is_none());
}
