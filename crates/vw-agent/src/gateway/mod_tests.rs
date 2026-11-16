// 验证网关顶层鉴权、超时选择与受保护路由的组合行为。

// 测试将启动期和常规 REST 路径分开，确认二者应用不同的认证例外。
use super::*;

use crate::app::agent::config::Config;
use crate::app::agent::memory::NoneMemory;
use model_types::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::Router;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;
use tower::util::ServiceExt;
use {
    vw_api_transport::id::COMPANY_ID_HEADER,
    vw_api_transport::id::PLATFORM_ID_HEADER,
};

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

// 认证状态显式控制配对开关与令牌集合，避免从用户环境读取隐式配置。
fn auth_state(require_pairing: bool, paired_tokens: &[String]) -> AppState {
    AppState {
        config: Arc::new(Mutex::new(Config::default())),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(require_pairing, paired_tokens)),
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

// 启动期应用单独组装，用来证明引导端点的例外不会扩散到常规路由。
fn bootstrap_rest_app(state: AppState) -> Router {
    Router::<AppState>::new()
        .merge(bootstrap::external_router())
        .merge(bootstrap::client_router(state.clone(), Router::new(), Router::new()))
        .with_state(state)
}

fn tenant_request(path: &str) -> Request<Body> {
    Request::get(path)
        .header(PLATFORM_ID_HEADER, "platform-test")
        .header(COMPANY_ID_HEADER, "company-test")
        .body(Body::empty())
        .expect("request should build")
}

#[tokio::test]
async fn run_gateway_rejects_public_bind_without_tunnel_or_opt_in() {
    let mut config = Config::default();
    config.tunnel.provider = "none".to_string();
    config.gateway.allow_public_bind = false;

    let error = run_gateway("0.0.0.0", 0, config).await.unwrap_err();

    assert!(error.to_string().contains("Refusing to bind to 0.0.0.0"));
}

#[tokio::test]
async fn bootstrap_router_registers_v1_agent_scheduler_project_routes() {
    let state = auth_state(false, &[]);
    let app = Router::new().nest("/v1", bootstrap::stateful_handler_router()).with_state(state);

    for path in [
        "/v1/agent-scheduler/projects/project-gateway/commands",
        "/v1/agent-scheduler/projects/project-gateway/wake",
    ] {
        let response = app.clone().oneshot(tenant_request(path)).await.unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED, "{path}");
    }
}

#[tokio::test]
async fn bootstrap_router_rejects_client_routes_without_tenant_headers() {
    let app = bootstrap_rest_app(auth_state(false, &[]));

    let response =
        app.oneshot(Request::get("/v1/global/health").body(Body::empty()).unwrap()).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn bootstrap_hooks_routes_remain_behind_gateway_authentication() {
    let token = String::from("paired-token");
    let app = bootstrap_rest_app(auth_state(true, &[token]));

    for path in ["/v1/hooks", "/v1/hooks/audit"] {
        let response = app.clone().oneshot(tenant_request(path)).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
    }
}

#[tokio::test]
async fn bootstrap_router_keeps_v1_health_and_webhook_outside_tenant_middleware() {
    let app = bootstrap_rest_app(auth_state(false, &[]));

    let health_response =
        app.clone().oneshot(Request::get("/v1/health").body(Body::empty()).unwrap()).await.unwrap();
    let webhook_response =
        app.oneshot(Request::get("/v1/webhook").body(Body::empty()).unwrap()).await.unwrap();

    assert_eq!(health_response.status(), StatusCode::OK);
    assert_eq!(webhook_response.status(), StatusCode::METHOD_NOT_ALLOWED);
}
