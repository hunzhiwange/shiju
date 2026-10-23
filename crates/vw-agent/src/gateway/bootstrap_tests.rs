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
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(false, &[])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        whatsapp: None,
        whatsapp_app_secret: None,
        linq: None,
        linq_signing_secret: None,
        nextcloud_talk: None,
        nextcloud_talk_webhook_secret: None,
        wati: None,
        qq: None,
        qq_webhook_enabled: false,
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
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
    assert_eq!(routes.len(), 234);
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
    let config = Config {
        default_provider: Some("unknown-gateway-provider".into()),
        ..Config::default()
    };
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
    let config =
        Config { default_provider: Some("openai".into()), ..Config::default() };
    let provider = gateway_provider(&config);
    assert!(provider.supports_streaming());
}
