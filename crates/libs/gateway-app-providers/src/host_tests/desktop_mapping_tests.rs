use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Router, http::StatusCode, middleware, response::IntoResponse};
use gateway_app_provider::GatewayAppProviderClient;
use gateway_endpoint::{GatewayAuth, GatewayEndpoint};
use gateway_http::{AgentAppsReadError, GatewayTransport};
use gateway_pairing::pairing::PairingGuard;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, process::Command};
use vw_agent_apps::providers::{
    claude_desktop::ClaudeDesktopFiles,
    live::commit::{ExecutionHostId, PendingTarget, ProviderTransactions},
};
use vw_api_app_aggregation::{AggregationQuery, DesktopMappingView};
use vw_api_app_identity::AppId;
use vw_api_app_provider_profiles::{claude_desktop::*, desktop_mapping::*};
use vw_api_transport::{LOCAL_HOST_ID, ProjectScope, id::TenantScope};

const SKEY: &str = "fixture-claude-desktop-gateway-skey";
const WRONG_SKEY: &str = "wrong-gateway-skey";
const CHILD: &str = "SHIJU_DESKTOP_MAPPING_TEST_ROOT";

fn tenant() -> TenantScope {
    TenantScope::new("p", "a").unwrap()
}

fn run_in_isolated_process() -> bool {
    if std::env::var_os(CHILD).is_some() {
        return false;
    }
    let root = tempfile::tempdir().unwrap();
    let test_name = std::thread::current().name().unwrap().to_owned();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", &test_name, "--nocapture"])
        .current_dir(root.path())
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env(CHILD, root.path())
        .env("HOME", root.path())
        .env("CFFIXED_USER_HOME", root.path())
        .env("VIBEWINDOW_TEST_ROOT", root.path());
    for (var, name) in [
        ("CODEX_HOME", ".codex"),
        ("CLAUDE_CONFIG_DIR", ".claude"),
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_CACHE_HOME", "cache"),
        ("XDG_STATE_HOME", "state"),
        ("XDG_RUNTIME_DIR", "runtime"),
        ("TMPDIR", "tmp"),
    ] {
        let path = root.path().join(name);
        std::fs::create_dir(&path).unwrap();
        command.env(var, path);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success()
            && String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed;"),
        "isolated Desktop mapping test failed: {}
{}
{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    true
}

async fn require_gateway_pairing(
    Extension(guard): Extension<PairingGuard>,
    request: axum::extract::Request,
    next: middleware::Next,
) -> axum::response::Response {
    let key = gateway_types::auth::extract_auth_skey(request.headers()).unwrap_or_default();
    if !guard.is_authenticated(key) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    next.run(request).await
}

struct Fixture {
    root: PathBuf,
    db: GlobalDatabase,
    port: u16,
    client: GatewayAppProviderClient,
    server: tokio::task::JoinHandle<()>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

impl Fixture {
    async fn new() -> Self {
        let root = PathBuf::from(std::env::var_os(CHILD).unwrap());
        assert_eq!(std::env::var_os("VIBEWINDOW_TEST_ROOT").as_deref(), Some(root.as_os_str()));
        assert_eq!(agent_paths::paths().home, root);
        drop(ClaudeDesktopFiles::open(&root).unwrap());
        std::fs::write(
            root.join("Library/Application Support/Claude/claude_desktop_config.json"),
            br#"{"mcpServers":{"mine":{}}}"#,
        )
        .unwrap();
        let db = GlobalDatabase::for_home(&root).unwrap();
        let router = super::claude_desktop::router_with_policy(None)
            .merge(super::desktop_mapping::router_with_policy(None))
            .layer(Extension(db.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware))
            .layer(middleware::from_fn(require_gateway_pairing))
            .layer(Extension(PairingGuard::new(true, &[SKEY.into()])));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            axum::serve(listener, Router::new().nest("/v1", router)).await.unwrap()
        });
        let client = GatewayAppProviderClient::new(Self::transport(port, SKEY));
        Self { root, db, port, client, server }
    }

    fn transport(port: u16, key: &str) -> GatewayTransport {
        GatewayTransport::new(
            GatewayEndpoint::new("127.0.0.1", port)
                .with_tenant_scope(tenant())
                .with_auth(GatewayAuth { skey: Some(key.into()) }),
        )
        .unwrap()
    }

    fn unpaired_client(&self) -> GatewayAppProviderClient {
        GatewayAppProviderClient::new(Self::transport(self.port, WRONG_SKEY))
    }

    fn file(&self) -> PathBuf {
        self.root.join("Library/Application Support/Claude/claude_desktop_config.json")
    }

    fn target(&self) -> ClaudeDesktopTarget {
        ClaudeDesktopTarget { target_host_id: LOCAL_HOST_ID.into() }
    }

    fn credential_root(&self) -> PathBuf {
        let tenant = tenant();
        let scope =
            serde_json::to_vec(&(tenant.platform_id().as_ref(), tenant.company_id().as_ref()))
                .unwrap();
        agent_paths::paths().data.join("apps").join(hex::encode(Sha256::digest(scope)))
    }

    fn transaction(&self) -> Option<ProviderTransactions> {
        ProviderTransactions::open_existing(
            &self.credential_root(),
            ExecutionHostId::new(LOCAL_HOST_ID).unwrap(),
        )
        .unwrap()
    }

    async fn form(&self, key: &str, project: &str) -> reqwest::Response {
        let transport = Self::transport(self.port, key);
        let endpoint = transport.endpoint();
        let request = transport
            .dynamic_host_client()
            .post(format!("{}/v1/app-providers/claude-desktop/form/form/read", endpoint.base_url()))
            .json(&AggregationQuery {
                target_host_id: LOCAL_HOST_ID.into(),
                project_id: project.into(),
            });
        gateway_http::apply_request_context(request, endpoint).send().await.unwrap()
    }

    fn record(&self, id: &str) -> Option<String> {
        GatewaySettingsStore::new(self.db.clone())
            .unwrap()
            .get(&tenant(), "app_providers.records", &format!("\"claude-desktop\":{id}"))
            .unwrap()
    }
}

#[tokio::test]
async fn cc04_desktop_mapping_sdk_api_import_validates_and_never_activates() {
    if run_in_isolated_process() {
        return;
    }
    let f = Fixture::new().await;
    let tenant = tenant();
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
    assert!(matches!(
        f.unpaired_client()
            .claude_desktop_import_claude(&f.target(), WRONG_SKEY)
            .await
            .unwrap_err(),
        AgentAppsReadError::Http { status: 401, .. }
    ));
    assert!(f.record("one").is_none());
    assert_eq!(f.client.claude_desktop_import_claude(&f.target(), SKEY).await.unwrap().imported, 1);
    assert_eq!(f.client.claude_desktop_import_claude(&f.target(), SKEY).await.unwrap().imported, 0);
    assert!(f.record("official").is_none());
    let row: Value = serde_json::from_str(&f.record("one").unwrap()).unwrap();
    assert_eq!(row["meta"]["claudeDesktopModelRoutes"]["claude-sonnet-5"]["model"], "kimi-k2");
    let defaults = f.client.claude_desktop_default_routes(&f.target(), SKEY).await.unwrap();
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
    let mut request = DesktopMappingWrite { target_host_id: LOCAL_HOST_ID.into(), mapping };
    let result = f.client.claude_desktop_mapping_save("one", &request, SKEY).await.unwrap();
    assert!(result.catalog_saved);
    assert_eq!(result.live, ClaudeDesktopLiveResult::NotAttempted);
    let saved = f.record("one").unwrap();
    request.mapping.routes.values_mut().next().unwrap().model = "bad\nmodel".into();
    assert!(f.client.claude_desktop_mapping_save("one", &request, SKEY).await.is_err());
    assert_eq!(f.record("one").unwrap(), saved);
    assert!(f.client.claude_desktop_provider_switch("one", &f.target(), SKEY).await.is_err());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert!(f.transaction().is_none());
}
#[tokio::test]
async fn cc04_desktop_mapping_imported_native_profile_reaches_transactional_files() {
    if run_in_isolated_process() {
        return;
    }
    let f = Fixture::new().await;
    let tenant = tenant();
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    use vw_api_app_provider_base::claude::{ClaudeConfig, ClaudeProfile};
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
    assert_eq!(f.client.claude_desktop_import_claude(&f.target(), SKEY).await.unwrap().imported, 1);
    let result =
        f.client.claude_desktop_provider_switch("native", &f.target(), SKEY).await.unwrap();
    assert_eq!(result.live, ClaudeDesktopLiveResult::Applied);
    let profile = f.root.join(format!(
        "Library/Application Support/Claude-3p/configLibrary/{}.json",
        vw_agent_apps::providers::claude_desktop::PROFILE_ID
    ));
    let value: Value = serde_json::from_slice(&std::fs::read(profile).unwrap()).unwrap();
    assert_eq!(value["inferenceGatewayApiKey"], "fake-native");
    assert_eq!(value["inferenceGatewayBaseUrl"], "https://fixture.test");
    let state = f.transaction().unwrap().state(AppId::ClaudeDesktop).unwrap();
    let pointer = state.pointer.unwrap();
    assert_eq!(pointer.id, "native");
    assert_eq!(pointer.tenant, tenant);
    assert_eq!(store.list(&tenant, AppId::ClaudeDesktop).unwrap().len(), 1);
}
#[tokio::test]
async fn cc04_desktop_mapping_permissions_refuse_catalog_side_effects() {
    if run_in_isolated_process() {
        return;
    }
    let f = Fixture::new().await;
    let settings = GatewaySettingsStore::new(f.db.clone()).unwrap();
    let tenant = tenant();
    settings.upsert(&tenant,"app_providers.records","\"claude\":one",&json!({"name":"Fixture","settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"https://fixture.test","ANTHROPIC_AUTH_TOKEN":"fake","ANTHROPIC_MODEL":"claude-opus-5"}}}).to_string()).unwrap();
    let before = std::fs::read(f.file()).unwrap();
    assert_eq!(f.client.claude_desktop_default_routes(&f.target(), SKEY).await.unwrap().len(), 4);
    let rejected = f.unpaired_client();
    assert!(matches!(
        rejected.claude_desktop_import_claude(&f.target(), WRONG_SKEY).await.unwrap_err(),
        AgentAppsReadError::Http { status: 401, .. }
    ));
    let request = DesktopMappingWrite {
        target_host_id: LOCAL_HOST_ID.into(),
        mapping: DesktopMapping { mode: DesktopMappingMode::Direct, routes: Default::default() },
    };
    assert!(matches!(
        rejected.claude_desktop_mapping_save("one", &request, WRONG_SKEY).await.unwrap_err(),
        AgentAppsReadError::Http { status: 401, .. }
    ));
    assert!(f.record("one").is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert!(f.transaction().is_none());
}

#[tokio::test]
async fn cc04_ui_desktop_form_read_is_redacted_and_permission_checked() {
    if run_in_isolated_process() {
        return;
    }
    let f = Fixture::new().await;
    let write = ClaudeDesktopWrite {
        target_host_id: LOCAL_HOST_ID.into(),
        profile: ClaudeDesktopProfile {
            name: "中文直连".into(),
            config: ClaudeDesktopConfig::ThirdParty {
                base_url: "https://fixture.test".into(),
                auth_token: "fake-private-secret".into(),
            },
        },
        activation: ClaudeDesktopActivation::CatalogOnly,
    };
    f.client.claude_desktop_provider_save("form", &write, SKEY).await.unwrap();
    let before = f.record("form").unwrap();
    let project = ProjectScope::new(tenant(), "fixture-project".into()).unwrap();
    let tx = ProviderTransactions::new(
        &f.credential_root(),
        ExecutionHostId::new(LOCAL_HOST_ID).unwrap(),
    )
    .unwrap();
    {
        let mut files = ClaudeDesktopFiles::open(&f.root).unwrap();
        tx.commit_project_checked(
            &project,
            &mut files.guard,
            &tx.state(AppId::ClaudeDesktop).unwrap(),
            true,
            &[],
            PendingTarget::default(),
            || Ok(()),
        )
        .unwrap();
    }
    let state = tx.state(AppId::ClaudeDesktop).unwrap();
    let file_before = std::fs::read(f.file()).unwrap();
    let response = f.form(SKEY, "fixture-project").await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    assert!(!body.contains("fake-private-secret"));
    let view: DesktopMappingView = serde_json::from_str(&body).unwrap();
    assert!(!view.revision.is_empty());
    assert_eq!(view.form.name, "中文直连");
    assert!(view.form.direct_editable && view.form.mapping_editable);
    assert_eq!(view.form.mapping.mode, DesktopMappingMode::Direct);
    assert_eq!(f.form(WRONG_SKEY, "fixture-project").await.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(f.form(SKEY, "foreign-project").await.status(), StatusCode::FORBIDDEN);
    assert_eq!(f.record("form").unwrap(), before);
    let mut unsafe_row: Value = serde_json::from_str(&before).unwrap();
    unsafe_row["settingsConfig"]["env"]["ANTHROPIC_BASE_URL"] =
        json!("https://fixture.test?api_key=private");
    GatewaySettingsStore::new(f.db.clone())
        .unwrap()
        .upsert(
            &tenant(),
            "app_providers.records",
            "\"claude-desktop\":form",
            &unsafe_row.to_string(),
        )
        .unwrap();
    assert_eq!(f.form(SKEY, "fixture-project").await.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(tx.state(AppId::ClaudeDesktop).unwrap() == state);
    assert!(tx.pending(AppId::ClaudeDesktop).unwrap().is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), file_before);
}
