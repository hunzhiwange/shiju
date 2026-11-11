use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    middleware,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use tower::ServiceExt;
use vw_agent_apps::providers::{
    live::{commit::*, write_guard::lock_app},
    mode_state::StackState,
};
use vw_api_types::{
    agent_apps::{AppId, aggregation::*},
    id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, ProjectId, ProjectScope, TenantScope},
};
const TOKEN: &str = "aggregation-t40-local-test-bearer-token-32-bytes";
fn tenant() -> TenantScope {
    TenantScope::new("platform", "company").unwrap()
}
fn scope(project: &str) -> ProjectScope {
    ProjectScope::new(tenant(), ProjectId::from(project)).unwrap()
}
struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    tx: ProviderTransactions,
    policy: std::path::PathBuf,
    router: tokio::sync::OnceCell<Router>,
    routing: Option<std::sync::Arc<crate::app_routing::RoutingListener>>,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let private = root.path().join("private");
        std::fs::create_dir(&private).unwrap();
        std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700)).unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let store = GatewaySettingsStore::new(db.clone()).unwrap();
        for (id, model) in [("alpha", "first"), ("beta", "second")] {
            store.upsert(&tenant(),"app_providers.records",&format!("\"claude\":{id}"),&json!({"name":id,"category":"custom",
                "settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"https://fixture.invalid","ANTHROPIC_API_KEY":"synthetic-private-key","ANTHROPIC_MODEL":model}}}).to_string()).unwrap();
        }
        let tx =
            ProviderTransactions::new(&private, ExecutionHostId::new("host").unwrap()).unwrap();
        let mut guard = lock_app(AppId::Claude, root.path()).unwrap();
        tx.commit_project_checked(
            &scope("project"),
            &mut guard,
            &tx.state(AppId::Claude).unwrap(),
            true,
            &[],
            PendingTarget {
                pointer: Some("alpha".into()),
                mode: Some(ModeState {
                    mode: Mode::Direct,
                    attached: false,
                    proxy_route: Some("alpha".into()),
                    contract: None,
                }),
                stack: Some(StackState {
                    enabled: false,
                    members: vec!["alpha".into(), "beta".into()],
                    keys: [("alpha".into(), "alpha".into()), ("beta".into(), "beta".into())].into(),
                }),
                ..Default::default()
            },
            || Ok(()),
        )
        .unwrap();
        let policy = root.path().join("access.json");
        let f = Self { root, db, tx, policy, router: Default::default(), routing: None };
        f.policy(true, true, false);
        f
    }
    fn policy(&self, write: bool, aggregation: bool, routing: bool) {
        let apps = json!([{"project_id":"project","app_id":"claude"},{"project_id":"other","app_id":"claude"}]);
        std::fs::write(&self.policy,json!({"principals":[{"id":"writer","token_sha256":hex::encode(Sha256::digest(TOKEN)),
            "grants":[{"platform_id":"platform","company_id":"company","host_id":"host",
                "provider_config_read":true,"aggregation_read":apps,
                "provider_config_write":write,"aggregation_write":if aggregation {apps.clone()} else {json!([])},
                "routing_read":routing,"routing_manage":routing,"routing_apps":apps}]}],
            "hosts":[{"id":"host","online":true,"binding":"local","home":self.root.path(),"credential_root":self.root.path().join("private"),"search_paths":[],"applications":self.root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&self.policy, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    async fn router(&self) -> Router {
        self.router.get_or_init(|| self.build_router()).await.clone()
    }
    async fn build_router(&self) -> Router {
        let router = super::aggregation_read::router_with_policy(Some(self.policy.clone()))
            .merge(super::app_providers::operations::router_with_policy(Some(self.policy.clone())))
            .layer(Extension(self.db.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
        match &self.routing {
            Some(routing) => {
                super::app_routing::configure_with_policy(
                    router,
                    routing,
                    self.policy.clone(),
                    self.db.clone(),
                )
                .await
            }
            None => router,
        }
    }
    async fn request(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        payload: Value,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header(PLATFORM_ID_HEADER, "platform")
            .header(COMPANY_ID_HEADER, "company");
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        let body = if method == "GET" {
            Body::empty()
        } else {
            builder = builder.header("content-type", "application/json");
            Body::from(payload.to_string())
        };
        let response = self.router().await.oneshot(builder.body(body).unwrap()).await.unwrap();
        let status = response.status();
        assert_eq!(
            response.headers().get("cache-control").and_then(|h| h.to_str().ok()),
            Some("no-store")
        );
        let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-private-key"));
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, body)
    }
    #[allow(dead_code)]
    async fn current(&self) -> Value {
        let (status, value) = self
            .request(
                "GET",
                "/agent-apps/claude/aggregation/state?targetHostId=host&projectId=project",
                Some(TOKEN),
                json!({}),
            )
            .await;
        assert_eq!(status, 200, "{value}");
        value
    }
}

#[tokio::test]
async fn aggregation_t41_aggregation_publish_preserves_contract() {
    let f = Fixture::new();
    f.policy(true, true, true);
    let (status, preview) = f
        .request(
            "POST",
            "/agent-apps/claude/aggregation/preview",
            Some(TOKEN),
            json!({"targetHostId":"host","projectId":"project"}),
        )
        .await;
    assert_eq!(status, 200, "{preview}");
    assert!(preview["previewToken"].is_string(), "{preview}");
    let (status, body) = f.request("POST", "/agent-apps/claude/aggregation/publish", Some(TOKEN), json!({"targetHostId":"host","projectId":"project","expectedRevision":preview["revision"],"previewToken":preview["previewToken"],"requestId":"first","confirmed":false})).await;
    assert_eq!(status, 422, "{body}");
    assert_eq!(body["error"]["code"], "confirmation_required");
    assert!(!f.tx.state(AppId::Claude).unwrap().stack.enabled);
}

impl Fixture {
    async fn preview(&self) -> Value {
        let (status, value) = self
            .request(
                "POST",
                "/agent-apps/claude/aggregation/preview",
                Some(TOKEN),
                json!({"targetHostId":"host","projectId":"project"}),
            )
            .await;
        assert_eq!(status, 200, "{value}");
        value
    }
    async fn publish_request(&self) -> Value {
        let preview = self.preview().await;
        json!({"targetHostId":"host","projectId":"project","expectedRevision":preview["revision"],
            "previewToken":preview["previewToken"],"requestId":"publication","confirmed":true})
    }
    async fn publish(&self, body: Value) -> (StatusCode, Value) {
        self.request("POST", "/agent-apps/claude/aggregation/publish", Some(TOKEN), body).await
    }
    async fn settled(&self, id: &str) -> Value {
        for _ in 0..300 {
            let (status, value) = self
                .request(
                    "GET",
                    &format!("/app-providers/operations/{id}?targetHostId=host&projectId=project"),
                    Some(TOKEN),
                    json!({}),
                )
                .await;
            assert_eq!(status, 200, "{value}");
            if matches!(value["status"].as_str(), Some("completed" | "failed" | "cancelled")) {
                return value;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("publication did not settle");
    }
    fn native(&self) -> std::path::PathBuf {
        self.root.path().join(".claude/settings.json")
    }
    fn prepare_native(&self) {
        std::fs::create_dir(self.root.path().join(".claude")).unwrap();
        std::fs::write(
            self.native(),
            json!({"env":{"ANTHROPIC_MODEL":"first"},"hooks":{"user":true}}).to_string(),
        )
        .unwrap();
    }
    fn listener(&mut self) -> std::sync::Arc<crate::app_routing::RoutingListener> {
        self.policy(true, true, true);
        let listener = std::sync::Arc::new(crate::app_routing::RoutingListener::default());
        listener.start(0).unwrap();
        self.routing = Some(listener.clone());
        listener
    }
}
#[tokio::test]
async fn aggregation_t41_authorization_revision_and_signed_intent_precede_side_effects() {
    let f = Fixture::new();
    f.policy(true, true, true);
    let original = f.tx.state(AppId::Claude).unwrap();
    let request = f.publish_request().await;
    let (status, _) =
        f.request("POST", "/agent-apps/claude/aggregation/publish", None, request.clone()).await;
    assert_eq!(status, 401);
    f.policy(true, true, false);
    assert_eq!(f.publish(request.clone()).await.0, 403);
    f.policy(false, true, true);
    assert_eq!(f.publish(request.clone()).await.0, 403);
    f.policy(true, true, true);
    for field in ["expectedRevision", "previewToken"] {
        let mut changed = request.clone();
        changed[field] = json!("tampered");
        let (status, body) = f.publish(changed).await;
        assert_eq!(status, 409, "{body}");
        assert_eq!(body["error"]["code"], "revision_conflict");
    }
    for field in ["targetHostId", "projectId"] {
        let mut changed = request.clone();
        changed[field] = json!("ungranted");
        assert_eq!(f.publish(changed).await.0, 403);
    }
    let mut injected = request.clone();
    injected["path"] = json!("/tmp/untrusted");
    assert_eq!(f.publish(injected).await.0, 400);
    f.prepare_native();
    let (status, _) = f.publish(request).await;
    assert_eq!(status, 409);
    assert!(f.tx.state(AppId::Claude).unwrap() == original);
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
}
#[tokio::test]
async fn aggregation_t41_initial_publication_reuses_catalog_transaction_and_live_route() {
    use axum::routing::post;
    let mut f = Fixture::new();
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = upstream.local_addr().unwrap();
    let (send, mut receive) = tokio::sync::mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        axum::serve(upstream, Router::new().route("/v1/messages", post(move |axum::Json(body): axum::Json<Value>| {
            let send = send.clone(); async move {
                send.send(body.clone()).unwrap();
                axum::Json(json!({"id":"fixture","type":"message","role":"assistant","model":body["model"],
                    "content":[{"type":"text","text":"ok"}],"stop_reason":"end_turn","usage":{"input_tokens":1,"output_tokens":1}}))
            }
        }))).await.unwrap();
    });
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    for (id, model) in [("alpha", "first"), ("beta", "second")] {
        let old = store.claude_record(&tenant(), id).unwrap();
        store
            .claude_save(
                &tenant(),
                id,
                &vw_api_types::app_providers::claude::ClaudeProfile {
                    name: id.into(),
                    config: vw_api_types::app_providers::claude::ClaudeConfig::ThirdParty {
                        base_url: format!("http://{address}"),
                        api_key: "synthetic-private-key".into(),
                        api_key_header: true,
                    },
                    model: Some(model.into()),
                },
                old.as_ref(),
                false,
            )
            .unwrap();
    }
    f.prepare_native();
    let listener = f.listener();
    let mut guard = lock_app(AppId::Claude, f.root.path()).unwrap();
    let before = f.tx.state(AppId::Claude).unwrap();
    let mut planned = before.stack.clone();
    planned.members.retain(|id| id != "beta");
    f.tx.commit_project_checked(
        &scope("project"),
        &mut guard,
        &before,
        false,
        &[],
        PendingTarget { stack: Some(planned), ..Default::default() },
        || Ok(()),
    )
    .unwrap();
    drop(guard);
    let (status, preview) = f
        .request(
            "POST",
            "/agent-apps/claude/aggregation/preview",
            Some(TOKEN),
            json!({"targetHostId":"host","projectId":"project","defaultProviderId":"beta"}),
        )
        .await;
    assert_eq!(status, 200, "{preview}");
    let request = json!({"targetHostId":"host","projectId":"project","expectedRevision":preview["revision"],
        "previewToken":preview["previewToken"],"requestId":"publication","confirmed":true});
    let ((status, progress), (replay_status, concurrent)) =
        tokio::join!(f.publish(request.clone()), f.publish(request.clone()));
    assert_eq!(status, 202, "{progress}");
    assert_eq!(replay_status, 202, "{concurrent}");
    assert_eq!(progress["operationId"], concurrent["operationId"]);
    let id = progress["operationId"].as_str().unwrap();
    let terminal = f.settled(id).await;
    assert_eq!(terminal["status"], "completed", "{terminal}");
    assert_eq!(terminal["aggregationOutcome"]["kind"], "published");
    assert_eq!(terminal["aggregationOutcome"]["state"]["mode"], "aggregation");
    assert_eq!(terminal["aggregationOutcome"]["state"]["defaultProviderId"], "beta");
    let _: vw_api_types::app_providers::operations::ProviderOperationProgress =
        serde_json::from_value(terminal.clone()).unwrap();
    let state = f.tx.state(AppId::Claude).unwrap();
    assert!(state.stack.enabled && state.mode.as_ref().unwrap().attached);
    assert_eq!(state.pointer.as_ref().unwrap().id, "alpha");
    assert_eq!(state.mode.as_ref().unwrap().proxy_route.as_deref(), Some("beta"));
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
    let native: Value = serde_json::from_slice(&std::fs::read(f.native()).unwrap()).unwrap();
    assert_eq!(native["hooks"]["user"], true);
    let state = f.current().await;
    let model = state["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["providerId"] == "beta")
        .unwrap()["models"][0]["id"]
        .as_str()
        .unwrap();
    let response = reqwest::Client::new()
        .post(format!("{}/v1/messages", native["env"]["ANTHROPIC_BASE_URL"].as_str().unwrap()))
        .bearer_auth(native["env"]["ANTHROPIC_AUTH_TOKEN"].as_str().unwrap())
        .json(
            &json!({"model":model,"max_tokens":8,"messages":[{"role":"user","content":"fixture"}]}),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(receive.recv().await.unwrap()["model"], "second");
    let bytes = std::fs::read(f.native()).unwrap();
    let (status, replay) = f.publish(request.clone()).await;
    assert_eq!(status, 202);
    assert_eq!(replay["operationId"], id);
    assert_eq!(std::fs::read(f.native()).unwrap(), bytes);
    let mut changed = request.clone();
    changed["expectedRevision"] = json!("changed");
    let (status, body) = f.publish(changed).await;
    assert_eq!(status, 409);
    assert_eq!(body["error"]["code"], "request_conflict");
    let missing_scope = format!("/app-providers/operations/{id}?targetHostId=host");
    assert_eq!(f.request("GET", &missing_scope, None, json!({})).await.0, 401);
    assert_eq!(f.request("GET", &missing_scope, Some(TOKEN), json!({})).await.0, 404);
    let path = format!("/app-providers/operations/{id}?targetHostId=host&projectId=other");
    assert_eq!(f.request("GET", &path, Some(TOKEN), json!({})).await.0, 404);
    let path = format!("/app-providers/operations/{id}?targetHostId=host&projectId=project");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
    policy["principals"][0]["id"] = json!("other-principal");
    std::fs::write(&f.policy, policy.to_string()).unwrap();
    assert_eq!(f.request("GET", &path, Some(TOKEN), json!({})).await.0, 404);
    assert_eq!(f.request("DELETE", &path, Some(TOKEN), json!({})).await.0, 404);
    assert_eq!(f.publish(request.clone()).await.0, 409);
    f.policy(true, true, true);
    let (status, terminal_cancel) = f.request("DELETE", &path, Some(TOKEN), json!({})).await;
    assert_eq!(status, 200);
    assert_eq!(terminal_cancel["status"], "completed");
    f.policy(true, true, false);

    assert_eq!(f.request("GET", &path, Some(TOKEN), json!({})).await.0, 403);
    assert_eq!(f.request("DELETE", &path, Some(TOKEN), json!({})).await.0, 403);
    f.policy(true, true, true);
    listener.stop().await.unwrap();
    server.abort();
}
#[tokio::test]
async fn aggregation_t41_target_failure_preserves_original_state_and_native_bytes() {
    let mut f = Fixture::new();
    f.prepare_native();
    let listener = f.listener();
    // A valid preview catalog can still describe a transport the executor cannot safely publish.
    let request = f.publish_request().await;
    let state = f.tx.state(AppId::Claude).unwrap();
    let bytes = std::fs::read(f.native()).unwrap();
    let (status, progress) = f.publish(request).await;
    assert_eq!(status, 202, "{progress}");
    let terminal = f.settled(progress["operationId"].as_str().unwrap()).await;
    assert_eq!(terminal["status"], "failed", "{terminal}");
    assert!(terminal["aggregationOutcome"]["code"].is_string());
    assert!(f.tx.state(AppId::Claude).unwrap() == state);
    assert_eq!(std::fs::read(f.native()).unwrap(), bytes);
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
    listener.stop().await.unwrap();
}

#[path = "aggregation_publish_client_tests.rs"]
mod clients;

#[path = "aggregation_publish_operation_tests.rs"]
mod operation_contract;

#[path = "aggregation_publish_failure_tests.rs"]
mod failure_contract;

#[path = "../../aggregation_lifecycle_tests.rs"]
mod lifecycle_tests;
