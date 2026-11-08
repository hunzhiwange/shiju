//! Cross the SDK, SQLite, resolver and execution boundaries on macOS.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use axum::{
    Extension, Json, Router,
    routing::{get, post},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderWrite, ProviderProtocol},
    id::{ModelId, TenantScope},
    model_catalog::*,
};
use vw_gateway_client::{
    AgentAppsReadError, GatewayAuth, GatewayChatStreamEvent, GatewayChatStreamRequest,
    GatewayClient, GatewayEndpoint,
};

use crate::{
    app_providers::AppProviderStore, config, gateway, global_database::GlobalDatabase, project,
    provider::provider, session::ui_store::with_tenant_scope,
};

const CHILD: &str = "VW_MACOS_MODEL_CATALOG_ACCEPTANCE";
const TOKEN: &str = "model-catalog-macos-acceptance-app-token";
const RUNTIME_KEY: &str = "acceptance-runtime-key";
const PROVIDER: &str = "catalog-acceptance";
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
            .env("VW_RECOVERY_WINDOW_HTTP", "1")
            .env("VIBEWINDOW_DISABLE_MODELS_FETCH", "1")
            .env("SHIJU_AGENT_APPS_ACCESS_FILE", home.join("access.json"))
            .env("VW_CATALOG_ACCEPTANCE_KEY", RUNTIME_KEY)
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
    database_path: PathBuf,
    server: Loopback,
    _upstream: Loopback,
    requests: Arc<Mutex<Vec<Value>>>,
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
        let db = GlobalDatabase::for_home(&crate::global::paths().home).unwrap();
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

        let requests = Arc::new(Mutex::new(Vec::new()));
        let hold_discovery = Arc::new(AtomicBool::new(false));
        let discovery_started = Arc::new(tokio::sync::Notify::new());
        let observed = requests.clone();
        let hold = hold_discovery.clone();
        let started = discovery_started.clone();
        let upstream = Loopback::serve(Router::new()
            .route("/v1/models", get(move || async move {
                started.notify_one();
                if hold.load(Ordering::Relaxed) {
                    std::future::pending::<()>().await;
                }
                Json(json!({"data":[{"id":MODEL},{"id":"Unselected"}]}))
            }))
            .route("/v1/chat/completions", post(move |Json(body): Json<Value>| async move {
                observed.lock().unwrap().push(body);
                (
                    [("content-type", "text/event-stream")],
                    concat!(
                        "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ready\"},\"finish_reason\":null}]}\n\n",
                        "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
                        "data: [DONE]\n\n"
                    ),
                )
            }))).await;
        let deployment = ModelCatalogIdentity {
            target_host_id: HOST.into(),
            app_id: AppId::Codex,
            provider_id: PROFILE.into(),
            endpoint: format!("{}/v1", upstream.endpoint),
        };
        let database_path = db.sqlite_path().unwrap().to_path_buf();
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
        config::update_global_for_tenant(
            &tenant,
            json!({"providers": {(PROVIDER): {
                "api":deployment.endpoint,"adapter":"openai-compatible",
                "env":["VW_CATALOG_ACCEPTANCE_KEY"],"model_catalog":deployment,
            }}}),
        )
        .await
        .unwrap();
        let cache = vw_provider_resolver::global::paths().cache.join("models.json");
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
            Loopback::serve(gateway::dynamic_tools_acceptance_router().layer(Extension(db))).await;
        Self {
            tenant,
            project,
            deployment,
            database_path,
            server,
            _upstream: upstream,
            requests,
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

    fn client(&self) -> GatewayClient {
        GatewayClient::new(self.endpoint()).unwrap()
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

    async fn send(&self, content: Value) -> Vec<GatewayChatStreamEvent> {
        let mut events = Vec::new();
        let body = GatewayChatStreamRequest {
            model: Some(format!("{PROVIDER}/{MODEL}")),
            messages: vec![json!({"role":"user","content":content})],
            allowed_tools: Some(vec![]),
            ..Default::default()
        };
        tokio::time::timeout(
            WAIT,
            GatewayClient::stream_chat(
                &self.endpoint(),
                Some(&self.project.directory),
                &body,
                |event| {
                    events.push(event);
                    true
                },
            ),
        )
        .await
        .unwrap()
        .unwrap();
        events
    }
}

fn write_json(path: &Path, value: Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, value.to_string()).unwrap();
}

#[tokio::test]
async fn catalog_macos_acceptance_persists_and_enforces_identity() {
    if isolated("catalog_macos_acceptance_persists_and_enforces_identity").await {
        return;
    }
    let mut f = Fixture::new().await;
    let started = f.discovery().await;
    let models = tokio::time::timeout(WAIT, async {
        loop {
            let status = f
                .client()
                .model_discovery(&f.project, HOST, &started.task_id, TOKEN)
                .await
                .unwrap();
            match status.state {
                ModelDiscoveryState::Completed { models } => break models,
                ModelDiscoveryState::Failed { .. } | ModelDiscoveryState::Cancelled => {
                    panic!("discovery failed: {status:?}")
                }
                _ => tokio::time::sleep(Duration::from_millis(20)).await,
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(models.len(), 2);
    assert!(f.read(&f.deployment).await.items.is_empty());
    let selected = models.into_iter().find(|row| row.identity.model_id.as_ref() == MODEL).unwrap();
    assert_eq!(selected.identity.deployment, f.deployment);
    let saved = f.selected(&selected.identity.deployment, selected.identity.model_id).await;
    let preview = f
        .client()
        .preview_model_catalog_fill(
            &f.project,
            &ModelCatalogFillPreviewRequest { identity: saved.identity.clone() },
            TOKEN,
        )
        .await
        .unwrap();
    assert_eq!(preview.identity, saved.identity);
    assert_eq!(preview.proposed.context_window, None);
    assert_eq!(preview.proposed.max_output_tokens, Some(4096));
    assert_eq!(
        preview.proposed.provenance[&ModelMetadataField::MaxOutputTokens].source,
        ModelMetadataSource::ModelsDev
    );
    let written = f
        .client()
        .commit_model_catalog_fill(
            &f.project,
            &ModelCatalogFillCommitRequest {
                identity: preview.identity,
                expected_revision: preview.revision,
            },
            TOKEN,
        )
        .await
        .unwrap();
    assert!(written.changed);

    f.server.task.abort();
    let _ = (&mut f.server.task).await;
    let db = GlobalDatabase::new(&f.database_path).unwrap();
    f.server =
        Loopback::serve(gateway::dynamic_tools_acceptance_router().layer(Extension(db))).await;
    let read = f.read(&f.deployment).await;
    assert_eq!(read.revision, written.revision);
    assert_eq!(read.items.len(), 1);
    assert_eq!(read.items[0].identity, saved.identity);
    assert_eq!(read.items[0].display_alias.as_deref(), Some(ALIAS));
    assert_eq!(read.items[0].metadata.context_window, Some(24_000));
    assert_eq!(read.default_model_id, Some(MODEL.into()));
    with_tenant_scope(f.tenant.clone(), async {
        let resolved = provider::get_model(PROVIDER, MODEL).await.unwrap();
        assert_eq!(resolved.api.id, MODEL);
        assert_eq!(resolved.api.url, f.deployment.endpoint);
        assert_eq!(resolved.limit.output, 4096);
        assert!(!resolved.capabilities.input.image);
    })
    .await;
    let errors = f
        .send(json!([
            {"type":"text","text":"Read image"},
            {"type":"image","mimeType":"image/png","data":"aW1hZ2U="}
        ]))
        .await;
    assert!(
        errors.iter().any(|event| matches!(event, GatewayChatStreamEvent::Error(error)
        if error.contains(MODEL) && error.contains("does not support image input"))),
        "{errors:?}"
    );
    assert!(f.requests.lock().unwrap().is_empty(), "image rejection reached the upstream");

    let response = f.send(json!("Read text")).await;
    assert!(
        response.iter().any(|event| matches!(event, GatewayChatStreamEvent::Delta(delta)
        if delta == "ready")),
        "{response:?}"
    );
    let requests = f.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["model"], MODEL);
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
