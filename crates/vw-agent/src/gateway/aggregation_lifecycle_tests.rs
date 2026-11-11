use super::*;
use crate::app_routing::RoutingListener;
use std::sync::Arc;

async fn published_claude() -> (
    Fixture,
    Arc<RoutingListener>,
    tokio::task::JoinHandle<()>,
    tokio::sync::mpsc::UnboundedReceiver<Value>,
    String,
) {
    let mut f = Fixture::new();
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", upstream.local_addr().unwrap());
    let (send, receive) = tokio::sync::mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        axum::serve(upstream, Router::new().route("/v1/messages", axum::routing::post(move |axum::Json(body): axum::Json<Value>| {
            let send = send.clone(); async move {
                let _ = send.send(body.clone());
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
                        base_url: origin.clone(),
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
    let (status, progress) = f.publish(f.publish_request().await).await;
    assert_eq!(status, 202, "{progress}");
    let terminal = f.settled(progress["operationId"].as_str().unwrap()).await;
    assert_eq!(terminal["status"], "completed", "{terminal}");
    (f, listener, server, receive, origin)
}

#[tokio::test]
async fn aggregation_t19_aggregation_lifecycle_preserves_contract() {
    let (f, listener, server, _, origin) = published_claude().await;
    listener.stop().await.unwrap();
    let state = f.tx.state(AppId::Claude).unwrap();
    assert!(
        !state.mode.as_ref().unwrap().attached,
        "service stop must detach the client before listener shutdown"
    );
    assert!(state.stack.enabled, "service stop must retain startup intent");
    let live: Value = serde_json::from_slice(&std::fs::read(f.native()).unwrap()).unwrap();
    assert_eq!(live["env"]["ANTHROPIC_BASE_URL"], origin);
    server.abort();
}

fn configure_lifecycle(f: &Fixture, listener: &Arc<RoutingListener>) {
    listener.set_aggregation_lifecycle(
        crate::gateway::aggregation_lifecycle::AggregationLifecycle {
            database: f.db.clone(),
            policy: f.policy.clone(),
            tenant: tenant(),
            host: "host".into(),
        },
    );
}
fn native(f: &Fixture) -> Value {
    serde_json::from_slice(&std::fs::read(f.native()).unwrap()).unwrap()
}
async fn request_model(
    listener: &RoutingListener,
    settings: &Value,
    model: &str,
) -> reqwest::Response {
    let address = listener.bound_address().unwrap();
    reqwest::Client::new()
        .post(format!("http://{address}/v1/messages"))
        .bearer_auth(settings["env"]["ANTHROPIC_AUTH_TOKEN"].as_str().unwrap())
        .json(
            &json!({"model":model,"max_tokens":8,"messages":[{"role":"user","content":"fixture"}]}),
        )
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn aggregation_t19_restart_rotates_credentials_and_repeated_events_preserve_files() {
    let (f, listener, server, mut receive, origin) = published_claude().await;
    let before = native(&f);
    let id = before["modelPicker"]["options"][0]["model"].as_str().unwrap();
    assert_eq!(request_model(&listener, &before, id).await.status(), 200);
    assert_eq!(receive.recv().await.unwrap()["model"], "first");
    let projection = f.tx.state(AppId::Claude).unwrap().aggregation_recovery.unwrap();
    assert_eq!(projection.published["modelPicker"], before["modelPicker"]);
    listener.stop().await.unwrap();
    let detached = std::fs::read(f.native()).unwrap();
    listener.stop().await.unwrap();
    assert_eq!(std::fs::read(f.native()).unwrap(), detached);
    assert_eq!(native(&f)["env"]["ANTHROPIC_BASE_URL"], origin);
    // Reopen the actual private journal and create a new service owner.
    let tx = ProviderTransactions::open_existing(
        &f.root.path().join("private"),
        ExecutionHostId::new("host").unwrap(),
    )
    .unwrap()
    .unwrap();
    assert!(!tx.state(AppId::Claude).unwrap().mode.unwrap().attached);
    let restarted = Arc::new(RoutingListener::default());
    restarted.start(0).unwrap();
    configure_lifecycle(&f, &restarted);
    restarted.resume_aggregations().await.unwrap();
    let current = native(&f);
    assert_eq!(current["hooks"]["user"], true);
    assert!(current["env"]["ANTHROPIC_AUTH_TOKEN"] != before["env"]["ANTHROPIC_AUTH_TOKEN"]);
    assert_eq!(request_model(&restarted, &before, id).await.status(), 401);
    assert_eq!(request_model(&restarted, &current, id).await.status(), 200);
    assert_eq!(receive.recv().await.unwrap()["model"], "first");
    let bytes = std::fs::read(f.native()).unwrap();
    restarted.resume_aggregations().await.unwrap();
    assert_eq!(std::fs::read(f.native()).unwrap(), bytes);
    restarted.stop().await.unwrap();
    server.abort();
}

#[tokio::test]
async fn aggregation_t19_external_edit_and_revoked_permission_fail_without_overwrite() {
    let (f, listener, server, _, _) = published_claude().await;
    let mut edited = native(&f);
    edited["modelPicker"] = json!({"user":"changed"});
    std::fs::write(f.native(), edited.to_string()).unwrap();
    let bytes = std::fs::read(f.native()).unwrap();
    assert!(matches!(
        listener.stop().await,
        Err(crate::app_routing::ListenerError::AggregationRecovery(StatusCode::CONFLICT))
    ));
    assert_eq!(std::fs::read(f.native()).unwrap(), bytes);
    assert!(f.tx.state(AppId::Claude).unwrap().mode.unwrap().attached);
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
    f.policy(true, false, true);
    let restarted = Arc::new(RoutingListener::default());
    restarted.start(0).unwrap();
    configure_lifecycle(&f, &restarted);
    assert!(matches!(
        restarted.resume_aggregations().await,
        Err(crate::app_routing::ListenerError::AggregationRecovery(StatusCode::FORBIDDEN))
    ));
    assert_eq!(std::fs::read(f.native()).unwrap(), bytes);
    assert!(restarted.stop().await.is_err());
    server.abort();
}

#[tokio::test]
async fn aggregation_t19_gateway_owner_shutdown_restores_its_client() {
    let (f, first, server, _, origin) = published_claude().await;
    first.stop().await.unwrap();
    let (signal, ready) = tokio::sync::oneshot::channel::<()>();
    crate::app_routing::gateway_owner::run_with_routing(
        async |routing| {
            routing.start(0).unwrap();
            configure_lifecycle(&f, routing);
            routing.resume_aggregations().await.unwrap();
            assert!(f.tx.state(AppId::Claude).unwrap().mode.unwrap().attached);
            signal.send(()).unwrap();
            std::future::pending::<anyhow::Result<()>>().await
        },
        async {
            ready.await.unwrap();
            Ok(())
        },
    )
    .await
    .unwrap();
    assert!(!f.tx.state(AppId::Claude).unwrap().mode.unwrap().attached);
    assert_eq!(native(&f)["env"]["ANTHROPIC_BASE_URL"], origin);
    server.abort();
}

#[tokio::test]
async fn aggregation_t19_codex_restores_selected_provider_and_reconnects_catalog() {
    let mut f = super::clients::client(AppId::Codex, "codex");
    let store = GatewaySettingsStore::new(f.db.clone()).unwrap();
    let mut direct_row = crate::app_providers::AppProviderStore::preview_record(
        &f.db,
        &tenant(),
        AppId::Codex,
        "alpha",
    )
    .unwrap();
    direct_row["settingsConfig"]["config"] = json!(format!(
        "model_catalog_json='external.json'\n{}",
        direct_row["settingsConfig"]["config"].as_str().unwrap()
    ));
    store
        .upsert(&tenant(), "app_providers.records", "\"codex\":alpha", &direct_row.to_string())
        .unwrap();
    std::fs::create_dir(f.root.path().join(".codex")).unwrap();
    let config = f.root.path().join(".codex/config.toml");
    std::fs::write(&config,"model='first'\nmodel_provider='relay'\nmodel_catalog_json='external.json'\n[model_providers.relay]\nbase_url='https://fixture.invalid/v1'\nwire_api='responses'\n[history]\npersistence='save-all'\n").unwrap();
    let listener = Arc::new(RoutingListener::default());
    listener.start(0).unwrap();
    f.routing = Some(listener.clone());
    let request = super::clients::intent(&f, "codex", "use_managed").await;
    let (status, progress) =
        f.request("POST", "/agent-apps/codex/aggregation/publish", Some(TOKEN), request).await;
    assert_eq!(status, 202, "{progress}");
    let terminal = f.settled(progress["operationId"].as_str().unwrap()).await;
    assert_eq!(terminal["status"], "completed", "{terminal}");
    listener.stop().await.unwrap();
    let direct: Value = toml::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
    assert_eq!(
        direct["model_providers"][direct["model_provider"].as_str().unwrap()]["base_url"],
        "https://fixture.invalid/v1"
    );
    assert_eq!(direct["model_catalog_json"], "external.json");
    assert_eq!(direct["history"]["persistence"], "save-all");
    let restarted = Arc::new(RoutingListener::default());
    restarted.start(0).unwrap();
    configure_lifecycle(&f, &restarted);
    let original = std::fs::read_to_string(&config).unwrap();
    std::fs::write(&config, original.replace("external.json", "other.json")).unwrap();
    let changed = std::fs::read(&config).unwrap();
    assert!(restarted.resume_aggregations().await.is_err());
    assert_eq!(std::fs::read(&config).unwrap(), changed);
    assert!(!f.tx.state(AppId::Codex).unwrap().mode.unwrap().attached);
    std::fs::write(&config, original).unwrap();
    restarted.resume_aggregations().await.unwrap();
    assert!(f.tx.state(AppId::Codex).unwrap().mode.unwrap().attached);
    let catalog: Value = serde_json::from_slice(
        &std::fs::read(f.root.path().join(".codex/shiju-model-catalog.json")).unwrap(),
    )
    .unwrap();
    assert!(
        catalog["models"]
            .as_array()
            .unwrap()
            .iter()
            .any(|model| model["slug"].as_str().unwrap().contains("second"))
    );
    restarted.stop().await.unwrap();
}

#[tokio::test]
async fn aggregation_t19_desktop_keeps_projection_then_republishes_on_start() {
    let mut f = super::clients::client(AppId::ClaudeDesktop, "claude-desktop");
    std::fs::create_dir_all(f.root.path().join("Applications/Claude.app")).unwrap();
    let listener = Arc::new(RoutingListener::default());
    listener.start(0).unwrap();
    f.routing = Some(listener.clone());
    let request = super::clients::intent(&f, "claude-desktop", "preserve").await;
    let (status, progress) = f
        .request("POST", "/agent-apps/claude-desktop/aggregation/publish", Some(TOKEN), request)
        .await;
    assert_eq!(status, 202, "{progress}");
    let terminal = f.settled(progress["operationId"].as_str().unwrap()).await;
    assert_eq!(terminal["status"], "completed", "{terminal}");
    let path=f.root.path().join("Library/Application Support/Claude-3p/configLibrary/00000000-0000-4000-8000-000000157210.json");
    let before = std::fs::read(&path).unwrap();
    listener.stop().await.unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(f.tx.state(AppId::ClaudeDesktop).unwrap().mode.unwrap().attached);
    let restarted = Arc::new(RoutingListener::default());
    restarted.start(0).unwrap();
    configure_lifecycle(&f, &restarted);
    restarted.resume_aggregations().await.unwrap();
    assert_ne!(std::fs::read(&path).unwrap(), before);
    let current = std::fs::read(&path).unwrap();
    restarted.resume_aggregations().await.unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), current);
    restarted.stop().await.unwrap();
}

struct UnexpectedRelayCall;
#[async_trait::async_trait]
impl crate::model_relay::RelayBackend for UnexpectedRelayCall {
    async fn execute(
        &self,
        _: &crate::model_relay::RelayIdentity,
        _: crate::model_relay::RelayRequest,
        _: tokio_util::sync::CancellationToken,
    ) -> Result<axum::response::Response, StatusCode> {
        panic!("a mismatched model must never reach a relay backend")
    }
}

#[tokio::test]
async fn aggregation_t19_service_stop_preserves_session_relay_capability_and_listener() {
    use crate::model_relay::{RelayBinding, RelayIdentity, RelayServer};
    let (f, listener, server, _, _) = published_claude().await;
    let directory = f.root.path().to_string_lossy().to_string();
    let identity = RelayIdentity {
        tenant: tenant(),
        project: "relay-project".into(),
        session: "session".into(),
        provider: "provider".into(),
        model: "model".into(),
        generation: 1,
    };
    let binding = RelayBinding::new(identity.clone(), std::time::Duration::from_secs(30)).unwrap();
    let context = crate::project::instance::Context {
        directory: directory.clone(),
        worktree: directory.clone(),
        project: crate::project::Info {
            id: identity.project.clone(),
            worktree: directory,
            vcs: None,
            name: None,
            icon: None,
            commands: None,
            time: crate::project::TimeInfo { created: 0, updated: 0, initialized: None },
            sandboxes: vec![],
        },
    };
    let relay =
        RelayServer::start(binding.clone(), Arc::new(UnexpectedRelayCall), context).await.unwrap();
    let client = reqwest::Client::new();
    let probe = || {
        client
            .post(format!("http://{}/v1/chat/completions", relay.address()))
            .bearer_auth(binding.bearer_token())
            .json(&json!({"model":"other-model","messages":[]}))
            .send()
    };
    assert_eq!(probe().await.unwrap().status(), 403);
    listener.stop().await.unwrap();
    assert!(binding.accepts(binding.bearer_token(), &identity));
    assert_eq!(probe().await.unwrap().status(), 403);
    relay.shutdown().await.unwrap();
    server.abort();
}

#[tokio::test]
async fn aggregation_t19_host_stop_and_start_use_authorized_router() {
    let (f, listener, server, _, origin) = published_claude().await;
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
    policy["hosts"][0]["routing_startup"] = json!({"enabled":true,"listen_port":0});
    std::fs::write(&f.policy, policy.to_string()).unwrap();
    let router = Router::new()
        .route("/stop/{host}", axum::routing::post(super::super::app_routing::stop))
        .route("/start/{host}", axum::routing::post(super::super::app_routing::start))
        .layer(Extension(tenant()));
    let router = super::super::app_routing::configure_with_policy(
        router,
        &listener,
        f.policy.clone(),
        f.db.clone(),
    )
    .await;
    let request = |path: &str, token: &str| {
        Request::builder()
            .method("POST")
            .uri(path)
            .header("x-agent-apps-token", token)
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap()
    };
    assert_eq!(
        router.clone().oneshot(request("/stop/host", "untrusted")).await.unwrap().status(),
        401
    );
    assert!(f.tx.state(AppId::Claude).unwrap().mode.unwrap().attached);
    assert_eq!(router.clone().oneshot(request("/stop/host", TOKEN)).await.unwrap().status(), 200);
    assert_eq!(native(&f)["env"]["ANTHROPIC_BASE_URL"], origin);
    assert_eq!(router.clone().oneshot(request("/start/host", TOKEN)).await.unwrap().status(), 200);
    assert!(f.tx.state(AppId::Claude).unwrap().mode.unwrap().attached);
    let bytes = std::fs::read(f.native()).unwrap();
    assert_eq!(router.clone().oneshot(request("/start/host", TOKEN)).await.unwrap().status(), 200);
    assert_eq!(std::fs::read(f.native()).unwrap(), bytes);
    assert_eq!(router.oneshot(request("/stop/host", TOKEN)).await.unwrap().status(), 200);
    server.abort();
}

struct RestorePermissions(std::path::PathBuf, std::fs::Permissions);
impl Drop for RestorePermissions {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.0, self.1.clone());
    }
}

#[tokio::test]
async fn aggregation_t19_failed_start_preserves_direct_files_and_retryable_intent() {
    let (f, listener, server, _, _) = published_claude().await;
    listener.stop().await.unwrap();
    let before = std::fs::read(f.native()).unwrap();
    let state = f.tx.state(AppId::Claude).unwrap();
    let private = f.root.path().join("private/provider-live/aG9zdA");
    let restore =
        RestorePermissions(private.clone(), std::fs::metadata(&private).unwrap().permissions());
    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o500)).unwrap();
    let restarted = Arc::new(RoutingListener::default());
    restarted.start(0).unwrap();
    configure_lifecycle(&f, &restarted);
    assert!(restarted.resume_aggregations().await.is_err());
    assert_eq!(std::fs::read(f.native()).unwrap(), before);
    assert!(f.tx.state(AppId::Claude).unwrap() == state);
    assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
    drop(restore);
    restarted.resume_aggregations().await.unwrap();
    assert!(f.tx.state(AppId::Claude).unwrap().mode.unwrap().attached);
    restarted.stop().await.unwrap();
    server.abort();
}
