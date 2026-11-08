use crate::global_database::GlobalDatabase;
use axum::{Extension, Router, middleware};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{os::unix::fs::PermissionsExt, path::Path};
use vw_api_types::{agent_apps::AppId, app_providers::mode_state::*, id::TenantScope};
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
        let router = super::mode_state::router_with_policy(Some(policy))
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
        Self { root, db, client, server }
    }
}

use crate::gateway::settings_store::GatewaySettingsStore;
use vw_agent_apps::providers::{
    live::{commit::*, write_guard::lock_app},
    mode_state::*,
};
fn query() -> ModeStateQuery {
    ModeStateQuery { target_host_id: "host".into(), provider_id: None, mode: ProviderMode::Direct }
}
fn seed(f: &Fixture) -> (ProviderTransactions, std::path::PathBuf, Vec<u8>) {
    let settings = json!({"env":{"ANTHROPIC_BASE_URL":"https://fixture.example","ANTHROPIC_API_KEY":"fake-saved-secret","ANTHROPIC_MODEL":"fixture"}});
    GatewaySettingsStore::new(f.db.clone())
        .unwrap()
        .upsert(
            &tenant(),
            "app_providers.records",
            "\"claude\":saved",
            &json!({"settingsConfig":settings,"category":"custom"}).to_string(),
        )
        .unwrap();
    write(f.root.path(), "native-config", b"native-file-unchanged");
    let tx = ProviderTransactions::new(
        &f.root.path().join("private"),
        ExecutionHostId::new("host").unwrap(),
    )
    .unwrap();
    let mut guard = lock_app(AppId::Claude, f.root.path()).unwrap();
    tx.commit(
        &tenant(),
        &mut guard,
        &[],
        PendingTarget {
            pointer: Some("saved".into()),
            mode: Some(ModeState {
                mode: Mode::Direct,
                attached: false,
                proxy_route: Some("route".into()),
                contract: Some(
                    ClientConfiguration::saved(AppId::Claude, &settings, false).unwrap().contract(),
                ),
            }),
            stack: Some(StackState {
                enabled: true,
                members: vec!["route".into()],
                keys: Default::default(),
            }),
            ..Default::default()
        },
    )
    .unwrap();
    let path = f.root.path().join("private/provider-live/aG9zdA/claude.state.json");
    let before = std::fs::read(&path).unwrap();
    (tx, path, before)
}
#[tokio::test]
async fn cc04_mode_state_sdk_api_reads_persisted_state_and_safe_decision_without_writes() {
    let f = Fixture::new().await;
    let (tx, path, before) = seed(&f);
    let db_before = std::fs::read(f.db.sqlite_path().unwrap()).unwrap();
    let mut q = query();
    q.provider_id = Some("saved".into());
    let status = f.client.provider_mode_state(AppId::Claude, &q, TOKEN).await.unwrap();
    assert_eq!(status.direct_provider.as_deref(), Some("saved"));
    assert_eq!(status.in_use_provider.as_deref(), Some("saved"));
    assert!(!status.stack.active);
    assert!(status.stack.enabled);
    assert_eq!(status.decision, Some(RewriteDecision::SkipRewrite));
    let value = serde_json::to_string(&status).unwrap();
    assert!(!value.contains("fake-saved-secret"));
    assert!(!value.contains("exclusive"));
    assert!(!value.contains("key"));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read(f.db.sqlite_path().unwrap()).unwrap(), db_before);
    assert_eq!(
        std::fs::read(f.root.path().join("native-config")).unwrap(),
        b"native-file-unchanged"
    );
    assert!(tx.pending(AppId::Claude).unwrap().is_none());
    let mut guard = lock_app(AppId::Claude, f.root.path()).unwrap();
    let mut mode = tx.state(AppId::Claude).unwrap().mode.unwrap();
    mode.mode = Mode::Proxy;
    mode.attached = false;
    tx.commit(&tenant(), &mut guard, &[], PendingTarget { mode: Some(mode), ..Default::default() })
        .unwrap();
    let status = f.client.provider_mode_state(AppId::Claude, &query(), TOKEN).await.unwrap();
    assert_eq!(status.in_use_provider.as_deref(), Some("route"));
    assert!(status.stack.active);
    assert!(!status.attached);
    assert_eq!(status.route_runtime, RouteRuntimeStatus::Unavailable);
}
#[tokio::test]
async fn cc04_mode_state_denial_unknown_fields_and_unavailable_proxy_do_not_write() {
    let f = Fixture::new().await;
    let (_tx, path, before) = seed(&f);
    for (q, token, expected) in [
        (query(), "wrong", 401),
        (
            ModeStateQuery {
                mode: ProviderMode::Proxy,
                provider_id: Some("saved".into()),
                ..query()
            },
            TOKEN,
            503,
        ),
    ] {
        let error = f.client.provider_mode_state(AppId::Claude, &q, token).await.unwrap_err();
        assert!(
            matches!(error,vw_gateway_client::AgentAppsReadError::Http{status,..} if status==expected)
        );
    }
    assert_eq!(std::fs::read(path).unwrap(), before);
}
#[tokio::test]
async fn cc04_mode_state_missing_state_is_empty_without_provisioning() {
    let f = Fixture::new().await;
    let view = f.client.provider_mode_state(AppId::Claude, &query(), TOKEN).await.unwrap();
    assert_eq!(view.mode, ProviderMode::Direct);
    assert!(view.direct_provider.is_none());
    assert!(view.in_use_provider.is_none());
    assert!(view.decision.is_none());
    assert!(!view.has_contract);
    assert!(!f.root.path().join("private/provider-live").exists());
}
#[tokio::test]
async fn cc04_mode_state_forged_contract_foreign_owner_and_pending_are_rejected() {
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;
    let f = Fixture::new().await;
    let (_tx, path, before) = seed(&f);
    let router = super::mode_state::router_with_policy(Some(f.root.path().join("access.json")))
        .layer(Extension(f.db.clone()))
        .layer(Extension(tenant()));
    let response = router
        .oneshot(
            Request::builder()
                .uri("/app-providers/claude/mode/state?targetHostId=host&contract=forged")
                .header("x-agent-apps-token", TOKEN)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
    let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("fake-saved-secret"));
    let mut state: AppState = serde_json::from_slice(&before).unwrap();
    state.mode_tenant = Some(TenantScope::new("p", "other").unwrap());
    std::fs::write(&path, serde_json::to_vec(&state).unwrap()).unwrap();
    let error = f.client.provider_mode_state(AppId::Claude, &query(), TOKEN).await.unwrap_err();
    assert!(matches!(error, vw_gateway_client::AgentAppsReadError::Http { status: 403, .. }));
    std::fs::write(&path, &before).unwrap();
    let pending = path.with_file_name("claude.pending.json");
    std::fs::write(&pending,serde_json::to_vec(&json!({"version":4,"host":"host","app":"claude","tenant":tenant(),"directory_device":0,"directory_inode":0,"target":{},"files":[],"published":false})).unwrap()).unwrap();
    let error = f.client.provider_mode_state(AppId::Claude, &query(), TOKEN).await.unwrap_err();
    assert!(matches!(error, vw_gateway_client::AgentAppsReadError::Http { status: 409, .. }));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(pending.exists());
}
