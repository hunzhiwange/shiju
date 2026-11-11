use super::app_routing::{RoutingControl, configure_with_policy};
use crate::{
    app_providers::AppProviderStore, app_routing::RoutingListener, global_database::GlobalDatabase,
};
use axum::{
    Extension, Json, Router,
    body::{Body, to_bytes},
    http::{HeaderMap, Request, StatusCode},
    middleware,
    routing::{get, post},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, sync::Arc};
use tokio::{net::TcpListener, sync::mpsc};
use tower::ServiceExt;
use vw_agent_apps::providers::{claude::ClaudeFile, live::commit::*, mode_state::StackState};
use vw_api_types::{
    agent_apps::{AppId, aggregation::AggregationWriteQuery},
    app_providers::claude::{ClaudeConfig, ClaudeProfile},
    id::{COMPANY_ID_HEADER, PLATFORM_ID_HEADER, ProjectScope, TenantScope},
};
const TOKEN: &str = "aggregation-task16-fixture-manager-credential";
fn tenant() -> TenantScope {
    TenantScope::new("platform", "company").unwrap()
}
struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    tx: ProviderTransactions,
    router: Router,
    listener: Arc<RoutingListener>,
    control: RoutingControl,
    policy: std::path::PathBuf,
    requests: mpsc::UnboundedReceiver<(HeaderMap, Value)>,
    upstream: tokio::task::JoinHandle<()>,
    endpoint: String,
}
impl Fixture {
    async fn new(attached: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let private = root.path().join("private");
        fs::create_dir(&private).unwrap();
        fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
        let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", upstream.local_addr().unwrap());
        let (send, requests) = mpsc::unbounded_channel();
        let server=Router::new().route("/v1/messages",post(move|headers:HeaderMap,Json(body):Json<Value>|{let send=send.clone();async move{send.send((headers,body)).unwrap();Json(json!({"id":"fixture","type":"message","role":"assistant","content":[],"model":"fixture","stop_reason":"end_turn","usage":{"input_tokens":1,"output_tokens":1}}))}}));
        let upstream = tokio::spawn(async move { axum::serve(upstream, server).await.unwrap() });
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let store = AppProviderStore::new(db.clone()).unwrap();
        for (id, model) in [("alpha", "model-a"), ("beta", "model-b")] {
            store
                .claude_save(
                    &tenant(),
                    id,
                    &ClaudeProfile {
                        name: id.into(),
                        config: ClaudeConfig::ThirdParty {
                            base_url: endpoint.clone(),
                            api_key: format!("fixture-secret-for-{id}"),
                            api_key_header: true,
                        },
                        model: Some(model.into()),
                    },
                    None,
                    true,
                )
                .unwrap();
        }
        let tx =
            ProviderTransactions::new(&private, ExecutionHostId::new("host").unwrap()).unwrap();
        let project = ProjectScope::new(tenant(), "project".into()).unwrap();
        let mut file = ClaudeFile::open(root.path(), true).unwrap();
        fs::write(
            root.path().join(".claude/settings.json"),
            br#"{"env":{"USER_FLAG":"kept"},"hooks":{"user":true}}"#,
        )
        .unwrap();
        tx.commit_project_checked(
            &project,
            &mut file.guard,
            &tx.state(AppId::Claude).unwrap(),
            true,
            &[],
            PendingTarget {
                pointer: Some("alpha".into()),
                stack: Some(StackState {
                    enabled: true,
                    members: vec!["alpha".into(), "beta".into()],
                    keys: [("a".into(), "alpha".into()), ("b".into(), "beta".into())].into(),
                }),
                mode: Some(ModeState {
                    mode: Mode::Proxy,
                    attached,
                    proxy_route: Some("alpha".into()),
                    contract: None,
                }),
                ..Default::default()
            },
            || Ok(()),
        )
        .unwrap();
        drop(file);
        let policy = root.path().join("access.json");
        fs::write(&policy,json!({"principals":[{"id":"manager","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"platform","company_id":"company","host_id":"host","provider_config_read":true,"provider_config_write":true,"aggregation_read":[{"project_id":"project","app_id":"claude"}],"aggregation_write":[{"project_id":"project","app_id":"claude"}],"routing_read":true,"routing_manage":true,"routing_apps":[{"project_id":"project","app_id":"claude"}]}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":private,"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
        let listener = Arc::new(RoutingListener::default());
        listener.start(0).unwrap();
        let (send_control, mut receive) = mpsc::unbounded_channel();
        let router = super::aggregation_read::router_with_policy(Some(policy.clone()))
            .merge(super::app_providers::claude::router_with_policy(Some(policy.clone())))
            .route(
                "/__control",
                get(move |Extension(control): Extension<RoutingControl>| {
                    let send = send_control.clone();
                    async move {
                        send.send(control).unwrap();
                    }
                }),
            );
        let router = configure_with_policy(router, &listener, policy.clone(), db.clone())
            .await
            .layer(Extension(db.clone()))
            .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
        router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/__control")
                    .header(PLATFORM_ID_HEADER, "platform")
                    .header(COMPANY_ID_HEADER, "company")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let control = receive.try_recv().unwrap();
        Self { root, db, tx, router, listener, control, policy, requests, upstream, endpoint }
    }
    async fn request(&self, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
        let response = self
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header(PLATFORM_ID_HEADER, "platform")
                    .header(COMPANY_ID_HEADER, "company")
                    .header("authorization", format!("Bearer {TOKEN}"))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }
    async fn revision(&self) -> String {
        let (status, body) = self
            .request(
                "GET",
                "/agent-apps/claude/aggregation/state?targetHostId=host&projectId=project",
                Value::Null,
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        body["revision"].as_str().unwrap().into()
    }
    async fn change(&self, revision: &str, model: &str) -> StatusCode {
        self.request("PATCH",&format!("/app-providers/claude/beta/form?projectId=project&expectedRevision={revision}"),json!({"targetHostId":"host","fields":{"name":"beta"},"config":{"kind":"third_party","baseUrl":self.endpoint,"apiKeyHeader":true,"apiKey":{"action":"keep"}},"model":model,"apply":false})).await.0
    }
    fn bytes(&self) -> Vec<u8> {
        fs::read(self.root.path().join(".claude/settings.json")).unwrap()
    }
    async fn close(self) {
        self.listener.stop().await.unwrap();
        self.upstream.abort();
        let _ = self.upstream.await;
    }
}
#[tokio::test]
async fn aggregation_t16_claude_record_edit_publishes_real_files_and_listener() {
    let mut f = Fixture::new(true).await;
    let revision = f.revision().await;
    assert_eq!(f.change(&revision, "model-new").await, StatusCode::OK);
    let live: Value = serde_json::from_slice(&f.bytes()).unwrap();
    assert_eq!(live["env"]["USER_FLAG"], "kept");
    assert_eq!(live["hooks"]["user"], true);
    let address = live["env"]["ANTHROPIC_BASE_URL"].as_str().unwrap();
    let token = live["env"]["ANTHROPIC_AUTH_TOKEN"].as_str().unwrap();
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let response = client
        .post(format!("{address}/v1/messages"))
        .header("x-api-key", token)
        .json(&json!({"model":"ccs-claude-b--model-new","messages":[]}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let (headers, body) = f.requests.try_recv().unwrap();
    assert_eq!(body["model"], "model-new");
    assert_eq!(headers["x-api-key"], "fixture-secret-for-beta");
    assert!(!headers.contains_key("authorization"));
    assert_eq!(
        client
            .post(format!("{address}/v1/messages"))
            .header("x-api-key", token)
            .json(&json!({"model":"ccs-claude-b--model-b","messages":[]}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert!(f.requests.try_recv().is_err());
    let fresh = f.revision().await;
    assert_eq!(f.change(&fresh, "model-two").await, StatusCode::OK);
    assert_eq!(
        client
            .post(format!("{address}/v1/messages"))
            .header("x-api-key", token)
            .json(&json!({"model":"ccs-claude-b--model-new","messages":[]}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let before = f.bytes();
    assert_eq!(f.change(&revision, "late").await, StatusCode::CONFLICT);
    assert_eq!(f.bytes(), before);
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
    f.close().await;
}
#[tokio::test]
async fn aggregation_t16_unavailable_listener_and_detached_intent_preserve_native_config() {
    let f = Fixture::new(true).await;
    let initial = f.revision().await;
    assert_eq!(f.change(&initial, "model-first").await, StatusCode::OK);
    let revision = f.revision().await;
    let before = f.bytes();
    let published: Value = serde_json::from_slice(&before).unwrap();
    assert!(published.get("modelPicker").is_some());
    f.listener.stop().await.unwrap();
    assert_eq!(f.change(&revision, "model-new").await, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(f.bytes(), before);
    assert_eq!(f.revision().await, revision);
    f.close().await;
    let f = Fixture::new(false).await;
    let revision = f.revision().await;
    let before = f.bytes();
    assert_eq!(f.change(&revision, "model-new").await, StatusCode::OK);
    assert_eq!(f.bytes(), before);
    assert_ne!(f.revision().await, revision);
    f.close().await;
}
#[tokio::test]
async fn aggregation_t16_default_and_member_order_use_same_transaction() {
    let f = Fixture::new(true).await;
    let revision = f.revision().await;
    let store = AppProviderStore::new(f.db.clone()).unwrap();
    let mut headers = HeaderMap::new();
    headers.insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());
    let configured = super::aggregation_resync::configure(
        store,
        &f.db,
        Some(&f.policy),
        &headers,
        &tenant(),
        "host",
        AppId::Claude,
        AggregationWriteQuery {
            target_host_id: None,
            project_id: Some("project".into()),
            expected_revision: Some(revision),
        },
        Some(f.control.clone()),
    )
    .unwrap();
    configured
        .set_presentation(
            &tenant(),
            AppId::Claude,
            "beta",
            &vw_api_types::app_providers::presentation::AppProviderPresentation {
                sort_index: Some(0),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(f.tx.state(AppId::Claude).unwrap().stack.members, vec!["beta", "alpha"]);
    let configured = super::aggregation_resync::configure(
        AppProviderStore::new(f.db.clone()).unwrap(),
        &f.db,
        Some(&f.policy),
        &headers,
        &tenant(),
        "host",
        AppId::Claude,
        AggregationWriteQuery {
            target_host_id: None,
            project_id: Some("project".into()),
            expected_revision: Some(f.revision().await),
        },
        Some(f.control.clone()),
    )
    .unwrap();
    let context = configured.resync_context().unwrap();
    let mut candidate = f.tx.state(AppId::Claude).unwrap();
    candidate.mode.as_mut().unwrap().proxy_route = Some("beta".into());
    let mut unavailable = context.as_ref().clone();
    unavailable.routing = None;
    assert_eq!(unavailable.publish_candidate(&candidate), Err(StatusCode::SERVICE_UNAVAILABLE));
    assert!(f.tx.state(AppId::Claude).unwrap() == context.expected);
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
    assert!(context.publish_candidate(&candidate).unwrap());
    let state = f.tx.state(AppId::Claude).unwrap();
    assert_eq!(state.stack.members, vec!["beta", "alpha"]);
    assert_eq!(state.mode.unwrap().proxy_route.as_deref(), Some("beta"));
    let live: Value = serde_json::from_slice(&f.bytes()).unwrap();
    assert_eq!(live["modelPicker"]["options"][0]["model"], "ccs-claude-b--model-b");
    f.close().await;
}

#[tokio::test]
async fn aggregation_t16_project_write_grant_is_checked_before_record_or_file_changes() {
    let f = Fixture::new(true).await;
    let revision = f.revision().await;
    let before = f.bytes();
    let old = AppProviderStore::preview_record(&f.db, &tenant(), AppId::Claude, "beta").unwrap();
    let mut policy: Value = serde_json::from_slice(&fs::read(&f.policy).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["aggregation_write"] = json!([]);
    fs::write(&f.policy, policy.to_string()).unwrap();
    assert_eq!(f.change(&revision, "denied").await, StatusCode::FORBIDDEN);
    assert_eq!(f.bytes(), before);
    assert!(
        AppProviderStore::preview_record(&f.db, &tenant(), AppId::Claude, "beta").unwrap() == old
    );
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
    f.close().await;
}

#[tokio::test]
async fn aggregation_t16_database_failure_keeps_durable_intent_and_revokes_partial_route() {
    let f = Fixture::new(true).await;
    let revision = f.revision().await;
    assert_eq!(f.change(&revision, "model-new").await, StatusCode::OK);
    let prior: Value = serde_json::from_slice(&f.bytes()).unwrap();
    let address = prior["env"]["ANTHROPIC_BASE_URL"].as_str().unwrap();
    let old_token = prior["env"]["ANTHROPIC_AUTH_TOKEN"].as_str().unwrap();
    f.db.connect_sqlite().unwrap().execute_batch("CREATE TRIGGER t16_record_failure BEFORE UPDATE ON gateway_settings WHEN NEW.namespace='app_providers.records' AND instr(NEW.payload_json,'model-failure')>0 AND instr(NEW.payload_json,'shiju_provider_publication')=0 BEGIN SELECT RAISE(ABORT,'fixture record failure'); END;").unwrap();
    let revision = f.revision().await;
    assert_eq!(f.change(&revision, "model-failure").await, StatusCode::BAD_GATEWAY);
    assert!(f.tx.pending(AppId::Claude).unwrap().unwrap().published);
    let record = AppProviderStore::preview_record(&f.db, &tenant(), AppId::Claude, "beta").unwrap();
    assert_eq!(record["claude"]["model"], "model-new");
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    assert_eq!(
        client
            .post(format!("{address}/v1/messages"))
            .header("x-api-key", old_token)
            .json(&json!({"model":"ccs-claude-b--model-new","messages":[]}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        f.request(
            "GET",
            "/agent-apps/claude/aggregation/state?targetHostId=host&projectId=project",
            Value::Null
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    f.db.connect_sqlite().unwrap().execute_batch("DROP TRIGGER t16_record_failure;").unwrap();
    // The first authorized retry only recovers the accepted intent, then requires a fresh revision.
    assert_eq!(f.change(&revision, "model-failure").await, StatusCode::CONFLICT);
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
    let record = AppProviderStore::preview_record(&f.db, &tenant(), AppId::Claude, "beta").unwrap();
    assert_eq!(record["claude"]["model"], "model-failure");
    let fresh = f.revision().await;
    assert_eq!(f.change(&fresh, "model-failure").await, StatusCode::OK);
    let live: Value = serde_json::from_slice(&f.bytes()).unwrap();
    assert_ne!(live["env"]["ANTHROPIC_AUTH_TOKEN"], prior["env"]["ANTHROPIC_AUTH_TOKEN"]);
    assert_eq!(
        client
            .post(format!("{address}/v1/messages"))
            .header("x-api-key", live["env"]["ANTHROPIC_AUTH_TOKEN"].as_str().unwrap())
            .json(&json!({"model":"ccs-claude-b--model-failure","messages":[]}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    f.close().await;
}

#[tokio::test]
async fn aggregation_t16_detached_catalog_edit_does_not_create_client_directory() {
    let f = Fixture::new(false).await;
    fs::remove_dir_all(f.root.path().join(".claude")).unwrap();
    let revision = f.revision().await;
    assert_eq!(f.change(&revision, "planned-model").await, StatusCode::OK);
    assert!(!f.root.path().join(".claude").exists());
    assert_eq!(
        AppProviderStore::preview_record(&f.db, &tenant(), AppId::Claude, "beta").unwrap()["claude"]
            ["model"],
        "planned-model"
    );
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
    f.close().await;
}

#[tokio::test]
async fn aggregation_t16_legacy_catalog_save_keeps_gateway_and_provider_credentials_separate() {
    let f = Fixture::new(false).await;
    let private = f.root.path().join("legacy-private");
    fs::create_dir(&private).unwrap();
    fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
    let _legacy =
        ProviderTransactions::new(&private, ExecutionHostId::new("host").unwrap()).unwrap();
    let mut policy: Value = serde_json::from_slice(&fs::read(&f.policy).unwrap()).unwrap();
    policy["hosts"][0]["credential_root"] = json!(private);
    fs::write(&f.policy, policy.to_string()).unwrap();
    let before = f.bytes();
    let response = f.router.clone().oneshot(
        Request::builder()
            .method("PATCH")
            .uri("/app-providers/claude/beta/form")
            .header(PLATFORM_ID_HEADER, "platform")
            .header(COMPANY_ID_HEADER, "company")
            .header("authorization", "Basic Z2F0ZXdheTpmaXh0dXJl")
            .header("x-agent-apps-token", TOKEN)
            .header("content-type", "application/json")
            .body(Body::from(json!({"targetHostId":"host","fields":{"name":"beta"},"config":{"kind":"third_party","baseUrl":f.endpoint,"apiKeyHeader":true,"apiKey":{"action":"keep"}},"model":"legacy-model","apply":false}).to_string()))
            .unwrap()
    ).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(f.bytes(), before);
    assert_eq!(
        AppProviderStore::preview_record(&f.db, &tenant(), AppId::Claude, "beta").unwrap()["claude"]
            ["model"],
        "legacy-model"
    );
    f.close().await;
}
