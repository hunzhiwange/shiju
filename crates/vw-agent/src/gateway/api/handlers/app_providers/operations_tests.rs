use crate::{app_providers::AppProviderStore, global_database::GlobalDatabase};
use axum::{Extension, Router, middleware};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::Path};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{import::ImportTrigger, operations::*},
    id::TenantScope,
    task::TaskStatus,
};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-import-authorized-token-32-characters";
fn write(home: &Path, name: &str, bytes: &[u8]) {
    let file = home.join(name);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, bytes).unwrap();
}
fn tenant() -> TenantScope {
    TenantScope::new("p", "a").unwrap()
}
struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    port: u16,
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
        std::fs::create_dir(root.path().join("private")).unwrap();
        std::fs::set_permissions(
            root.path().join("private"),
            std::fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy, json!({"principals":[{"id":"import","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":root.path().join("private"),"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let router = super::operations::router_with_policy(Some(policy))
            .layer(Extension(db.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            axum::serve(listener, Router::new().nest("/v1", router)).await.unwrap()
        });
        let client =
            GatewayClient::new(GatewayEndpoint::new("127.0.0.1", port).with_tenant_scope(tenant()))
                .unwrap();
        Self { root, db, port, client, server }
    }
    fn record(&self, app: AppId, id: &str) -> Value {
        AppProviderStore::preview_record(&self.db, &tenant(), app, id).unwrap()
    }
}

#[tokio::test]
async fn cc04_operations_sdk_api_import_progress_correlates_and_denies_foreign_scope() {
    let f = Fixture::new().await;
    let native=br#"{"providers":{"one":{"apiKey":"fake-one","models":[]},"two":{"apiKey":"fake-two","models":[]}}}"#;
    write(f.root.path(), ".pi/agent/models.json", native);
    let req = ProviderOperationRequest {
        target_host_id: "host".into(),
        request_id: "request-one".into(),
        action: ProviderOperationAction::Import { app: AppId::Pi, trigger: ImportTrigger::Manual },
    };
    let first = f.client.provider_operation_start(&req, TOKEN).await.unwrap();
    let id = first.operation_id.0.clone();
    let mut last = first;
    for _ in 0..200 {
        last = f.client.provider_operation_get(&id, "host", TOKEN).await.unwrap();
        if !matches!(last.status, TaskStatus::Queued | TaskStatus::Running) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(last.status, TaskStatus::Completed);
    let foreign = GatewayClient::new(
        GatewayEndpoint::new("127.0.0.1", f.port)
            .with_tenant_scope(TenantScope::new("p", "other").unwrap()),
    )
    .unwrap();
    assert!(foreign.provider_operation_get(&id, "host", TOKEN).await.is_err());
    assert!(foreign.provider_operation_cancel(&id, "host", TOKEN).await.is_err());
    let mut conflicting = req.clone();
    conflicting.action =
        ProviderOperationAction::Import { app: AppId::Pi, trigger: ImportTrigger::Startup };
    assert!(matches!(
        f.client.provider_operation_start(&conflicting, TOKEN).await.unwrap_err(),
        vw_gateway_client::AgentAppsReadError::Http { status: 409, .. }
    ));
    assert_eq!(last.request_id, "request-one");
    assert_eq!((last.completed, last.total), (2, 2));
    assert_eq!(
        f.client.provider_operation_start(&req, TOKEN).await.unwrap().operation_id,
        last.operation_id
    );
    assert_eq!(std::fs::read(f.root.path().join(".pi/agent/models.json")).unwrap(), native);
    assert!(f.record(AppId::Pi, "two").to_string().contains("fake-two"));
    let bad = f.client.provider_operation_cancel(&id, "other", TOKEN).await.unwrap_err();
    assert!(matches!(bad, vw_gateway_client::AgentAppsReadError::Http { status: 403, .. }));
    assert!(f.client.provider_operation_get(&id, "host", "wrong-token").await.is_err());
}

#[tokio::test]
async fn cc04_operations_projection_persists_selected_apps_and_read_only_denial_is_inert() {
    let f = Fixture::new().await;
    let settings = crate::gateway::settings_store::GatewaySettingsStore::new(f.db.clone()).unwrap();
    let parent = json!({"config":{"name":"Shared","providerType":"custom","apps":{"claude":true,"codex":true},"baseUrl":"https://example.com","models":{"claude":{"model":"claude-main"},"codex":{"model":"codex-main"}}},"apiKey":"fixture-key"});
    settings.upsert(&tenant(), "app_providers.universal", "shared", &parent.to_string()).unwrap();
    let mut req = ProviderOperationRequest {
        target_host_id: "host".into(),
        request_id: "projection".into(),
        action: ProviderOperationAction::Project {
            id: "shared".into(),
            apps: vec![AppId::Claude, AppId::Codex],
        },
    };
    let first = f.client.provider_operation_start(&req, TOKEN).await.unwrap();
    let id = first.operation_id.0;
    let last = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let status = f.client.provider_operation_get(&id, "host", TOKEN).await.unwrap();
            if !matches!(status.status, TaskStatus::Queued | TaskStatus::Running) {
                break status;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(last.status, TaskStatus::Completed);
    assert_eq!((last.completed, last.total), (2, 2));
    assert!(f.record(AppId::Claude, "universal-claude-shared").to_string().contains("fixture-key"));
    assert!(f.record(AppId::Codex, "universal-codex-shared").to_string().contains("fixture-key"));
    assert!(!serde_json::to_string(&last).unwrap().contains("fixture-key"));
    assert_eq!(f.client.provider_operation_cancel(&id, "host", TOKEN).await.unwrap(), last);
    let before = f.record(AppId::Claude, "universal-claude-shared");
    let policy = f.root.path().join("access.json");
    let mut access: Value = serde_json::from_slice(&std::fs::read(&policy).unwrap()).unwrap();
    access["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(policy, access.to_string()).unwrap();
    req.request_id = "read-only".into();
    assert!(matches!(
        f.client.provider_operation_start(&req, TOKEN).await.unwrap_err(),
        vw_gateway_client::AgentAppsReadError::Http { status: 403, .. }
    ));
    assert!(f.client.provider_operation_cancel(&id, "host", TOKEN).await.is_err());
    assert_eq!(f.record(AppId::Claude, "universal-claude-shared"), before);
}

#[tokio::test]
async fn cc04_operations_registered_route_requires_gateway_authentication() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let app = crate::gateway::router::handler_router().layer(Extension(tenant())).layer(
        middleware::from_fn(|req, next| async {
            crate::gateway::middleware::basic_auth_middleware_with_credentials(
                req,
                next,
                Some("secret"),
                "test",
            )
            .await
        }),
    );
    let response=app.oneshot(Request::builder().method("POST").uri("/app-providers/operations").header("content-type","application/json").body(Body::from(r#"{"targetHostId":"host","requestId":"request","action":{"kind":"import","app":"pi","trigger":"manual"}}"#)).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
