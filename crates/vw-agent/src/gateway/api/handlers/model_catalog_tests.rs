use crate::{
    app_providers::AppProviderStore, gateway::middleware::tenant_scope_middleware,
    global_database::GlobalDatabase, project,
};
use axum::{Extension, Router, middleware};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderWrite, ProviderProtocol},
    common::PaginationRequest,
    id::TenantScope,
    model_catalog::*,
};
use vw_gateway_client::{AgentAppsReadError, GatewayClient, GatewayEndpoint};

const TOKEN: &str = "model-catalog-test-application-credential";
const EDITOR_TOKEN: &str = "model-catalog-test-editor-credential";
const SECRET: &str = "fixture-provider-credential-value";

struct Fixture {
    _root: tempfile::TempDir,
    server: tokio::task::JoinHandle<()>,
    endpoint: GatewayEndpoint,
    db_path: std::path::PathBuf,
    project: ModelCatalogProject,
    query: ModelCatalogQuery,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

impl Fixture {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("project");
        std::fs::create_dir(&directory).unwrap();
        let directory = directory.to_str().unwrap().to_owned();
        let project = ModelCatalogProject { project_id: "registered".into(), directory };
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let tenant = TenantScope::new("platform", "company").unwrap();
        let projects = project::store::ProjectStore::new(db.clone()).unwrap();
        for (tenant, id, worktree) in [
            (&tenant, "registered", project.directory.clone()),
            (&tenant, "other-project", root.path().join("other").to_str().unwrap().to_owned()),
            (
                &TenantScope::new("platform", "other-company").unwrap(),
                "registered",
                project.directory.clone(),
            ),
        ] {
            projects
                .create(
                    tenant,
                    &project::Info {
                        id: id.into(),
                        worktree,
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
        let policy = root.path().join("access.json");
        std::fs::write(&policy, json!({
            "principals": [{"id":"editor", "token_sha256":hex::encode(Sha256::digest(EDITOR_TOKEN)),
                "grants":[{"platform_id":"platform","company_id":"company","host_id":"mac",
                    "provider_config_read":true,"provider_config_write":true}]},
                {"id":"reader", "token_sha256":hex::encode(Sha256::digest(TOKEN)),
                "grants":[{"platform_id":"platform","company_id":"company","host_id":"mac","provider_config_read":true},
                          {"platform_id":"platform","company_id":"other-company","host_id":"mac","provider_config_read":true}]}],
            "hosts":[{"id":"mac","online":true,"binding":"local","home":root.path(),
                "search_paths":[],"applications":root.path().join("Applications")}]
        }).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();

        let deployment = ModelCatalogIdentity {
            target_host_id: "mac".into(),
            app_id: AppId::Codex,
            provider_id: "fixture".into(),
            endpoint: "https://fixture.test/v1".into(),
        };
        let store = AppProviderStore::new(db.clone()).unwrap();
        store
            .save(
                &tenant,
                deployment.app_id,
                &deployment.provider_id,
                &AppProviderWrite {
                    name: "Fixture".into(),
                    protocol: ProviderProtocol::OpenaiResponses,
                    api_key: SECRET.into(),
                    base_url: deployment.endpoint.clone(),
                    model: "vendor/One:free".into(),
                    request_options: None,
                },
            )
            .unwrap();
        let revision = store.read_model_catalog(&tenant, &deployment).unwrap().revision;
        let metadata = ModelMetadataDto {
            context_window: Some(32768),
            input_modalities: Some(vec!["text".into()]),
            tool_calls: ModelCapabilityState::Unsupported,
            provenance: [(
                ModelMetadataField::ContextWindow,
                ModelMetadataProvenance { source: ModelMetadataSource::Manual, fetched_at: None },
            )]
            .into(),
            ..Default::default()
        };
        store
            .edit_model_catalog(
                &tenant,
                &ModelCatalogEditRequest {
                    deployment: deployment.clone(),
                    expected_revision: revision,
                    edits: ["vendor/One:free", "vendor/Two:free", "vendor/Three"]
                        .into_iter()
                        .map(|id| ModelCatalogEdit::Upsert {
                            model_id: id.into(),
                            display_alias: Some(format!("展示 {id}")),
                            metadata: Box::new(metadata.clone()),
                        })
                        .collect(),
                    default_change: Some(ModelCatalogDefaultChange::Set {
                        model_id: "vendor/One:free".into(),
                    }),
                },
            )
            .unwrap();

        let db_path = db.sqlite_path().unwrap().to_path_buf();
        let (server, endpoint) = Self::serve(db, policy, tenant).await;
        Self {
            _root: root,
            server,
            endpoint,
            db_path,
            project,
            query: ModelCatalogQuery { deployment, search: None, page: None },
        }
    }

    async fn serve(
        db: GlobalDatabase,
        policy: std::path::PathBuf,
        tenant: TenantScope,
    ) -> (tokio::task::JoinHandle<()>, GatewayEndpoint) {
        let router = Router::new()
            .nest("/v1", super::model_catalog::router_with_policy(Some(policy)))
            .layer(Extension(db))
            .layer(middleware::from_fn(tenant_scope_middleware));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = GatewayEndpoint::new("127.0.0.1", listener.local_addr().unwrap().port())
            .with_tenant_scope(tenant);
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        (server, endpoint)
    }

    fn client(&self) -> GatewayClient {
        GatewayClient::new(self.endpoint.clone()).unwrap()
    }
}

fn assert_http<T: std::fmt::Debug>(result: Result<T, AgentAppsReadError>, expected: u16) {
    assert_eq!(
        result.unwrap_err(),
        AgentAppsReadError::Http { status: expected, body: serde_json::Value::Null }
    );
}

#[tokio::test]
async fn catalog_api_read_denies_other_host() {
    let f = Fixture::new().await;
    let client = f.client();
    let mut other = f.query.clone();
    other.deployment.target_host_id = "other-host".into();
    assert_http(client.model_catalog(&f.project, &other, TOKEN).await, 403);
    let read = client.model_catalog(&f.project, &f.query, TOKEN).await.unwrap();
    assert_eq!(
        read.items[0].metadata.provenance[&ModelMetadataField::ContextWindow].source,
        ModelMetadataSource::Manual
    );
    assert!(!serde_json::to_string(&read).unwrap().contains(SECRET));
}

#[tokio::test]
async fn catalog_api_read_denies_other_project_or_tenant() {
    let f = Fixture::new().await;
    let mut project = f.project.clone();
    project.project_id = "other-project".into();
    assert_http(f.client().model_catalog(&project, &f.query, TOKEN).await, 403);
    let client = GatewayClient::new(
        f.endpoint
            .clone()
            .with_tenant_scope(TenantScope::new("platform", "other-company").unwrap()),
    )
    .unwrap();
    assert_http(client.model_catalog(&f.project, &f.query, TOKEN).await, 404);
}

#[tokio::test]
async fn catalog_api_read_searches_alias_and_pages_saved_models() {
    let mut f = Fixture::new().await;
    let client = f.client();
    f.query.search = Some("展示 VENDOR/".into());
    f.query.page = Some(PaginationRequest { cursor: None, limit: Some(2) });
    let first = client.model_catalog(&f.project, &f.query, TOKEN).await.unwrap();
    assert_eq!(first.items.len(), 2);
    assert_eq!(first.default_model_id.as_ref().unwrap().as_ref(), "vendor/One:free");
    f.query.page.as_mut().unwrap().cursor = Some(first.next_cursor.unwrap());
    let next = client.model_catalog(&f.project, &f.query, TOKEN).await.unwrap();
    assert_eq!(next.items.len(), 1);
    assert_ne!(first.items[0].identity, next.items[0].identity);
    assert_ne!(first.items[1].identity, next.items[0].identity);
    assert_eq!(next.revision, first.revision);
    assert!(next.next_cursor.is_none());
}

#[tokio::test]
async fn catalog_api_read_rejects_unbounded_or_invalid_parameters() {
    let mut f = Fixture::new().await;
    let client = f.client();
    f.query.page = Some(PaginationRequest { cursor: None, limit: Some(u32::MAX) });
    assert_http(client.model_catalog(&f.project, &f.query, TOKEN).await, 400);
    f.query.page = Some(PaginationRequest { cursor: Some("bad-cursor".into()), limit: None });
    assert_http(client.model_catalog(&f.project, &f.query, TOKEN).await, 400);
    f.query.page = None;
    f.query.search = Some("x".repeat(257));
    assert_http(client.model_catalog(&f.project, &f.query, TOKEN).await, 400);
    f.query.search = None;
    f.query.deployment.endpoint = "https://user:password@fixture.test/v1".into();
    assert_http(client.model_catalog(&f.project, &f.query, TOKEN).await, 400);
}

#[tokio::test]
async fn catalog_api_read_lists_presets_and_templates_without_auth_material() {
    let f = Fixture::new().await;
    let client = f.client();
    let presets = client
        .model_catalog_presets(&f.project, "mac", AppId::Codex, Some("openai"), None, TOKEN)
        .await
        .unwrap();
    assert!(
        presets
            .items
            .iter()
            .any(|preset| preset.app_id == AppId::Codex && preset.name.contains("OpenAI"))
    );
    let agy = client
        .model_catalog_presets(&f.project, "mac", AppId::Agy, None, None, TOKEN)
        .await
        .unwrap();
    assert!(agy.items.is_empty());
    let templates =
        client.model_catalog_templates(&f.project, "mac", AppId::Codex, TOKEN).await.unwrap();
    assert!(templates.codex_config.as_ref().unwrap().contains("wire_api"));
    assert!(templates.user_agents.iter().any(|ua| ua.starts_with("claude-cli/")));
    let body = serde_json::to_string(&templates).unwrap();
    assert!(!body.contains("OPENAI_API_KEY") && !body.contains(SECRET));
    assert_http(client.model_catalog_templates(&f.project, "mac", AppId::Codex, "bad").await, 401);
}

#[tokio::test]
async fn catalog_api_read_resolves_only_the_exact_model_deployment() {
    let f = Fixture::new().await;
    let client = f.client();
    let model = "vendor/One:free".into();
    let caps = client
        .model_catalog_capabilities(&f.project, &f.query.deployment, &model, TOKEN)
        .await
        .unwrap();
    assert_eq!(caps.model.identity.model_id, model);
    assert_eq!(caps.image_input, ModelCapabilityState::Unsupported);
    assert!(caps.text_only);
    assert_eq!(caps.model.metadata.tool_calls, ModelCapabilityState::Unsupported);
    let mut deployment = f.query.deployment.clone();
    deployment.endpoint = "https://other.test/v1".into();
    assert_http(
        client.model_catalog_capabilities(&f.project, &deployment, &model, TOKEN).await,
        404,
    );
    assert_http(
        client
            .model_catalog_capabilities(
                &f.project,
                &f.query.deployment,
                &"vendor/one:free".into(),
                TOKEN,
            )
            .await,
        404,
    );
}

#[path = "model_catalog_edits_tests.rs"]
mod edits;
