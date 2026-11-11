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
        let f = Self { root, db, tx, policy, routing: None };
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
        let router = super::aggregation_read::router_with_policy(Some(self.policy.clone()))
            .merge(super::aggregation_members::router_with_policy(Some(self.policy.clone())))
            .layer(Extension(self.db.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
        match &self.routing {
            Some(routing) => {
                super::app_routing::configure_with_policy(router, routing, self.policy.clone())
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
        let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-private-key"));
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, body)
    }
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
    async fn change(&self, path: &str, extra: Value) -> (StatusCode, Value) {
        let mut body = json!({"targetHostId":"host","projectId":"project","expectedRevision":self.current().await["revision"]});
        for (key, value) in extra.as_object().unwrap() {
            body[key] = value.clone();
        }
        self.request("PUT", &format!("/agent-apps/claude/aggregation/{path}"), Some(TOKEN), body)
            .await
    }
}

#[tokio::test]
async fn aggregation_t40_aggregation_members_preserves_contract() {
    let f = Fixture::new();
    let original = f.tx.state(AppId::Claude).unwrap();
    let (status, value) = f.change("members/beta", json!({"enabled":false})).await;
    assert_eq!(status, 200, "{value}");
    let state: AggregationState = serde_json::from_value(value).unwrap();
    assert_eq!(state.members.len(), 1);
    let settled = f.tx.state(AppId::Claude).unwrap();
    assert_eq!(settled.project_scope, original.project_scope);
    assert!(settled.pointer == original.pointer);
    assert_eq!(settled.stack.keys, original.stack.keys);
    assert!(!f.root.path().join(".claude").exists());
    let (status, _) = f.change("members/beta", json!({"enabled":true})).await;
    assert_eq!(status, 200);
    let (status, _) = f.change("default", json!({"providerId":"beta"})).await;
    assert_eq!(status, 200);
    assert_eq!(
        f.tx.state(AppId::Claude).unwrap().mode.unwrap().proxy_route.as_deref(),
        Some("beta")
    );
    assert!(f.tx.state(AppId::Claude).unwrap().pointer == original.pointer);
    let state = f.tx.state(AppId::Claude).unwrap();
    let current = f.current().await;
    let (status, repeat) = f.change("default", json!({"providerId":"beta"})).await;
    assert_eq!(status, 200);
    assert_eq!(repeat["revision"], current["revision"]);
    assert!(f.tx.state(AppId::Claude).unwrap() == state);
}
#[tokio::test]
async fn aggregation_t40_write_authorization_and_scope_precede_side_effects() {
    let f = Fixture::new();
    let original = f.tx.state(AppId::Claude).unwrap();
    let body = json!({"targetHostId":"host","projectId":"project","expectedRevision":"stale","enabled":false});
    let path = "/agent-apps/claude/aggregation/members/beta";
    let (status, value) = f.request("PUT", path, None, body.clone()).await;
    assert_eq!(status, 401, "{value}");
    for (write, aggregation) in [(false, true), (true, false)] {
        f.policy(write, aggregation, false);
        let (status, value) = f.request("PUT", path, Some(TOKEN), body.clone()).await;
        assert_eq!(status, 403, "{value}");
    }
    f.policy(true, true, false);
    for (field, value) in
        [("targetHostId", "elsewhere"), ("projectId", "other"), ("projectId", "ungranted")]
    {
        let mut body = body.clone();
        body[field] = json!(value);
        let (status, value) = f.request("PUT", path, Some(TOKEN), body).await;
        assert_eq!(status, 403, "{value}");
    }
    assert!(f.tx.state(AppId::Claude).unwrap() == original);
}
#[tokio::test]
async fn aggregation_t40_stale_revision_and_invalid_intents_preserve_state() {
    let f = Fixture::new();
    let original = f.tx.state(AppId::Claude).unwrap();
    let revision = f.current().await["revision"].clone();
    let (status, value) =
        f.change("members/beta", json!({"enabled":false,"expectedRevision":"stale"})).await;
    assert_eq!(status, 409, "{value}");
    assert_eq!(value["error"]["code"], "revision_conflict");
    let (status, value) =
        f.change("members/beta", json!({"enabled":false,"path":"untrusted"})).await;
    assert_eq!(status, 400, "{value}");
    let (status, value) = f.change("default", json!({"providerId":"missing"})).await;
    assert_eq!(status, 404, "{value}");
    let store = GatewaySettingsStore::new(f.db.clone()).unwrap();
    store
        .upsert(
            &tenant(),
            "app_providers.records",
            "\"claude\":official",
            &json!({"name":"Official","category":"official","settingsConfig":{"env":{}}})
                .to_string(),
        )
        .unwrap();
    let (status, value) = f.change("members/official", json!({"enabled":true})).await;
    assert_eq!(status, 422, "{value}");
    assert_eq!(value["error"]["code"], "invalid_member");
    assert!(f.tx.state(AppId::Claude).unwrap() == original);
    assert_eq!(f.current().await["revision"], revision);
}
#[tokio::test]
async fn aggregation_t40_active_default_removal_is_explicit_and_does_not_publish() {
    let f = Fixture::new();
    let mut guard = lock_app(AppId::Claude, f.root.path()).unwrap();
    let expected = f.tx.state(AppId::Claude).unwrap();
    let mut stack = expected.stack.clone();
    stack.enabled = true;
    f.tx.commit_project_checked(
        &scope("project"),
        &mut guard,
        &expected,
        false,
        &[],
        PendingTarget {
            stack: Some(stack),
            mode: Some(ModeState {
                mode: Mode::Proxy,
                attached: true,
                proxy_route: Some("alpha".into()),
                contract: None,
            }),
            ..Default::default()
        },
        || Ok(()),
    )
    .unwrap();
    drop(guard);
    f.policy(true, true, true);
    let original = f.tx.state(AppId::Claude).unwrap();
    let (status, value) = f.change("members/alpha", json!({"enabled":false})).await;
    assert_eq!(status, 409, "{value}");
    assert_eq!(value["error"]["code"], "default_member_required");
    f.policy(true, true, false);
    let (status, value) = f.change("members/beta", json!({"enabled":false})).await;
    assert_eq!(status, 403, "{value}");
    assert!(f.tx.state(AppId::Claude).unwrap() == original);
}

#[tokio::test]
async fn aggregation_t40_target_failure_preserves_members_and_revision() {
    let f = Fixture::new();
    let original = f.tx.state(AppId::Claude).unwrap();
    let revision = f.current().await["revision"].clone();
    let resource = f.root.path().join("private/provider-live/aG9zdA/claude.resource.json");
    std::fs::remove_file(&resource).unwrap();
    std::fs::create_dir(&resource).unwrap();
    let (status, value) = f.change("members/beta", json!({"enabled":false})).await;
    assert_eq!(status, 502, "{value}");
    assert_eq!(value["error"]["code"], "target_failed");
    assert!(f.tx.state(AppId::Claude).unwrap() == original);
    assert_eq!(f.current().await["revision"], revision);
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
}

#[tokio::test]
async fn aggregation_t40_default_adds_member_and_native_change_invalidates_revision() {
    let f = Fixture::new();
    let (status, value) = f.change("members/beta", json!({"enabled":false})).await;
    assert_eq!(status, 200, "{value}");
    let (status, value) = f.change("default", json!({"providerId":"beta"})).await;
    assert_eq!(status, 200, "{value}");
    assert!(f.tx.state(AppId::Claude).unwrap().stack.members.contains(&"beta".into()));
    let revision = value["revision"].clone();
    std::fs::create_dir(f.root.path().join(".claude")).unwrap();
    std::fs::write(
        f.root.path().join(".claude/settings.json"),
        br#"{"env":{"USER_SETTING":"changed"}}"#,
    )
    .unwrap();
    let original = f.tx.state(AppId::Claude).unwrap();
    let (status, value) =
        f.change("members/alpha", json!({"enabled":false,"expectedRevision":revision})).await;
    assert_eq!(status, 409, "{value}");
    assert_eq!(value["error"]["code"], "revision_conflict");
    assert!(f.tx.state(AppId::Claude).unwrap() == original);
}

#[tokio::test]
async fn aggregation_t40_active_changes_republish_real_catalog_and_routes() {
    use axum::routing::post;
    use vw_agent_apps::providers::mode_state::ClientConfiguration;
    let mut f = Fixture::new();
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = upstream.local_addr().unwrap();
    let (sent, mut received) = tokio::sync::mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        let app=Router::new().route("/v1/messages",post(move |axum::Json(body):axum::Json<Value>| async move {
            sent.send(body.clone()).unwrap();
            axum::Json(json!({"id":"fixture","type":"message","role":"assistant","model":body["model"],"content":[{"type":"text","text":"ok"}],"stop_reason":"end_turn","usage":{"input_tokens":1,"output_tokens":1}}))
        }));
        axum::serve(upstream, app).await.unwrap();
    });
    let store = crate::app_providers::AppProviderStore::new(f.db.clone()).unwrap();
    for (id, model) in [("alpha", "first"), ("beta", "second"), ("gamma", "third")] {
        let before = store.claude_record(&tenant(), id).unwrap();
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
                before.as_ref(),
                before.is_none(),
            )
            .unwrap();
    }
    let previous = json!({"env":{"ANTHROPIC_BASE_URL":"http://127.0.0.1:1","ANTHROPIC_AUTH_TOKEN":"old-local-token","ANTHROPIC_MODEL":"first"},"hooks":{"user":true}});
    std::fs::create_dir(f.root.path().join(".claude")).unwrap();
    let path = f.root.path().join(".claude/settings.json");
    std::fs::write(&path, previous.to_string()).unwrap();
    let mut guard = lock_app(AppId::Claude, f.root.path()).unwrap();
    let original = f.tx.state(AppId::Claude).unwrap();
    let mut stack = original.stack.clone();
    stack.enabled = true;
    f.tx.commit_project_checked(
        &scope("project"),
        &mut guard,
        &original,
        false,
        &[],
        PendingTarget {
            stack: Some(stack),
            mode: Some(ModeState {
                mode: Mode::Proxy,
                attached: true,
                proxy_route: Some("alpha".into()),
                contract: Some(
                    ClientConfiguration::saved(AppId::Claude, &previous, false).unwrap().contract(),
                ),
            }),
            ..Default::default()
        },
        || Ok(()),
    )
    .unwrap();
    drop(guard);
    f.policy(true, true, true);
    let listener = std::sync::Arc::new(crate::app_routing::RoutingListener::default());
    listener.start(0).unwrap();
    f.routing = Some(listener.clone());
    let (status, added) = f.change("members/gamma", json!({"enabled":true})).await;
    assert_eq!(status, 200, "{added}");
    assert!(f.tx.state(AppId::Claude).unwrap().stack.members.contains(&"gamma".into()));
    let native: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(native["hooks"]["user"], true);
    let model = added["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["providerId"] == "gamma")
        .unwrap()["models"][0]["id"]
        .as_str()
        .unwrap();
    assert!(
        native["modelPicker"]["options"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["model"] == model)
    );
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();
    let send = |native: &Value, model: &str| {
        client.post(format!("{}/v1/messages",native["env"]["ANTHROPIC_BASE_URL"].as_str().unwrap()))
        .bearer_auth(native["env"]["ANTHROPIC_AUTH_TOKEN"].as_str().unwrap())
        .json(&json!({"model":model,"messages":[{"role":"user","content":"test"}],"max_tokens":8}))
    };
    let response = send(&native, model).send().await.unwrap();
    assert_eq!(response.status(), 200, "{}", response.text().await.unwrap());
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(10), received.recv())
            .await
            .unwrap()
            .unwrap()["model"],
        "third"
    );
    let (status, changed) = f.change("default", json!({"providerId":"beta"})).await;
    assert_eq!(status, 200, "{changed}");
    assert_eq!(changed["defaultProviderId"], "beta");
    assert_eq!(f.tx.state(AppId::Claude).unwrap().pointer.as_ref().unwrap().id, "alpha");
    let revoked = send(&native, model).send().await.unwrap();
    assert_eq!(revoked.status(), 401);
    assert!(received.try_recv().is_err());
    let native_after: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let response = send(&native_after, "second").send().await.unwrap();
    assert_eq!(response.status(), 200, "{}", response.text().await.unwrap());
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(10), received.recv())
            .await
            .unwrap()
            .unwrap()["model"],
        "second"
    );
    let (status, removed) = f.change("members/gamma", json!({"enabled":false})).await;
    assert_eq!(status, 200, "{removed}");
    assert!(!f.tx.state(AppId::Claude).unwrap().stack.members.contains(&"gamma".into()));
    assert!(f.tx.state(AppId::Claude).unwrap().stack.keys.values().any(|id| id == "gamma"));
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
    let current_native: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let stale_model = send(&current_native, model).send().await.unwrap();
    assert_eq!(stale_model.status(), 404);
    assert!(received.try_recv().is_err());
    listener.stop().await.unwrap();
    server.abort();
    assert!(server.await.unwrap_err().is_cancelled());
}

#[tokio::test]
async fn aggregation_t40_active_changes_reject_failover_before_publication() {
    let f = Fixture::new();
    let mut guard = lock_app(AppId::Claude, f.root.path()).unwrap();
    let state = f.tx.state(AppId::Claude).unwrap();
    let mut stack = state.stack.clone();
    stack.enabled = true;
    f.tx.commit_project_checked(
        &scope("project"),
        &mut guard,
        &state,
        false,
        &[],
        PendingTarget {
            stack: Some(stack),
            mode: Some(ModeState {
                mode: Mode::Proxy,
                attached: true,
                proxy_route: Some("alpha".into()),
                contract: None,
            }),
            ..Default::default()
        },
        || Ok(()),
    )
    .unwrap();
    drop(guard);
    f.policy(true, true, true);
    let scope = vw_api_types::agent_apps_high_availability::FailoverQueueScope {
        project: scope("project"),
        target_host_id: "host".into(),
        app_id: AppId::Claude,
    };
    let key = crate::gateway::agent_apps_high_availability_store::config_key(&scope).unwrap();
    GatewaySettingsStore::new(f.db.clone())
        .unwrap()
        .upsert(
            &tenant(),
            "agent_apps.high_availability",
            &key,
            &json!({"version":0,"enabled":true,"members":[]}).to_string(),
        )
        .unwrap();
    let before = f.tx.state(AppId::Claude).unwrap();
    let (status, value) = f.change("members/beta", json!({"enabled":false})).await;
    assert_eq!(status, 409, "{value}");
    assert_eq!(value["error"]["code"], "failover_conflict");
    assert!(f.tx.state(AppId::Claude).unwrap() == before);
    assert!(!f.root.path().join(".claude").exists());
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
}

#[path = "aggregation_members_client_tests.rs"]
mod client_tests;
