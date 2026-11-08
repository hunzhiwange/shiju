use crate::{
    app_providers::AppProviderStore, gateway::middleware::tenant_scope_middleware,
    global_database::GlobalDatabase, project,
};
use axum::{Extension, Json, Router, http::HeaderMap, middleware, routing::get};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, sync::Arc, time::Duration};
use tokio::sync::Notify;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        AppProviderWrite, ProviderProtocol, agy::AgyProfile, opencode::OpenCodeProfile,
    },
    id::{TaskId, TenantScope},
    model_catalog::*,
};
use vw_gateway_client::{AgentAppsReadError, GatewayClient, GatewayEndpoint};

const TOKEN: &str = "model-discovery-application-credential";
const OTHER_TOKEN: &str = "model-discovery-different-principal";
const SECRET: &str = "fixture-provider-credential-value";
const WAIT: Duration = Duration::from_secs(5);

struct Fixture {
    root: tempfile::TempDir,
    server: tokio::task::JoinHandle<()>,
    upstream: tokio::task::JoinHandle<()>,
    endpoint: GatewayEndpoint,
    project: ModelCatalogProject,
    input: ModelDiscoveryRequest,
    reached: Arc<Notify>,
    release: Arc<Notify>,
    replied: Arc<Notify>,
    database: GlobalDatabase,
    tenant: TenantScope,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
        self.upstream.abort();
    }
}

impl Fixture {
    async fn new(block: bool, malformed: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().canonicalize().unwrap();
        let dir = home.join("project");
        fs::create_dir(&dir).unwrap();
        let bin = home.join("trusted-bin");
        fs::create_dir(&bin).unwrap();
        let project = ModelCatalogProject {
            project_id: "registered".into(),
            directory: dir.to_str().unwrap().into(),
        };
        let database = GlobalDatabase::for_home(&home).unwrap();
        let tenant = TenantScope::new("platform", "company").unwrap();
        let other_tenant = TenantScope::new("platform", "other-company").unwrap();
        let projects = project::store::ProjectStore::new(database.clone()).unwrap();
        for (scope, id) in
            [(&tenant, "registered"), (&tenant, "other-project"), (&other_tenant, "registered")]
        {
            projects
                .create(
                    scope,
                    &project::Info {
                        id: id.into(),
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
        }
        let policy = home.join("access.json");
        let grant = |company: &str, host: &str| {
            json!({
                "platform_id":"platform", "company_id":company, "host_id":host, "provider_config_read":true
            })
        };
        let host = |id: &str| {
            json!({
                "id":id, "online":true, "binding":"local", "home":home,
                "search_paths":[bin], "applications":home.join("Applications")
            })
        };
        fs::write(&policy, json!({
            "principals": [
                {"id":"owner", "token_sha256":hex::encode(Sha256::digest(TOKEN)),
                 "grants":[grant("company","mac"),grant("company","other-mac"),grant("other-company","mac")]},
                {"id":"other", "token_sha256":hex::encode(Sha256::digest(OTHER_TOKEN)),
                 "grants":[grant("company","mac")]}
            ],
            "hosts":[host("mac"),host("other-mac")]
        }).to_string()).unwrap();
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();

        let reached = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let replied = Arc::new(Notify::new());
        let upstream_app = Router::new().route(
            "/v1/models",
            get({
                let (reached, release, replied) =
                    (reached.clone(), release.clone(), replied.clone());
                move |headers: HeaderMap| {
                    let (reached, release, replied) =
                        (reached.clone(), release.clone(), replied.clone());
                    async move {
                        assert_eq!(headers["authorization"], format!("Bearer {SECRET}"));
                        reached.notify_one();
                        // The upstream attempts to deliver a result even when the
                        // cancelled client has already closed its connection.
                        tokio::spawn(async move {
                            if block {
                                release.notified().await;
                            }
                            replied.notify_one();
                            Json(if malformed {
                                json!({"rawUpstreamDetail": SECRET})
                            } else {
                                json!({"data":[{"id":"vendor/Exact:free"}]})
                            })
                        })
                        .await
                        .unwrap()
                    }
                }
            }),
        );
        let upstream_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}/v1", upstream_listener.local_addr().unwrap());
        let upstream =
            tokio::spawn(
                async move { axum::serve(upstream_listener, upstream_app).await.unwrap() },
            );
        AppProviderStore::new(database.clone())
            .unwrap()
            .save(
                &tenant,
                AppId::Codex,
                "saved",
                &AppProviderWrite {
                    name: "Saved".into(),
                    protocol: ProviderProtocol::OpenaiResponses,
                    api_key: SECRET.into(),
                    base_url: base_url.clone(),
                    model: "current".into(),
                    request_options: None,
                },
            )
            .unwrap();
        let router = Router::new()
            .nest("/v1", super::model_catalog::router_with_policy(Some(policy)))
            .layer(Extension(database.clone()))
            .layer(middleware::from_fn(tenant_scope_middleware));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
            .with_tenant_scope(tenant.clone());
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        Self {
            root,
            server,
            upstream,
            endpoint,
            project,
            reached,
            release,
            replied,
            database,
            tenant,
            input: ModelDiscoveryRequest {
                deployment: ModelCatalogIdentity {
                    target_host_id: "mac".into(),
                    app_id: AppId::Codex,
                    provider_id: "saved".into(),
                    endpoint: base_url,
                },
                method: ModelDiscoveryMethod::Http,
                account_id: None,
            },
        }
    }

    fn client(&self) -> GatewayClient {
        GatewayClient::new(self.endpoint.clone()).unwrap()
    }

    async fn start(&self) -> ModelDiscoveryStatus {
        self.client().start_model_discovery(&self.project, &self.input, TOKEN).await.unwrap()
    }

    async fn read(&self, id: &TaskId) -> ModelDiscoveryStatus {
        self.client().model_discovery(&self.project, "mac", id, TOKEN).await.unwrap()
    }

    async fn cancel(&self, id: &TaskId) -> ModelDiscoveryStatus {
        self.client().cancel_model_discovery(&self.project, "mac", id, TOKEN).await.unwrap()
    }

    async fn terminal(&self, id: &TaskId) -> ModelDiscoveryStatus {
        tokio::time::timeout(WAIT, async {
            loop {
                let status = self.read(id).await;
                if matches!(
                    status.state,
                    ModelDiscoveryState::Completed { .. }
                        | ModelDiscoveryState::Failed { .. }
                        | ModelDiscoveryState::Cancelled
                ) {
                    break status;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap()
    }

    fn cli(&self, name: &str, script: &str) {
        let path = self.root.path().join("trusted-bin").join(name);
        fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }

    fn opencode(&mut self) {
        self.input.deployment.app_id = AppId::OpenCode;
        self.input.method = ModelDiscoveryMethod::OpenCode;
        AppProviderStore::new(self.database.clone()).unwrap().opencode_save(
            &self.tenant, "saved",
            &OpenCodeProfile {
                name: "Saved".into(),
                format: Default::default(),
                declaration: json!({"npm":"@ai-sdk/openai-compatible","options":{"baseURL":self.input.deployment.endpoint}}),
            },
            None, None,
        ).unwrap();
    }
}

fn assert_http(result: Result<ModelDiscoveryStatus, AgentAppsReadError>, expected: u16) {
    assert_eq!(
        result.unwrap_err(),
        AgentAppsReadError::Http { status: expected, body: serde_json::Value::Null }
    );
}

fn model_ids(status: ModelDiscoveryStatus) -> Vec<String> {
    let ModelDiscoveryState::Completed { models } = status.state else {
        panic!("unexpected terminal state: {:?}", status.state);
    };
    models
        .into_iter()
        .map(|row| {
            assert_eq!(row.identity.deployment, status.deployment);
            assert_eq!(row.identity.scope, TenantScope::new("platform", "company").unwrap());
            row.identity.model_id.as_ref().to_owned()
        })
        .collect()
}

#[tokio::test]
async fn catalog_api_discovery_cancel_prevents_result_publication() {
    let f = Fixture::new(true, false).await;
    let created = f.start().await;
    assert_eq!(created.state, ModelDiscoveryState::Queued);
    tokio::time::timeout(WAIT, f.reached.notified()).await.unwrap();
    assert_eq!(f.read(&created.task_id).await.state, ModelDiscoveryState::Discovering);
    assert_eq!(f.cancel(&created.task_id).await.state, ModelDiscoveryState::Cancelled);
    f.release.notify_one();
    tokio::time::timeout(WAIT, f.replied.notified()).await.unwrap();
    assert_eq!(f.read(&created.task_id).await.state, ModelDiscoveryState::Cancelled);
    assert_eq!(f.cancel(&created.task_id).await.state, ModelDiscoveryState::Cancelled);
}

#[tokio::test]
async fn catalog_api_discovery_http_completes_without_saving_models() {
    let f = Fixture::new(false, false).await;
    let created = f.start().await;
    let done = f.terminal(&created.task_id).await;
    assert_eq!(model_ids(done.clone()), ["vendor/Exact:free"]);
    assert!(!serde_json::to_string(&done).unwrap().contains(SECRET));
    assert_eq!(f.cancel(&created.task_id).await, done);
    assert!(
        AppProviderStore::new(f.database.clone())
            .unwrap()
            .read_model_catalog(&f.tenant, &f.input.deployment)
            .unwrap()
            .items
            .is_empty()
    );
}

#[tokio::test]
async fn catalog_api_discovery_http_uses_the_selected_native_codex_provider() {
    let f = Fixture::new(false, false).await;
    let store = AppProviderStore::new(f.database.clone()).unwrap();
    let before = store.codex_record(&f.tenant, "saved").unwrap().unwrap();
    let profile = vw_api_types::app_providers::codex_catalog::CodexProfile {
        name: "Saved".into(),
        config: format!(
            "model_provider = \"native\"\n[model_providers.native]\nname = \"Native\"\nbase_url = \"{}\"\nwire_api = \"responses\"\n",
            f.input.deployment.endpoint
        ),
        api_key: Some(SECRET.into()),
        preserve_login: false,
    };
    store.codex_save(&f.tenant, "saved", &profile, Some(&before), None).unwrap();
    let created = f.start().await;
    assert_eq!(model_ids(f.terminal(&created.task_id).await), ["vendor/Exact:free"]);
}

#[tokio::test]
async fn catalog_api_discovery_keeps_failure_details_private() {
    let f = Fixture::new(false, true).await;
    let created = f.start().await;
    let done = f.terminal(&created.task_id).await;
    assert_eq!(
        done.state,
        ModelDiscoveryState::Failed { reason: ModelDiscoveryFailure::InvalidResponse }
    );
    assert!(!serde_json::to_string(&done).unwrap().contains(SECRET));
}

#[tokio::test]
async fn catalog_api_discovery_task_id_does_not_grant_ownership() {
    let f = Fixture::new(true, false).await;
    let created = f.start().await;
    let client = f.client();
    assert_http(
        client.model_discovery(&f.project, "mac", &created.task_id, OTHER_TOKEN).await,
        404,
    );
    assert_http(
        client.cancel_model_discovery(&f.project, "other-mac", &created.task_id, TOKEN).await,
        404,
    );
    let other = ModelCatalogProject { project_id: "other-project".into(), ..f.project.clone() };
    assert_http(client.cancel_model_discovery(&other, "mac", &created.task_id, TOKEN).await, 404);
    let other = GatewayClient::new(
        f.endpoint
            .clone()
            .with_tenant_scope(TenantScope::new("platform", "other-company").unwrap()),
    )
    .unwrap();
    assert_http(other.model_discovery(&f.project, "mac", &created.task_id, TOKEN).await, 404);
    assert_eq!(f.cancel(&created.task_id).await.state, ModelDiscoveryState::Cancelled);
}

#[tokio::test]
async fn catalog_api_discovery_rejects_unbound_input_before_network() {
    let mut f = Fixture::new(false, false).await;
    f.input.deployment.endpoint.push_str("/forged");
    assert_http(f.client().start_model_discovery(&f.project, &f.input, TOKEN).await, 403);
    f.input.deployment.target_host_id = "ungranted".into();
    assert_http(f.client().start_model_discovery(&f.project, &f.input, TOKEN).await, 403);
    assert!(tokio::time::timeout(Duration::from_millis(50), f.reached.notified()).await.is_err());
}

#[tokio::test]
async fn catalog_api_discovery_rejects_commands_and_mismatched_methods() {
    let mut f = Fixture::new(false, false).await;
    let mut injected = serde_json::to_value(&f.input).unwrap();
    injected["command"] = json!("/bin/sh");
    let response = reqwest::Client::new()
        .post(format!("{}/v1/model-catalog/discoveries", f.endpoint.base_url()))
        .query(&f.project)
        .query(&[("targetHostId", "mac")])
        .header("x-agent-apps-token", TOKEN)
        .header("x-platform-id", "platform")
        .header("x-company-id", "company")
        .json(&injected)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    f.input.method = ModelDiscoveryMethod::OpenCode;
    assert_http(f.client().start_model_discovery(&f.project, &f.input, TOKEN).await, 422);
    assert!(tokio::time::timeout(Duration::from_millis(50), f.reached.notified()).await.is_err());
}

#[tokio::test]
async fn catalog_api_discovery_opencode_preserves_qualified_model_ids() {
    let mut f = Fixture::new(false, false).await;
    f.opencode();
    f.cli(
        "opencode",
        r#"
[ "$#" = 1 ] && [ "$1" = models ] && [ "$OPENCODE_DISABLE_PROJECT_CONFIG" = true ] || exit 12
printf '%s\n' 'zen/vendor/模型' 'zen/vendor/模型' 'bad row'
"#,
    );
    let created = f.start().await;
    assert_eq!(model_ids(f.terminal(&created.task_id).await), ["zen/vendor/模型"]);
}

#[tokio::test]
async fn catalog_api_discovery_agy_uses_saved_profile_and_native_display_name() {
    let mut f = Fixture::new(false, false).await;
    f.input.deployment.app_id = AppId::Agy;
    f.input.method = ModelDiscoveryMethod::Agy;
    AppProviderStore::new(f.database.clone())
        .unwrap()
        .agy_save(
            &f.tenant,
            "saved",
            &AgyProfile {
                name: "Saved".into(),
                model: None,
                api_key: Some(SECRET.into()),
                base_url: Some(f.input.deployment.endpoint.clone()),
            },
        )
        .unwrap();
    f.cli(
        "agy",
        &format!(
            r#"
[ "$#" = 2 ] && [ "$2" = models ] && [ "$GEMINI_API_KEY" = "{SECRET}" ] || exit 12
printf 'native-alias\t模型 Gemini\n'
"#
        ),
    );
    let created = f.start().await;
    assert_eq!(model_ids(f.terminal(&created.task_id).await), ["模型 Gemini"]);
}

#[tokio::test]
async fn catalog_api_discovery_cancel_waits_for_cli_exit() {
    let mut f = Fixture::new(false, false).await;
    f.opencode();
    f.cli(
        "opencode",
        r#"
printf '%s\n' "$$" > "$HOME/started"
while :; do /bin/sleep 1; done
"#,
    );
    let created = f.start().await;
    let pid = wait_for_pid(&f.root.path().join("started")).await;
    assert_eq!(f.cancel(&created.task_id).await.state, ModelDiscoveryState::Cancelled);
    // SAFETY: signal zero probes the fixture's recorded PID without sending a signal.
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
}

async fn wait_for_pid(path: &Path) -> i32 {
    tokio::time::timeout(WAIT, async {
        loop {
            if let Ok(text) = fs::read_to_string(path)
                && let Ok(pid) = text.trim().parse()
            {
                break pid;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
