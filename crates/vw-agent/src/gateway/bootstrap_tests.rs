// 验证网关启动时的路由注册、可选能力开关与路由摘要。

// 路由摘要既要反映已启用能力，也不能暴露被配置关闭的可选入口。
use super::*;

use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tower::util::ServiceExt;

// 启动路由测试只需可构造的 provider，所有响应均在进入模型前完成。
struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok("ok".to_string())
    }
}

// 状态 fixture 关闭外部依赖，只保留路由装配所需的最小共享组件。
fn state() -> AppState {
    AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(false, &[])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: Default::default(),
    }
}

#[tokio::test]
async fn external_router_registers_only_declared_webhook_methods() {
    let app = external_router().with_state(state());

    for (method, path) in [
        ("POST", "/v1/health"),
        ("GET", "/v1/webhook"),
        ("PUT", "/v1/whatsapp"),
        ("GET", "/v1/linq"),
        ("PUT", "/v1/wati"),
        ("GET", "/v1/nextcloud-talk"),
        ("GET", "/v1/qq"),
    ] {
        let request = Request::builder().method(method).uri(path).body(Body::empty()).unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED, "{method} {path}");
    }

    let response = app
        .oneshot(Request::get("/v1/not-an-external-route").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[test]
fn gateway_route_summary_lists_tenant_policy_and_client_api_entries() {
    let lines = gateway_route_summary(GatewayRouteSummaryOptions {
        whatsapp_enabled: true,
        linq_enabled: true,
        wati_enabled: true,
        nextcloud_talk_enabled: true,
        qq_webhook_enabled: true,
        node_control_enabled: true,
    });
    let text = lines.join("\n");

    assert!(text.contains("Public probes and third-party webhooks (no tenant headers):"));
    assert!(text.contains("Gateway routes: "));
    assert!(text.contains("Client APIs (require X-Platform-ID and X-Company-ID):"));
    for route in [
        "/v1/webhook",
        "/v1/whatsapp",
        "/v1/linq",
        "/v1/wati",
        "/v1/nextcloud-talk",
        "/v1/qq",
        "/v1/health",
        "/v1/metrics",
        "/v1/agent",
        "/v1/chat/completions",
        "/v1/models",
        "/v1/ws/chat",
        "/v1/events",
        "/v1/status",
        "/v1/tools",
        "/v1/mcp/inspect",
        "/v1/cron",
        "/v1/cron/{id}",
        "/v1/integrations",
        "/v1/integrations/settings",
        "/v1/integrations/{id}/credentials",
        "/v1/doctor",
        "/v1/memory",
        "/v1/memory/{key}",
        "/v1/cli-tools",
        "/v1/api-health",
        "/v1/dashboard/config",
        "/v1/hooks",
        "/v1/hooks/trust",
        "/v1/hooks/revoke",
        "/v1/hooks/audit",
        "/v1/node-control",
        "/v1/shiju/account/login",
        "/v1/shiju/account/login/poll",
        "/v1/shiju/account/profile",
        "/v1/shiju/account/monthly-quota",
        "/v1/shiju/apps/catalog",
        "/v1/shiju/apps",
        "/v1/shiju/apps/api-keys",
        "/v1/shiju/apps/{installation_id}/api-key-binding",
        "/v1/shiju/apps/{installation_id}/launch",
        "/v1/shiju/skills/search",
        "/v1/shiju/skills/installed",
        "/v1/shiju/skills/system",
        "/v1/shiju/skills/maintained",
        "/v1/shiju/skills/{skill_id}",
        "/v1/shiju/skills/{skill_id}/versions",
        "/v1/shiju/skills/{skill_id}/install",
        "/v1/agent-scheduler/projects/{project_id}/tasks",
        "/v1/session/{session_id}/message/{message_id}/part/{part_id}",
        "/v1/workflow/applications/{uuid}/chat-messages",
        "/v1/knowledge/datasets/{dataset_id}/documents/import-async",
    ] {
        assert!(text.contains(route), "missing route summary entry: {route}");
    }
}

#[test]
fn gateway_route_summary_lists_all_documented_routes() {
    let routes = documented_gateway_routes();
    assert_eq!(routes.len(), 240);
    for (method, path) in [
        ("POST", "/v1/session/{session_id}/dynamic-tools/hosts"),
        ("PUT", "/v1/session/{session_id}/dynamic-tools"),
        ("GET", "/v1/session/{session_id}/dynamic-tools/calls"),
        ("POST", "/v1/session/{session_id}/dynamic-tools/calls/{call_id}/response"),
    ] {
        assert!(
            routes
                .iter()
                .any(|route| route.method == method && route.path == path && route.requires_tenant)
        );
    }
    let unique = routes
        .iter()
        .map(|route| (route.method, route.path))
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(unique.len(), routes.len(), "documented routes must be unique");
    assert!(routes.iter().any(|route| route.path == "/v1/desktop/project-preferences"));
    assert!(routes.iter().any(|route| route.path == "/v1/session/{session_id}/title"));
    assert!(routes.iter().any(|route| route.path == "/v1/channels/{channel}/qrcode"));
    assert!(routes.iter().any(|route| route.path == "/v1/channels/{channel}/qrcode/status"));
}

#[tokio::test]
async fn stateful_handler_router_registers_channel_qr_routes() {
    let app = Router::new().nest("/v1", stateful_handler_router()).with_state(state());
    let request = Request::builder()
        .method("GET")
        .uri("/v1/channels/feishu/qrcode")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn stateful_handler_router_registers_shiju_gateway_routes() {
    let app = Router::new().nest("/v1", stateful_handler_router()).with_state(state());

    for (method, path) in [
        ("GET", "/v1/shiju/account/login"),
        ("POST", "/v1/shiju/account/profile"),
        ("POST", "/v1/shiju/apps/catalog"),
        ("POST", "/v1/shiju/apps"),
        ("POST", "/v1/shiju/apps/api-keys"),
        ("POST", "/v1/shiju/apps/01K1APP0000000000000000001/api-key-binding"),
        ("DELETE", "/v1/shiju/apps/01K1APP0000000000000000001/api-key-binding"),
        ("GET", "/v1/shiju/apps/01K1APP0000000000000000001/launch"),
        ("POST", "/v1/shiju/skills/installed"),
        ("POST", "/v1/shiju/skills/42/versions"),
    ] {
        let request = Request::builder().method(method).uri(path).body(Body::empty()).unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED, "{method} {path}");
    }
}

#[test]
fn gateway_route_summary_hides_disabled_optional_routes() {
    let lines = gateway_route_summary(GatewayRouteSummaryOptions {
        whatsapp_enabled: false,
        linq_enabled: false,
        wati_enabled: false,
        nextcloud_talk_enabled: false,
        qq_webhook_enabled: false,
        node_control_enabled: false,
    });
    let text = lines.join("\n");

    assert!(!text.contains("GET    /health"), "root health route should not be shown");

    for route in
        ["/v1/whatsapp", "/v1/linq", "/v1/wati", "/v1/nextcloud-talk", "/v1/qq", "/v1/node-control"]
    {
        assert!(!text.contains(route), "disabled route should not be shown: {route}");
    }
}

#[tokio::test]
async fn unavailable_default_provider_keeps_health_route_available_and_rejects_inference() {
    let config =
        Config { default_provider: Some("unknown-gateway-provider".into()), ..Config::default() };
    let mut state = state();
    state.provider = gateway_provider(&config);
    let error = state.provider.simple_chat("hello", "model", 0.0).await.unwrap_err();
    assert!(error.to_string().contains("Default provider is unavailable"));
    let response = external_router()
        .with_state(state)
        .oneshot(Request::get("/v1/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn supported_default_provider_is_initialized_normally() {
    let config = Config { default_provider: Some("openai".into()), ..Config::default() };
    let provider = gateway_provider(&config);
    assert!(provider.supports_streaming());
}

#[tokio::test]
async fn dynamic_tools_gateway_bootstrap_mounts_authenticated_host_routes() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _config = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let directory = tempfile::tempdir().unwrap();
    let scope = crate::session::runtime::SessionScope::new(
        vw_api_types::id::TenantScope::new("gateway-host-bootstrap", "validation").unwrap(),
        directory.path(),
    )
    .unwrap();
    let session =
        crate::session::runtime::create_session(crate::session::runtime::CreateSessionInput {
            scope: scope.clone(),
            parent_id: None,
            title: None,
            permission: None,
        })
        .await
        .unwrap();
    let mut state = state();
    state.pairing = Arc::new(PairingGuard::new(true, &["dynamic-host-test-skey".into()]));
    let app = client_router(state.clone(), Router::new(), Router::new()).with_state(state.clone());
    let request = |suffix: &str, token: Option<&str>| {
        let mut request = Request::builder()
            .method(if suffix == "/hosts" { "POST" } else { "GET" })
            .uri(format!("/v1/session/{}/dynamic-tools{suffix}", session.info.id))
            .header("Authorization", "Bearer dynamic-host-test-skey")
            .header("X-Platform-ID", "gateway-host-bootstrap")
            .header("X-Company-ID", "validation")
            .header("x-vibewindow-directory", directory.path().to_str().unwrap());
        if let Some(token) = token {
            request = request.header("x-vibewindow-tool-host", token);
        }
        request.body(Body::empty()).unwrap()
    };
    let response = app.clone().oneshot(request("/hosts", None)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let binding: vw_api_types::tools::DynamicToolHostBinding =
        serde_json::from_slice(&axum::body::to_bytes(response.into_body(), 4096).await.unwrap())
            .unwrap();
    let stream = app.oneshot(request("/calls", Some(&binding.binding_token))).await.unwrap();
    assert_eq!(stream.status(), StatusCode::OK);
    drop(stream);
    let mut protected = state;
    protected.pairing = Arc::new(PairingGuard::new(true, &[]));
    let app = client_router(protected.clone(), Router::new(), Router::new()).with_state(protected);
    let mut unauthenticated = request("/hosts", None);
    unauthenticated.headers_mut().remove("Authorization");
    assert_eq!(app.oneshot(unauthenticated).await.unwrap().status(), StatusCode::UNAUTHORIZED);
}

// Reuse the production authenticated route assembly for cross-module acceptance.
pub(crate) fn dynamic_tools_acceptance_router() -> Router {
    let mut state = state();
    state.pairing = Arc::new(PairingGuard::new(true, &["dynamic-acceptance-skey".into()]));
    client_router(state.clone(), Router::new(), Router::new()).with_state(state)
}

#[tokio::test]
async fn cc_apps_production_gateway_routes_preserve_authorization() {
    let initial = state();
    let app = client_router(initial.clone(), Router::new(), Router::new()).with_state(initial);
    for resource in ["catalog", "installations", "launch-identities"] {
        let uri = format!("/v1/agent-apps/{resource}?targetHostId=task46&appId=claude");
        let request = Request::builder()
            .uri(&uri)
            .header("X-Platform-ID", "task46")
            .header("X-Company-ID", "task46")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::UNAUTHORIZED,
            "{resource}: application token still required"
        );
        let request = Request::builder().uri(&uri).body(Body::empty()).unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::BAD_REQUEST,
            "{resource}: tenant required"
        );
    }
    let mut protected = state();
    protected.pairing = Arc::new(PairingGuard::new(true, &["task46-test-key".into()]));
    let app = client_router(protected.clone(), Router::new(), Router::new()).with_state(protected);
    let request = Request::builder()
        .uri("/v1/agent-apps/catalog?targetHostId=task46")
        .header("X-Platform-ID", "task46")
        .header("X-Company-ID", "task46")
        .body(Body::empty())
        .unwrap();
    assert_eq!(app.oneshot(request).await.unwrap().status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn cc_apps_omp_provider_stateful_gateway_persists_and_isolates_tenant_records() {
    use axum::{Extension, middleware};
    let root = tempfile::tempdir().unwrap();
    let app = Router::new()
        .nest("/v1", stateful_handler_router())
        .with_state(state())
        .layer(Extension(crate::global_database::GlobalDatabase::for_home(root.path()).unwrap()))
        .layer(middleware::from_fn(crate::gateway::middleware::tenant_scope_middleware));
    let profile = serde_json::json!({"name":"fixture","protocol":"openai_responses","apiKey":"fixture-provider-key","baseUrl":"https://provider.example.test/v1","model":"fixture-model"});
    let request = |method: &str, company: &str, body: String| {
        Request::builder()
            .method(method)
            .uri("/v1/app-providers/omp/selected")
            .header("x-platform-id", "p")
            .header("x-company-id", company)
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap()
    };
    let response = app.clone().oneshot(request("PUT", "a", profile.to_string())).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "stateful CLI gateway must register provider writes"
    );
    let bytes = axum::body::to_bytes(response.into_body(), 65536).await.unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("fixture-provider-key"));
    assert_eq!(
        app.clone().oneshot(request("GET", "a", String::new())).await.unwrap().status(),
        StatusCode::OK
    );
    assert_eq!(
        app.oneshot(request("GET", "foreign", String::new())).await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
}
