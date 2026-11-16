//! 会话 Gateway 路由、租户作用域与持久化行为测试。
//!
//! 这些测试覆盖路由方法契约、请求级租户隔离，以及流式聊天落盘时的模型标识
//! 拆分、用户/助手消息生成和预分配消息 id 复用。

use super::router;

use crate::app::agent::config::Config;
use crate::app::agent::gateway::{AppState, GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use model_types::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::Router;
use axum::body::Body;
use axum::extract::Extension;
use axum::http::{Method, Request, StatusCode};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tower::ServiceExt;
use vw_api_transport::id::TenantScope;

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(message.to_string())
    }
}

fn state() -> AppState {
    let (event_tx, _) = broadcast::channel(16);
    AppState {
        config: Arc::new(parking_lot::Mutex::new(Config::default())),
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
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: Default::default(),
    }
}

pub(super) fn test_app() -> Router {
    test_app_for_tenant(TenantScope::default())
}

pub(super) fn test_app_for_tenant(tenant: TenantScope) -> Router {
    router().layer(Extension(tenant)).with_state(state())
}

async fn response_status(method: Method, uri: &str) -> StatusCode {
    test_app()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("route should respond")
        .status()
}







#[test]
fn session_router_can_be_constructed() {
    let _: Router<AppState> = router();
}

#[tokio::test]
async fn session_router_registers_expected_paths_and_methods() {
    let cases = [
        (Method::GET, "/chat/stream"),
        (Method::PUT, "/session"),
        (Method::POST, "/session/status"),
        (Method::PUT, "/session/session-a"),
        (Method::POST, "/session/session-a/children"),
        (Method::POST, "/session/session-a/todo"),
        (Method::GET, "/session/session-a/fork"),
        (Method::GET, "/session/session-a/reset"),
        (Method::POST, "/session/session-a/diff"),
        (Method::GET, "/session/session-a/summarize"),
        (Method::POST, "/session/session-a/message"),
        (Method::POST, "/session/session-a/message/message-a"),
        (Method::GET, "/session/session-a/message/message-a/part/part-a"),
        (Method::POST, "/session/ui-previews"),
        (Method::POST, "/session/archived"),
        (Method::POST, "/session/scope"),
        (Method::POST, "/session/session-a/ui"),
        (Method::POST, "/session/session-a/preview"),
        (Method::POST, "/session/session-a/path"),
        (Method::POST, "/session/session-a/any"),
        (Method::GET, "/session/session-a/title"),
    ];

    for (method, uri) in cases {
        assert_eq!(response_status(method, uri).await, StatusCode::METHOD_NOT_ALLOWED, "{uri}");
    }
}

#[tokio::test]
async fn session_router_rejects_unknown_paths() {
    assert_eq!(
        response_status(Method::GET, "/session/session-a/unknown").await,
        StatusCode::NOT_FOUND
    );
}





#[tokio::test]
async fn persist_stream_chat_turn_creates_user_and_assistant_messages() {
    gateway_session::stream_persistence_acceptance::persist_stream_chat_turn_creates_user_and_assistant_messages::<super::stream::Host>().await;
}

#[tokio::test]
async fn persist_stream_chat_turn_reuses_preallocated_message_ids() {
    gateway_session::stream_persistence_acceptance::persist_stream_chat_turn_reuses_preallocated_message_ids::<super::stream::Host>().await;
}

#[tokio::test]
async fn persist_stream_chat_turn_stores_extra_assistant_patch_parts() {
    gateway_session::stream_persistence_acceptance::persist_stream_chat_turn_stores_extra_assistant_patch_parts::<super::stream::Host>().await;
}
