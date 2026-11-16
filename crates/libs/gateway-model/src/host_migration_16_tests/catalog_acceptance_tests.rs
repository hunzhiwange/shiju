//! Cross the SDK, SQLite, resolver and execution boundaries on macOS.
use gateway_model_catalog::GatewayModelCatalogClient;

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use axum::{
    Extension, Json, Router,
    routing::get,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use vw_api_app_identity::AppId;
use vw_api_app_provider_base::AppProviderWrite;
use vw_api_app_provider_base::ProviderProtocol;
use vw_api_transport::id::ModelId;
use vw_api_transport::id::TenantScope;
use vw_api_model_catalog::*;
use gateway_endpoint::{GatewayAuth, GatewayEndpoint};
use gateway_http::AgentAppsReadError;

use app_provider_services::AppProviderStore;
use agent_database::GlobalDatabase;
use agent_project as project;


const CHILD: &str = "VW_MACOS_MODEL_CATALOG_DOMAIN_ACCEPTANCE";
const TOKEN: &str = "model-catalog-macos-acceptance-app-token";
const PROFILE: &str = "catalog-profile";
const HOST: &str = "mac";
const MODEL: &str = "Vendor/Exact[1m]";
const ALIAS: &str = "部署展示名";
const WAIT: Duration = Duration::from_secs(10);

// Global configuration, the database, and the models.dev cache share process state.
async fn isolated(test: &str) -> bool {
    if std::env::var_os(CHILD).is_some() {
        return false;
    }
    let root = tempfile::tempdir().unwrap();
    let home = root.path().canonicalize().unwrap();
    let (_, module) = module_path!().split_once("::").unwrap();
    let output = tokio::time::timeout(
        Duration::from_secs(60),
        tokio::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &format!("{module}::{test}"), "--nocapture"])
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", &home)
            .env("TMPDIR", &home)
            .env("VIBEWINDOW_CONFIG_DIR", home.join("config"))
            .env("VW_MODEL_RECOVERY_TEST_ROOT", &home)
            .env("VIBEWINDOW_TEST_ROOT", &home)
            .env("VW_RECOVERY_WINDOW_HTTP", "1")
            .env("VIBEWINDOW_DISABLE_MODELS_FETCH", "1")
            .env("SHIJU_AGENT_APPS_ACCESS_FILE", home.join("access.json"))
            .env(CHILD, "1")
            .current_dir(&home)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .expect("macOS catalog acceptance timed out")
    .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

struct Loopback {
    endpoint: String,
    task: tokio::task::JoinHandle<()>,
}

impl Loopback {
    async fn serve(router: Router) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        Self { endpoint, task }
    }
}

impl Drop for Loopback {
    fn drop(&mut self) {
        self.task.abort();
    }
}

struct Fixture {
    tenant: TenantScope,
    project: ModelCatalogProject,
    deployment: ModelCatalogIdentity,
    server: Loopback,
    _upstream: Loopback,
    hold_discovery: Arc<AtomicBool>,
    discovery_started: Arc<tokio::sync::Notify>,
}

impl Fixture {
    async fn new() -> Self {
        let home = std::env::current_dir().unwrap();
        let tenant = TenantScope::new("acceptance", "company").unwrap();
        let directory = home.join("project");
        fs::create_dir(&directory).unwrap();
        let project = ModelCatalogProject {
            project_id: "registered".into(),
            directory: directory.to_str().unwrap().into(),
        };
        let db = GlobalDatabase::for_home(&agent_paths::paths().home).unwrap();
        project::store::ProjectStore::new(db.clone())
            .unwrap()
            .create(
                &tenant,
                &project::Info {
                    id: project.project_id.0.clone(),
                    worktree: project.directory.clone(),
                    vcs: None,
                    name: None,
                    icon: None,
                    commands: None,
                    time: project::TimeInfo { created: 1, updated: 1, initialized: None },
                    sandboxes: vec![],
                },
            )
            .unwrap();
        let policy = home.join("access.json");
        write_json(
            &policy,
            json!({
                "principals":[{"id":"editor","token_sha256":hex::encode(Sha256::digest(TOKEN)),
                    "grants":[{"platform_id":"acceptance","company_id":"company","host_id":HOST,
                        "provider_config_read":true,"provider_config_write":true}]}],
                "hosts":[{"id":HOST,"online":true,"binding":"local","home":home,
                    "search_paths":[],"applications":home.join("Applications")}]
            }),
        );
        fs::set_permissions(policy, fs::Permissions::from_mode(0o600)).unwrap();

        let hold_discovery = Arc::new(AtomicBool::new(false));
        let discovery_started = Arc::new(tokio::sync::Notify::new());
        let hold = hold_discovery.clone();
        let started = discovery_started.clone();
        let upstream = Loopback::serve(Router::new()
            .route("/v1/models", get(move || async move {
                started.notify_one();
                if hold.load(Ordering::Relaxed) {
                    std::future::pending::<()>().await;
                }
                Json(json!({"data":[{"id":MODEL},{"id":"Unselected"}]}))
            }))).await;
        let deployment = ModelCatalogIdentity {
            target_host_id: HOST.into(),
            app_id: AppId::Codex,
            provider_id: PROFILE.into(),
            endpoint: format!("{}/v1", upstream.endpoint),
        };
        AppProviderStore::new(db.clone())
            .unwrap()
            .save(
                &tenant,
                AppId::Codex,
                PROFILE,
                &AppProviderWrite {
                    name: "Fixture".into(),
                    protocol: ProviderProtocol::OpenaiResponses,
                    api_key: "private-fixture-credential".into(),
                    base_url: deployment.endpoint.clone(),
                    model: "old".into(),
                    request_options: None,
                },
            )
            .unwrap();
        let cache = provider_runtime_paths::paths().cache.join("models.json");
        assert!(cache.starts_with(&home), "the models.dev fixture must stay inside its own home");
        write_json(
            &cache,
            json!({"fixture":{
                "id":"fixture","name":"Fixture","api":deployment.endpoint,"env":[],
                "models":{(MODEL):{"id":MODEL,"name":"Fixture","family":"fixture",
                    "release_date":"2026-01-01","attachment":false,"reasoning":false,
                    "tool_call":true,"temperature":false,"modalities":{"input":["text"],"output":["text"]},
                    "limit":{"context":64000,"output":4096},"cost":{"input":1,"output":2}}}
            }}),
        );
        let server =
            Loopback::serve(Router::new().nest("/v1", super::model_catalog::router_with_policy(Some(home.join("access.json")))).layer(axum::middleware::from_fn(agent_gateway::middleware::tenant_scope_middleware)).layer(Extension(db))).await;
        Self {
            tenant,
            project,
            deployment,
            server,
            _upstream: upstream,
            hold_discovery,
            discovery_started,
        }
    }

    fn endpoint(&self) -> GatewayEndpoint {
        let port = self.server.endpoint.rsplit_once(':').unwrap().1.parse().unwrap();
        GatewayEndpoint::new("127.0.0.1", port)
            .with_auth(GatewayAuth { skey: Some("dynamic-acceptance-skey".into()) })
            .with_tenant_scope(self.tenant.clone())
    }

    fn client(&self) -> GatewayModelCatalogClient {
        GatewayModelCatalogClient::new(gateway_http::GatewayTransport::new(self.endpoint()).unwrap())
    }

    async fn read(&self, deployment: &ModelCatalogIdentity) -> ModelCatalogReadResponse {
        self.client()
            .model_catalog(
                &self.project,
                &ModelCatalogQuery { deployment: deployment.clone(), search: None, page: None },
                TOKEN,
            )
            .await
            .unwrap()
    }

    async fn discovery(&self) -> ModelDiscoveryStatus {
        self.client()
            .start_model_discovery(
                &self.project,
                &ModelDiscoveryRequest {
                    deployment: self.deployment.clone(),
                    method: ModelDiscoveryMethod::Http,
                    account_id: None,
                },
                TOKEN,
            )
            .await
            .unwrap()
    }

    async fn selected(
        &self,
        deployment: &ModelCatalogIdentity,
        model: ModelId,
    ) -> ModelCatalogEntry {
        let before = self.read(deployment).await;
        let saved = self
            .client()
            .edit_model_catalog(
                &self.project,
                &ModelCatalogEditRequest {
                    deployment: deployment.clone(),
                    expected_revision: before.revision,
                    edits: vec![ModelCatalogEdit::Upsert {
                        model_id: model.clone(),
                        display_alias: Some(ALIAS.into()),
                        metadata: Box::new(ModelMetadataDto {
                            context_window: Some(24_000),
                            ..Default::default()
                        }),
                    }],
                    default_change: Some(ModelCatalogDefaultChange::Set {
                        model_id: model.clone(),
                    }),
                },
                TOKEN,
            )
            .await
            .unwrap();
        let read = self.read(deployment).await;
        assert!(saved.changed);
        assert_eq!(read.revision, saved.revision);
        assert_eq!(read.default_model_id.as_ref(), Some(&model));
        read.items.into_iter().find(|row| row.identity.model_id == model).unwrap()
    }


}

fn write_json(path: &Path, value: Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, value.to_string()).unwrap();
}

#[tokio::test]
async fn catalog_macos_acceptance_scopes_preset_selection_and_price() {
    if isolated("catalog_macos_acceptance_scopes_preset_selection_and_price").await {
        return;
    }
    let f = Fixture::new().await;
    let hits = f
        .client()
        .model_catalog_presets(&f.project, HOST, AppId::Codex, Some("月之暗面"), None, TOKEN)
        .await
        .unwrap();
    let endpoint = |host: &str| {
        hits.items.iter().find(|preset| preset.endpoint.as_deref() == Some(host)).unwrap()
    };
    let china = endpoint("https://api.moonshot.cn/v1");
    let global = endpoint("https://api.moonshot.ai/v1");
    let same = china.model_ids.iter().find(|id| global.model_ids.contains(id)).unwrap();
    let deployment =
        ModelCatalogIdentity { endpoint: china.endpoint.clone().unwrap(), ..f.deployment.clone() };
    f.selected(&deployment, same.clone()).await;
    let other = ModelCatalogIdentity { endpoint: global.endpoint.clone().unwrap(), ..deployment };
    assert!(f.read(&other).await.items.is_empty());
    assert!(f.read(&f.deployment).await.items.is_empty());

    let saved = f.selected(&f.deployment, MODEL.into()).await;
    let price = f
        .client()
        .preview_model_catalog_fill(
            &f.project,
            &ModelCatalogFillPreviewRequest { identity: saved.identity.clone() },
            TOKEN,
        )
        .await
        .unwrap();
    assert_eq!(price.proposed.cost.as_ref().unwrap().input, 1.0);
    let saved = f.selected(&other, MODEL.into()).await;
    let foreign = f
        .client()
        .preview_model_catalog_fill(
            &f.project,
            &ModelCatalogFillPreviewRequest { identity: saved.identity },
            TOKEN,
        )
        .await
        .unwrap();
    assert!(foreign.proposed.cost.is_none(), "price crossed deployment boundaries");
}

#[tokio::test]
async fn catalog_macos_acceptance_rejects_stale_or_unauthorized_fill() {
    if isolated("catalog_macos_acceptance_rejects_stale_or_unauthorized_fill").await {
        return;
    }
    let f = Fixture::new().await;
    let saved = f.selected(&f.deployment, MODEL.into()).await;
    let request = ModelCatalogFillPreviewRequest { identity: saved.identity.clone() };
    let preview = f.client().preview_model_catalog_fill(&f.project, &request, TOKEN).await.unwrap();
    let changed = f
        .client()
        .set_model_catalog_default(
            &f.project,
            &ModelCatalogDefaultRequest {
                deployment: f.deployment.clone(),
                expected_revision: preview.revision.clone(),
                change: ModelCatalogDefaultChange::Clear,
            },
            TOKEN,
        )
        .await
        .unwrap();
    let mut commit = ModelCatalogFillCommitRequest {
        identity: preview.identity,
        expected_revision: preview.revision,
    };
    let result = f.client().commit_model_catalog_fill(&f.project, &commit, TOKEN).await;
    assert!(matches!(result, Err(AgentAppsReadError::Http { status: 409, .. })));
    let before = f.read(&f.deployment).await;
    commit.identity.deployment.target_host_id = "ungranted".into();
    commit.expected_revision = changed.revision;
    let result = f.client().commit_model_catalog_fill(&f.project, &commit, TOKEN).await;
    assert!(matches!(result, Err(AgentAppsReadError::Http { status: 403, .. })));
    assert_eq!(f.read(&f.deployment).await, before);
    assert_eq!(before.items, [saved]);
}

#[tokio::test]
async fn catalog_macos_acceptance_cancel_keeps_saved_catalog() {
    if isolated("catalog_macos_acceptance_cancel_keeps_saved_catalog").await {
        return;
    }
    let f = Fixture::new().await;
    f.selected(&f.deployment, MODEL.into()).await;
    let before = f.read(&f.deployment).await;
    f.hold_discovery.store(true, Ordering::Relaxed);
    let task = f.discovery().await;
    tokio::time::timeout(WAIT, f.discovery_started.notified()).await.unwrap();
    let cancelled =
        f.client().cancel_model_discovery(&f.project, HOST, &task.task_id, TOKEN).await.unwrap();
    assert_eq!(cancelled.state, ModelDiscoveryState::Cancelled);
    let terminal =
        f.client().model_discovery(&f.project, HOST, &task.task_id, TOKEN).await.unwrap();
    assert_eq!(terminal.state, ModelDiscoveryState::Cancelled);
    assert!(!serde_json::to_string(&terminal).unwrap().contains("private-fixture-credential"));
    assert_eq!(f.read(&f.deployment).await, before);
}
