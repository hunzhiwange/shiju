use crate::{
    app_providers::AppProviderStore, config, gateway, global_database::GlobalDatabase,
    session::ui_store::with_tenant_scope,
};
use serde_json::json;
use vw_api_app_identity::AppId;
use vw_api_app_provider_base::AppProviderWrite;
use vw_api_app_provider_base::ProviderProtocol;
use vw_api_transport::id::TenantScope;
use vw_api_model_catalog::ModelCapabilityState;
use vw_api_model_catalog::ModelCatalogDefaultChange;
use vw_api_model_catalog::ModelCatalogEdit;
use vw_api_model_catalog::ModelCatalogEditRequest;
use vw_api_model_catalog::ModelCatalogIdentity;
use vw_api_model_catalog::ModelMetadataDto;
use vw_api_model_catalog::ModelMetadataField;
use vw_api_model_catalog::ModelMetadataProvenance;
use vw_api_model_catalog::ModelMetadataSource;

const PROVIDER: &str = "projection-fixture";
const APP_PROVIDER: &str = "saved-app-profile";
const ENDPOINT: &str = "https://fixture.invalid/v1";
const REQUEST_ID: &str = "Vendor/Model:free";
const ALIAS: &str = "目录展示名";
const RUNTIME_KEY: &str = "runtime-fixture-key";
const ISOLATED: &str = "VW_CATALOG_PROJECTION_TEST_CHILD";

// Execute the real config and SQLite boundaries in a process with its own home.
async fn run_isolated(test: &str) -> bool {
    if std::env::var_os(ISOLATED).is_some() {
        return false;
    }
    let root = tempfile::tempdir().unwrap();
    let module = module_path!().split_once("::").unwrap().1;
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        tokio::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &format!("{module}::{test}"), "--nocapture"])
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", root.path())
            .env("TMPDIR", root.path())
            .env("VW_MODEL_RECOVERY_TEST_ROOT", root.path())
            .env("VIBEWINDOW_TEST_ROOT", root.path())
            .env("VIBEWINDOW_CONFIG_DIR", root.path().join("config"))
            .env("VIBEWINDOW_DISABLE_MODELS_FETCH", "1")
            .env("SHIJU_CATALOG_PROJECTION_RUNTIME_KEY", RUNTIME_KEY)
            .env(ISOLATED, "1")
            .current_dir(root.path())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .expect("isolated catalog projection test timed out")
    .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

struct Fixture {
    tenant: TenantScope,
    deployment: ModelCatalogIdentity,
    database: GlobalDatabase,
}

impl Fixture {
    async fn new(tenant_name: &str) -> Self {
        Self::with_endpoint(tenant_name, ENDPOINT).await
    }

    async fn with_endpoint(tenant_name: &str, endpoint: &str) -> Self {
        let tenant = TenantScope::new("catalog-projection", tenant_name).unwrap();
        let deployment = ModelCatalogIdentity {
            target_host_id: "mac-fixture".into(),
            app_id: AppId::Codex,
            provider_id: APP_PROVIDER.into(),
            endpoint: endpoint.into(),
        };
        let database = GlobalDatabase::for_home(&crate::global::paths().home).unwrap();
        let fixture = Self { tenant, deployment, database };
        fixture
            .store()
            .save(
                &fixture.tenant,
                AppId::Codex,
                APP_PROVIDER,
                &AppProviderWrite {
                    name: "Native fixture".into(),
                    protocol: ProviderProtocol::OpenaiResponses,
                    api_key: "fixture-private-secret".into(),
                    base_url: endpoint.into(),
                    model: "old".into(),
                    request_options: None,
                },
            )
            .unwrap();
        fixture.bind(&fixture.tenant, &fixture.deployment, endpoint).await;
        fixture
    }

    fn store(&self) -> AppProviderStore {
        AppProviderStore::new(self.database.clone()).unwrap()
    }

    async fn bind(&self, tenant: &TenantScope, deployment: &ModelCatalogIdentity, api: &str) {
        config::update_global_for_tenant(
            tenant,
            json!({"providers": {PROVIDER: {
                "api": api,
                "adapter": "openai-responses",
                "env": ["SHIJU_CATALOG_PROJECTION_RUNTIME_KEY"],
                "model_catalog": deployment,
                "models": {"gpt-5-old": {"status": "active"}}
            }}}),
        )
        .await
        .unwrap();
    }

    fn edit(
        &self,
        deployment: &ModelCatalogIdentity,
        edits: Vec<ModelCatalogEdit>,
        default_change: Option<ModelCatalogDefaultChange>,
    ) {
        let store = self.store();
        let current = store.read_model_catalog(&self.tenant, deployment).unwrap();
        store
            .edit_model_catalog(
                &self.tenant,
                &ModelCatalogEditRequest {
                    deployment: deployment.clone(),
                    expected_revision: current.revision,
                    edits,
                    default_change,
                },
            )
            .unwrap();
    }

    fn select(&self, alias: &str, metadata: ModelMetadataDto) {
        self.edit(
            &self.deployment,
            vec![ModelCatalogEdit::Upsert {
                model_id: REQUEST_ID.into(),
                display_alias: Some(alias.into()),
                metadata: Box::new(metadata),
            }],
            Some(ModelCatalogDefaultChange::Set { model_id: REQUEST_ID.into() }),
        );
    }
}

#[tokio::test]
async fn catalog_resolver_projection_keeps_gateway_picker_in_tenant() {
    if run_isolated("catalog_resolver_projection_keeps_gateway_picker_in_tenant").await {
        return;
    }
    let fixture = Fixture::new("gateway").await;
    fixture.select(ALIAS, ModelMetadataDto::default());
    let other = Fixture::new("other-tenant").await;
    other.select("另一租户的展示名", ModelMetadataDto::default());
    let dir = tempfile::tempdir().unwrap();
    let app = gateway::dynamic_tools_acceptance_router();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let directory = dir.path().to_str().unwrap();
    for (tenant, alias) in [(fixture.tenant, ALIAS), (other.tenant, "另一租户的展示名")] {
        let client = gateway_model_provider::GatewayModelProviderClient::new(
            gateway_http::GatewayTransport::new(
                gateway_endpoint::GatewayEndpoint::new("127.0.0.1", port)
                    .with_auth(gateway_endpoint::GatewayAuth {
                        skey: Some("dynamic-acceptance-skey".into()),
                    })
                    .with_tenant_scope(tenant),
            )
            .unwrap(),
        );
        let list = client.provider_list(Some(directory)).await.unwrap();
        let provider = list.all.iter().find(|row| row.id == PROVIDER).unwrap();
        assert_eq!(provider.models[REQUEST_ID].name, alias);
        assert_eq!(list.default[PROVIDER], REQUEST_ID);
        assert!(list.connected.iter().any(|id| id == PROVIDER));
        assert_eq!(provider.key.as_deref(), Some(RUNTIME_KEY));
    }
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn catalog_execution_gate_rejects_gateway_image_before_ocr() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use {gateway_endpoint::{GatewayAuth, GatewayEndpoint}, {vw_api_chat::GatewayChatStreamEvent, vw_api_chat::GatewayChatStreamRequest, vw_gateway_client::GatewayClient}};

    if run_isolated("catalog_execution_gate_rejects_gateway_image_before_ocr").await {
        return;
    }
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1", upstream.local_addr().unwrap());
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let upstream_app = axum::Router::new().route(
        "/v1/responses",
        axum::routing::post(move || async move {
            observed.fetch_add(1, Ordering::Relaxed);
            axum::http::StatusCode::BAD_REQUEST
        }),
    );
    let upstream = tokio::spawn(async move { axum::serve(upstream, upstream_app).await.unwrap() });
    let fixture = Fixture::with_endpoint("execution-gateway", &url).await;
    fixture.select(
        ALIAS,
        ModelMetadataDto { input_modalities: Some(vec!["text".into()]), ..Default::default() },
    );
    let model = format!("{PROVIDER}/{REQUEST_ID}");
    config::update_global_for_tenant(
        &fixture.tenant,
        json!({"multimodal": {"ocr": {"enabled": true, "model": model}}}),
    )
    .await
    .unwrap();

    let workspace = tempfile::tempdir().unwrap();
    let gateway = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = GatewayEndpoint::new("127.0.0.1", gateway.local_addr().unwrap().port())
        .with_tenant_scope(fixture.tenant)
        .with_auth(GatewayAuth { skey: Some("dynamic-acceptance-skey".into()) });
    let gateway = tokio::spawn(async move {
        axum::serve(gateway, crate::gateway::dynamic_tools_acceptance_router()).await.unwrap();
    });
    let body = GatewayChatStreamRequest {
        model: Some(model.clone()),
        messages: vec![json!({"role":"user","content":[
            {"type":"text","text":"Describe the image"},
            {"type":"image","mimeType":"image/png","data":"aW1hZ2U="}
        ]})],
        ..Default::default()
    };
    let mut events = Vec::new();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        GatewayClient::stream_chat(&endpoint, workspace.path().to_str(), &body, |event| {
            events.push(event);
            true
        }),
    )
    .await;
    gateway.abort();
    upstream.abort();
    let _ = tokio::join!(gateway, upstream);
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    result.expect("gateway capability rejection timed out").unwrap();
    assert!(
        events.iter().any(|event| matches!(
            event, GatewayChatStreamEvent::Error(error)
                if error.contains(&model) && error.contains("does not support image input")
        )),
        "{events:?}"
    );
    assert!(!events.iter().any(|event| matches!(event, GatewayChatStreamEvent::Delta(_))));
}
