use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::{Extension, Json, Router, extract::State, http::StatusCode, middleware, routing::post};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    os::unix::fs::{MetadataExt, PermissionsExt},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use vw_agent_apps::providers::{live::commit::*, mode_apply::*};
use vw_api_types::{agent_apps::AppId, app_providers::mode_apply::*, id::TenantScope};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};
const TOKEN: &str = "fixture-mode-authorized-token-32-characters";
fn tenant() -> TenantScope {
    TenantScope::new("p", "a").unwrap()
}
struct RuntimeState {
    home: std::path::PathBuf,
    url: String,
    active: Mutex<Option<String>>,
    published: Mutex<Vec<Option<String>>>,
    fail_publish: AtomicBool,
    ready: AtomicBool,
}
fn valid_scope(value: &Value) -> bool {
    value["host"] == "host" && value["app"] == "claude" && value["tenant"] == json!(tenant())
}
async fn ready(
    State(state): State<Arc<RuntimeState>>,
    Json(value): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    assert!(valid_scope(&value));
    if !state.ready.load(Ordering::SeqCst) {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    Ok(Json(json!({"clientBaseUrl":state.url,"scope":value})))
}
async fn publish(
    State(state): State<Arc<RuntimeState>>,
    Json(value): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    assert!(valid_scope(&value["scope"]));
    let provider = value["provider"].as_str();
    // The HTTP service independently checks actual files AND durable state before
    // accepting its routing target. A return value/call count cannot satisfy this.
    let file: Value =
        serde_json::from_slice(&std::fs::read(state.home.join(".claude/settings.json")).unwrap())
            .unwrap();
    assert_eq!(
        file["env"]["ANTHROPIC_BASE_URL"],
        if provider.is_some() { state.url.as_str() } else { "https://direct.example" }
    );
    let tx = ProviderTransactions::open_existing(
        &state.home.join("private"),
        ExecutionHostId::new("host").unwrap(),
    )
    .unwrap()
    .unwrap();
    let current = tx.state(AppId::Claude).unwrap();
    assert_eq!(current.mode.as_ref().unwrap().attached, provider.is_some());
    if let Some(id) = provider {
        assert_eq!(current.mode.as_ref().unwrap().proxy_route.as_deref(), Some(id));
    }
    assert!(tx.pending(AppId::Claude).unwrap().is_none());
    if state.fail_publish.load(Ordering::SeqCst) {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    *state.active.lock().unwrap() = provider.map(str::to_owned);
    state.published.lock().unwrap().push(provider.map(str::to_owned));
    Ok(Json(json!({"accepted":true,"provider":value["provider"]})))
}
async fn messages(
    State(state): State<Arc<RuntimeState>>,
    headers: axum::http::HeaderMap,
    Json(value): Json<Value>,
) -> Json<Value> {
    assert_eq!(headers.get("x-api-key").unwrap(), CLIENT_TOKEN_PLACEHOLDER);
    assert_eq!(value["model"], "fixture-model");
    assert_eq!(value["max_tokens"], 16);
    assert_eq!(value["messages"][0]["role"], "user");
    assert_eq!(value["messages"][0]["content"], "ping");
    assert!(state.active.lock().unwrap().is_some());
    Json(
        json!({"id":"fixture-message","type":"message","role":"assistant","model":"fixture-model","content":[{"type":"text","text":"pong"}],"stop_reason":"end_turn","usage":{"input_tokens":1,"output_tokens":1}}),
    )
}
struct HttpRuntime {
    url: String,
    client: reqwest::Client,
    handle: tokio::runtime::Handle,
}
impl HttpRuntime {
    fn scope(scope: &RouteScope<'_>) -> Value {
        json!({"tenant":scope.tenant,"host":scope.host,"app":scope.app})
    }
    fn request(&self, path: &str, body: Value) -> Result<Value, ModeApplyError> {
        self.handle.block_on(async {
            self.client
                .post(format!("{}{path}", self.url))
                .json(&body)
                .send()
                .await
                .map_err(|_| ModeApplyError::Unavailable)?
                .error_for_status()
                .map_err(|_| ModeApplyError::Unavailable)?
                .json()
                .await
                .map_err(|_| ModeApplyError::Unavailable)
        })
    }
}
impl RouteRuntime for HttpRuntime {
    fn prepare(&self, scope: &RouteScope<'_>) -> Result<ClientEndpoint, ModeApplyError> {
        let body = Self::scope(scope);
        let response = self.request("/control/ready", body.clone())?;
        if response["scope"] != body {
            return Err(ModeApplyError::Unavailable);
        }
        ClientEndpoint::new(response["clientBaseUrl"].as_str().ok_or(ModeApplyError::Unavailable)?)
    }
    fn publish(
        &self,
        scope: &RouteScope<'_>,
        provider: Option<&str>,
    ) -> Result<(), ModeApplyError> {
        let response = self
            .request("/control/target", json!({"scope":Self::scope(scope),"provider":provider}))?;
        if response["accepted"] != true || response["provider"] != json!(provider) {
            return Err(ModeApplyError::Unavailable);
        }
        Ok(())
    }
}
struct Fixture {
    root: tempfile::TempDir,
    db: GlobalDatabase,
    client: GatewayClient,
    server: tokio::task::JoinHandle<()>,
    runtime_server: tokio::task::JoinHandle<()>,
    runtime: Arc<RuntimeState>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
        self.runtime_server.abort();
    }
}
impl Fixture {
    async fn new(inject: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::create_dir(root.path().join("private")).unwrap();
        std::fs::set_permissions(
            root.path().join("private"),
            std::fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        let policy = root.path().join("access.json");
        std::fs::write(&policy,json!({"principals":[{"id":"mode","token_sha256":hex::encode(Sha256::digest(TOKEN)),"grants":[{"platform_id":"p","company_id":"a","host_id":"host","provider_config_read":true,"provider_config_write":true}]}],"hosts":[{"id":"host","online":true,"binding":"local","home":root.path(),"credential_root":root.path().join("private"),"search_paths":[],"applications":root.path().join("Applications")}]}).to_string()).unwrap();
        std::fs::set_permissions(&policy, std::fs::Permissions::from_mode(0o600)).unwrap();
        let db = GlobalDatabase::for_home(root.path()).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let runtime = Arc::new(RuntimeState {
            home: root.path().into(),
            url: url.clone(),
            active: Mutex::new(None),
            published: Mutex::new(vec![]),
            fail_publish: AtomicBool::new(false),
            ready: AtomicBool::new(true),
        });
        let service = Router::new()
            .route("/control/ready", post(ready))
            .route("/control/target", post(publish))
            .route("/v1/messages", post(messages))
            .with_state(runtime.clone());
        let runtime_server =
            tokio::spawn(async move { axum::serve(listener, service).await.unwrap() });
        let mut router = super::mode_apply::router_with_policy(Some(policy));
        if inject {
            router = router.layer(Extension(super::mode_apply::ProviderRouteRuntime(Arc::new(
                HttpRuntime {
                    url,
                    client: reqwest::Client::builder()
                        .no_proxy()
                        .timeout(std::time::Duration::from_secs(3))
                        .build()
                        .unwrap(),
                    handle: tokio::runtime::Handle::current(),
                },
            ))));
        }
        let router = router
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
        Self { root, db, client, server, runtime_server, runtime }
    }
    fn seed(&self) {
        let store = GatewaySettingsStore::new(self.db.clone()).unwrap();
        for id in ["direct", "route", "other"] {
            let mut env = json!({"ANTHROPIC_BASE_URL":"https://direct.example","ANTHROPIC_API_KEY":"fake-secret","ANTHROPIC_MODEL":"fixture-model"});
            if id != "direct" {
                env["CLAUDE_CODE_DISABLE_ARTIFACT"] = json!("1");
                env["CLAUDE_CODE_MAX_OUTPUT_TOKENS"] = json!("131072");
                env["CLAUDE_CODE_MAX_CONTEXT_TOKENS"] = json!("262144");
            }
            store
                .upsert(
                    &tenant(),
                    "app_providers.records",
                    &format!("\"claude\":{id}"),
                    &json!({"category":"custom","settingsConfig":{"env":env}}).to_string(),
                )
                .unwrap();
        }
        // Seed direct via existing file transaction so mode operations never invent a
        // direct provider or alter its pointer.
        std::fs::create_dir_all(self.root.path().join(".claude")).unwrap();
        let tx = ProviderTransactions::new(
            &self.root.path().join("private"),
            ExecutionHostId::new("host").unwrap(),
        )
        .unwrap();
        let mut guard = vw_agent_apps::providers::live::write_guard::lock_app(
            AppId::Claude,
            &self.root.path().join(".claude"),
        )
        .unwrap();
        tx.commit(
            &tenant(),
            &mut guard,
            &[],
            PendingTarget { pointer: Some("direct".into()), ..Default::default() },
        )
        .unwrap();
    }
    fn path(&self) -> std::path::PathBuf {
        self.root.path().join(".claude/settings.json")
    }
    fn state(&self) -> AppState {
        ProviderTransactions::open_existing(
            &self.root.path().join("private"),
            ExecutionHostId::new("host").unwrap(),
        )
        .unwrap()
        .unwrap()
        .state(AppId::Claude)
        .unwrap()
    }
    async fn apply(&self, action: ModeApplyAction) -> ModeApplyResult {
        self.client
            .provider_mode_apply(
                AppId::Claude,
                &ModeApplyRequest { target_host_id: "host".into(), project_id: None, action },
                TOKEN,
            )
            .await
            .unwrap()
    }
}
fn proxy(id: &str) -> ModeApplyAction {
    ModeApplyAction::Proxy { provider_id: id.into() }
}
#[tokio::test]
async fn cc04_mode_apply_sdk_api_http_runtime_orders_files_and_target_and_preserves_edits() {
    let f = Fixture::new(true).await;
    f.seed();
    assert!(f.apply(proxy("route")).await.attached);
    let file: Value = serde_json::from_slice(&std::fs::read(f.path()).unwrap()).unwrap();
    assert_ne!(file["env"]["ANTHROPIC_AUTH_TOKEN"], "fake-secret");
    let response:Value=reqwest::Client::new().post(format!("{}/v1/messages",file["env"]["ANTHROPIC_BASE_URL"].as_str().unwrap())).header("x-api-key",file["env"]["ANTHROPIC_AUTH_TOKEN"].as_str().unwrap()).json(&json!({"model":"fixture-model","max_tokens":16,"messages":[{"role":"user","content":"ping"}]})).send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
    assert_eq!(response["content"][0]["text"], "pong");
    let before = std::fs::read(f.path()).unwrap();
    let meta = std::fs::metadata(f.path()).unwrap();
    assert!(!f.apply(proxy("other")).await.configuration_changed);
    assert_eq!(std::fs::read(f.path()).unwrap(), before);
    assert_eq!(std::fs::metadata(f.path()).unwrap().ino(), meta.ino());
    assert_eq!(std::fs::metadata(f.path()).unwrap().modified().unwrap(), meta.modified().unwrap());
    assert_eq!(f.state().pointer.unwrap().id, "direct");
    assert_eq!(f.runtime.active.lock().unwrap().as_deref(), Some("other"));
    let mut external = file;
    external["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"] = json!("external");
    external["env"]["CLAUDE_CODE_MAX_CONTEXT_TOKENS"] = json!("372000");
    external["userSetting"] = json!(true);
    std::fs::write(f.path(), serde_json::to_vec(&external).unwrap()).unwrap();
    assert!(!f.apply(ModeApplyAction::Detach {}).await.attached);
    let final_file: Value = serde_json::from_slice(&std::fs::read(f.path()).unwrap()).unwrap();
    assert_eq!(final_file["env"]["CLAUDE_CODE_DISABLE_ARTIFACT"], "external");
    assert_eq!(final_file["env"]["CLAUDE_CODE_MAX_CONTEXT_TOKENS"], "372000");
    assert!(final_file["env"].get("CLAUDE_CODE_MAX_OUTPUT_TOKENS").is_none());
    assert_eq!(final_file["userSetting"], true);
    let mode = f.state().mode.unwrap();
    assert!(mode.mode == Mode::Proxy);
    assert!(!mode.attached);
    assert_eq!(mode.proxy_route.as_deref(), Some("other"));
    f.apply(ModeApplyAction::Direct {}).await;
    assert!(f.state().mode.unwrap().mode == Mode::Direct);
}
#[tokio::test]
async fn cc04_mode_apply_http_publication_failure_retries_committed_configuration() {
    let f = Fixture::new(true).await;
    f.seed();
    f.runtime.fail_publish.store(true, Ordering::SeqCst);
    let result = f.apply(proxy("route")).await;
    assert_eq!(result.target_publication, TargetPublication::Pending);
    assert!(result.attached);
    assert!(f.runtime.active.lock().unwrap().is_none());
    let bytes = std::fs::read(f.path()).unwrap();
    let meta = std::fs::metadata(f.path()).unwrap();
    f.runtime.fail_publish.store(false, Ordering::SeqCst);
    let result = f.apply(proxy("route")).await;
    assert_eq!(result.target_publication, TargetPublication::Published);
    assert!(!result.configuration_changed);
    assert_eq!(std::fs::read(f.path()).unwrap(), bytes);
    assert_eq!(std::fs::metadata(f.path()).unwrap().ino(), meta.ino());
}
#[tokio::test]
async fn cc04_mode_apply_denial_unavailable_and_failed_readiness_do_not_write() {
    for inject in [false, true] {
        let f = Fixture::new(inject).await;
        if inject {
            f.runtime.ready.store(false, Ordering::SeqCst);
        }
        for (token, host, status) in [
            (TOKEN, "host", if inject { 503 } else { 403 }),
            ("wrong", "host", 401),
            (TOKEN, "foreign", 403),
        ] {
            let error = f
                .client
                .provider_mode_apply(
                    AppId::Claude,
                    &ModeApplyRequest {
                        target_host_id: host.into(),
                        project_id: None,
                        action: proxy("route"),
                    },
                    token,
                )
                .await
                .unwrap_err();
            assert!(
                matches!(error,vw_gateway_client::AgentAppsReadError::Http{status:s,..} if s==status),
                "{error:?}"
            );
        }
        assert!(!f.root.path().join(".claude").exists());
        assert!(!f.root.path().join("private/provider-live").exists());
        assert!(f.runtime.published.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn cc04_mode_apply_write_grant_foreign_owner_and_unsupported_input_are_rejected() {
    let f = Fixture::new(true).await;
    for (app, action, status) in
        [(AppId::Codex, proxy("route"), 422), (AppId::Claude, proxy("../bad"), 400)]
    {
        let error = f
            .client
            .provider_mode_apply(
                app,
                &ModeApplyRequest { target_host_id: "host".into(), project_id: None, action },
                TOKEN,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(error,vw_gateway_client::AgentAppsReadError::Http{status:s,..} if s==status)
        );
    }
    assert!(!f.root.path().join(".claude").exists());
    assert!(!f.root.path().join("private/provider-live").exists());
    let policy = f.root.path().join("access.json");
    let original = std::fs::read(&policy).unwrap();
    let mut denied: Value = serde_json::from_slice(&original).unwrap();
    denied["principals"][0]["grants"][0]["provider_config_write"] = json!(false);
    std::fs::write(&policy, serde_json::to_vec(&denied).unwrap()).unwrap();
    let error = f
        .client
        .provider_mode_apply(
            AppId::Claude,
            &ModeApplyRequest {
                target_host_id: "host".into(),
                project_id: None,
                action: proxy("route"),
            },
            TOKEN,
        )
        .await
        .unwrap_err();
    assert!(matches!(error, vw_gateway_client::AgentAppsReadError::Http { status: 403, .. }));
    assert!(!f.root.path().join(".claude").exists());
    assert!(!f.root.path().join("private/provider-live").exists());
    std::fs::write(&policy, original).unwrap();
    f.seed();
    let tx = ProviderTransactions::open_existing(
        &f.root.path().join("private"),
        ExecutionHostId::new("host").unwrap(),
    )
    .unwrap()
    .unwrap();
    let guard = vw_agent_apps::providers::live::write_guard::lock_app(
        AppId::Claude,
        &f.root.path().join(".claude"),
    )
    .unwrap();
    // A recorded pointer belonging to a different tenant must not be reused by an
    // otherwise host-authorized request, including a pending transaction recovery.
    let mut state = f.state();
    state.pointer.as_mut().unwrap().tenant = TenantScope::new("p", "foreign").unwrap();
    let state_path = f.root.path().join("private/provider-live/aG9zdA/claude.state.json");
    std::fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    drop(guard);
    let before = std::fs::read(&state_path).unwrap();
    let error = f
        .client
        .provider_mode_apply(
            AppId::Claude,
            &ModeApplyRequest {
                target_host_id: "host".into(),
                project_id: None,
                action: proxy("route"),
            },
            TOKEN,
        )
        .await
        .unwrap_err();
    assert!(matches!(error, vw_gateway_client::AgentAppsReadError::Http { status: 403, .. }));
    assert_eq!(std::fs::read(state_path).unwrap(), before);
    assert!(!f.path().exists());
    assert!(tx.pending(AppId::Claude).unwrap().is_none());
    assert!(f.runtime.published.lock().unwrap().is_empty());
}

struct ProductionFixture {
    _gateway: super::super::app_routing::lifecycle_api_tests::GatewayProcess,
    home: tempfile::TempDir,
    client: GatewayClient,
    http: reqwest::Client,
    mode_url: String,
    state: std::path::PathBuf,
}

impl ProductionFixture {
    async fn new(expires_at: u64) -> Self {
        use super::super::app_routing::lifecycle_api_tests::{
            HOST, MANAGER, client, spawn_gateway, write_policy,
        };
        use crate::{
            agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountScope},
            app_providers::{AppProviderStore, ClaudeBindingHost},
        };
        use vw_agent_apps::auth::credential_file::{AccountCredential, CredentialFileStore};
        use vw_api_types::{
            TimestampMs,
            agent_apps_auth::AuthService,
            app_providers::claude::{ClaudeConfig, ClaudeProfile},
        };

        let home = tempfile::tempdir().unwrap();
        let root = home.path().join("credentials");
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let policy = home.path().join("access.json");
        write_policy(&policy, home.path(), 0);
        let mut value: Value = serde_json::from_slice(&std::fs::read(&policy).unwrap()).unwrap();
        let grant = &mut value["principals"][0]["grants"][0];
        grant["provider_config_write"] = json!(true);
        grant["routing_apps"] = json!([{"project_id":"allowed","app_id":"claude"}]);
        std::fs::write(&policy, value.to_string()).unwrap();

        let tenant = TenantScope::new("p", "owner").unwrap();
        let db = GlobalDatabase::for_home(home.path()).unwrap();
        let store = AppProviderStore::new(db.clone()).unwrap();
        store
            .claude_save(
                &tenant,
                "route",
                &ClaudeProfile {
                    name: "Bound Claude".into(),
                    config: ClaudeConfig::Official { account: None },
                    model: None,
                },
                None,
                true,
            )
            .unwrap();
        let settings = GatewaySettingsStore::new(db).unwrap();
        settings.upsert(&tenant, "app_providers.records", "\"claude\":direct",
            &json!({"category":"custom","settingsConfig":{"env":{"ANTHROPIC_BASE_URL":"https://direct.example","ANTHROPIC_API_KEY":"fixture-key"}}}).to_string()).unwrap();

        let scope = AuthAccountScope {
            tenant: &tenant,
            principal: MANAGER,
            host: HOST,
            service: AuthService::ClaudeOfficial,
        };
        let accounts = store.claude_accounts(&root).unwrap();
        let credentials =
            CredentialFileStore::new(&root).unwrap().for_owner(scope.principal).unwrap();
        credentials.register_claude_directory(&tenant, HOST, "bound").unwrap();
        let AccountCredential::ClaudeDirectory(directory) =
            credentials.read(&tenant, HOST, AppId::Claude, "bound").unwrap()
        else {
            panic!("expected a managed account");
        };
        for (name, value) in [
            (
                ".claude.json",
                json!({"oauthAccount":{"accountUuid":"bound","organizationUuid":"org"}}),
            ),
            (
                ".credentials.json",
                json!({"claudeAiOauth":{"accessToken":"fixture-access","refreshToken":"fixture-refresh","expiresAt":expires_at}}),
            ),
        ] {
            let path = directory.path().join(name);
            std::fs::write(&path, value.to_string()).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        accounts
            .insert_metadata(
                &scope,
                "bound",
                &AuthAccountMetadata::new("bound:org", TimestampMs(1_800_000_000_000)).unwrap(),
            )
            .unwrap();
        store
            .bind_claude_account(
                scope,
                ClaudeBindingHost { home: home.path(), private_root: &root },
                "route",
                None,
                Some("bound"),
            )
            .unwrap();

        let native = home.path().join(".claude");
        std::fs::create_dir(&native).unwrap();
        let tx = ProviderTransactions::new(&root, ExecutionHostId::new(HOST).unwrap()).unwrap();
        let mut guard =
            vw_agent_apps::providers::live::write_guard::lock_app(AppId::Claude, &native).unwrap();
        tx.commit(
            &tenant,
            &mut guard,
            &[],
            PendingTarget { pointer: Some("direct".into()), ..Default::default() },
        )
        .unwrap();
        drop(guard);
        let state = std::fs::read_dir(root.join("provider-live"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path()
            .join("claude.state.json");

        let (_gateway, port) = spawn_gateway(home.path(), &policy).await;
        Self {
            _gateway,
            home,
            client: client(port, "owner"),
            http: reqwest::Client::builder().no_proxy().build().unwrap(),
            mode_url: format!("http://127.0.0.1:{port}/v1/app-providers/claude/mode/apply"),
            state,
        }
    }

    async fn mode(&self, project: Option<&str>, action: Value) -> reqwest::Response {
        use super::super::app_routing::lifecycle_api_tests::{HOST, MANAGER};
        let mut body = json!({"targetHostId":HOST,"action":action});
        if let Some(project) = project {
            body["projectId"] = json!(project);
        }
        self.http
            .post(&self.mode_url)
            .header("x-platform-id", "p")
            .header("x-company-id", "owner")
            .header("x-agent-apps-token", MANAGER)
            .json(&body)
            .send()
            .await
            .unwrap()
    }

    fn configuration(&self) -> std::path::PathBuf {
        self.home.path().join(".claude/settings.json")
    }
}

#[tokio::test]
async fn cc_route_109_authorized_preparation_uses_gateway_listener_and_preserves_direct() {
    use super::super::app_routing::lifecycle_api_tests::{HOST, MANAGER};
    let f = ProductionFixture::new(4_000_000_000_000).await;
    let proxy = json!({"kind":"proxy","providerId":"route"});
    let binding = f.client.app_routing_start(HOST, MANAGER).await.unwrap();
    let port = binding.bound_port.unwrap();
    let prepared = f.mode(Some("allowed"), proxy.clone()).await;
    assert_eq!(prepared.status(), 503);
    assert_eq!(
        prepared.json::<Value>().await.unwrap()["error"]["code"],
        "route_publication_unavailable"
    );

    let direct = f.mode(None, json!({"kind":"direct"})).await;
    assert_eq!(direct.status(), 200, "original direct request must work");
    let direct = direct.json::<ModeApplyResult>().await.unwrap();
    assert!(!direct.attached);
    let before = std::fs::read(f.configuration()).unwrap();
    let state = std::fs::read(&f.state).unwrap();
    for _ in 0..2 {
        let prepared = f.mode(Some("allowed"), proxy.clone()).await;
        assert_eq!(prepared.status(), 503);
        assert_eq!(
            prepared.json::<Value>().await.unwrap()["error"]["code"],
            "route_publication_unavailable"
        );
        assert_eq!(
            f.client.app_routing_status(HOST, MANAGER).await.unwrap().bound_port,
            Some(port)
        );
    }
    let data = f
        .http
        .post(format!("http://127.0.0.1:{port}/v1/messages"))
        .header("x-api-key", CLIENT_TOKEN_PLACEHOLDER)
        .send()
        .await
        .unwrap();
    assert_eq!(data.status(), 401);
    assert_eq!(std::fs::read(f.configuration()).unwrap(), before);
    assert_eq!(std::fs::read(&f.state).unwrap(), state);
    f.client.app_routing_stop(HOST, MANAGER).await.unwrap();
    let stopped = f.mode(Some("allowed"), proxy).await;
    assert_eq!(stopped.status(), 503);
    assert!(stopped.json::<Value>().await.is_err());
}

#[tokio::test]
async fn cc_route_109_missing_project_or_stale_account_generation_never_writes() {
    use super::super::app_routing::lifecycle_api_tests::{HOST, MANAGER};
    let f = ProductionFixture::new(1_700_000_000_000).await;
    let state = std::fs::read(&f.state).unwrap();
    f.client.app_routing_start(HOST, MANAGER).await.unwrap();
    for (project, status) in [(None, 403), (Some("denied"), 403), (Some("allowed"), 409)] {
        let response = f.mode(project, json!({"kind":"proxy","providerId":"route"})).await;
        assert_eq!(response.status(), status, "project {project:?}");
        assert!(!f.configuration().exists());
        assert_eq!(std::fs::read(&f.state).unwrap(), state);
        assert!(!f.state.with_file_name("claude.pending.json").exists());
    }
}

#[tokio::test]
async fn ha_enable_requires_routing_mode_and_keeps_queue_http() {
    use super::super::app_routing::lifecycle_api_tests::{HOST, MANAGER};
    use crate::app_providers::AppProviderStore;
    use vw_api_types::app_providers::claude::{ClaudeConfig, ClaudeProfile};

    let f = ProductionFixture::new(4_000_000_000_000).await;
    let db = GlobalDatabase::for_home(f.home.path()).unwrap();
    let store = AppProviderStore::new(db).unwrap();
    let tenant = TenantScope::new("p", "owner").unwrap();
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", upstream.local_addr().unwrap());
    let requests = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = Arc::clone(&requests);
    let server = tokio::spawn(async move {
        let app = Router::new().route("/v1/messages", post(move |headers: axum::http::HeaderMap| {
            let observed = Arc::clone(&observed);
            async move {
                assert_eq!(headers["x-api-key"], "fixture-route-key");
                observed.fetch_add(1, Ordering::SeqCst);
                Json(json!({"type":"message","role":"assistant","content":[{"type":"text","text":"OK"}]}))
            }
        }));
        axum::serve(upstream, app).await.unwrap();
    });
    for id in ["direct", "third-party", "replacement"] {
        let before = store.claude_record(&tenant, id).unwrap();
        store
            .claude_save(
                &tenant,
                id,
                &ClaudeProfile {
                    name: id.into(),
                    config: ClaudeConfig::ThirdParty {
                        base_url: endpoint.clone(),
                        api_key: "fixture-route-key".into(),
                        api_key_header: true,
                    },
                    model: Some("fixture".into()),
                },
                before.as_ref(),
                before.is_none(),
            )
            .unwrap();
    }
    let proxy = |id| json!({"kind":"proxy","providerId":id});
    assert_eq!(f.mode(None, proxy("third-party")).await.status(), 403);
    assert_eq!(f.mode(Some("denied"), proxy("third-party")).await.status(), 403);
    let binding = f.client.app_routing_start(HOST, MANAGER).await.unwrap();
    let route_url = format!("http://127.0.0.1:{}/v1/messages", binding.bound_port.unwrap());
    assert_eq!(f.mode(Some("allowed"), proxy("third-party")).await.status(), 200);
    let read_token = || {
        let config: Value =
            serde_json::from_slice(&std::fs::read(f.configuration()).unwrap()).unwrap();
        config["env"]["ANTHROPIC_AUTH_TOKEN"].as_str().unwrap().to_owned()
    };
    let token = read_token();
    let request = |token: &str| {
        f.http
            .post(&route_url)
            .header("x-api-key", token)
            .json(&json!({"model":"fixture","messages":[{"role":"user","content":"hello"}]}))
    };
    assert_eq!(request("PROXY_MANAGED").send().await.unwrap().status(), 401);
    assert_eq!(request(&token).send().await.unwrap().status(), 200);
    let before = std::fs::read(f.configuration()).unwrap();
    assert_eq!(f.mode(Some("allowed"), proxy("missing")).await.status(), 404);
    assert_eq!(std::fs::read(f.configuration()).unwrap(), before);
    assert_eq!(request(&token).send().await.unwrap().status(), 200);
    assert_eq!(f.mode(Some("allowed"), proxy("replacement")).await.status(), 200);
    assert_ne!(read_token(), token);
    assert_eq!(request(&token).send().await.unwrap().status(), 401);
    let token = read_token();
    assert_eq!(request(&token).send().await.unwrap().status(), 200);
    assert_eq!(f.mode(Some("allowed"), json!({"kind":"detach"})).await.status(), 200);
    assert_eq!(request(&token).send().await.unwrap().status(), 401);
    let state: AppState = serde_json::from_slice(&std::fs::read(&f.state).unwrap()).unwrap();
    assert!(!state.mode.as_ref().unwrap().attached);
    assert!(state.mode.as_ref().unwrap().mode == Mode::Proxy);
    assert_eq!(f.mode(Some("allowed"), proxy("replacement")).await.status(), 200);
    let token = read_token();
    assert_eq!(f.mode(Some("allowed"), json!({"kind":"direct"})).await.status(), 200);
    assert_eq!(request(&token).send().await.unwrap().status(), 401);
    assert_eq!(requests.load(Ordering::SeqCst), 3);
    f.client.app_routing_stop(HOST, MANAGER).await.unwrap();
    server.abort();
    let _ = server.await;
}
