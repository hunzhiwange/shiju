//! 会话 Gateway 路由、租户作用域与持久化行为测试。
//!
//! 这些测试覆盖路由方法契约、请求级租户隔离，以及流式聊天落盘时的模型标识
//! 拆分、用户/助手消息生成和预分配消息 id 复用。

use super::stream::{StreamTurnMessageIds, persist_stream_chat_turn, split_stream_model_ref};
use super::{router, session_tenant_scope};

use crate::app::agent::config::Config;
use crate::app::agent::gateway::{AppState, GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::project;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use crate::app::agent::session as agent_session;
use crate::session::ui_types as ui_models;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::Extension;
use axum::http::{Method, Request, StatusCode};
use axum::middleware::from_fn;
use axum::routing::get;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tower::ServiceExt;
use vw_api_types::id::TenantScope;

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
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: None,
    }
}

fn test_app() -> Router {
    router().layer(Extension(TenantScope::default())).with_state(state())
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

async fn tenant_scope_probe() -> String {
    let tenant = crate::session::ui_store::current_tenant_scope();
    format!("{}/{}", tenant.platform_id().as_ref(), tenant.company_id().as_ref())
}

fn tenant_scope_probe_app() -> Router {
    Router::new().route("/scope", get(tenant_scope_probe)).layer(from_fn(session_tenant_scope))
}

async fn scope_response(tenant: Option<TenantScope>) -> axum::response::Response {
    let mut request = Request::builder().uri("/scope").body(Body::empty()).expect("request");
    if let Some(tenant) = tenant {
        request.extensions_mut().insert(tenant);
    }
    tenant_scope_probe_app().oneshot(request).await.expect("scope response")
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
async fn session_tenant_scope_forwards_each_request_scope() {
    let first = TenantScope::new("platform-a", "company-a").expect("first tenant");
    let second = TenantScope::new("platform-b", "company-b").expect("second tenant");

    let (first_response, second_response) =
        tokio::join!(scope_response(Some(first)), scope_response(Some(second)));
    assert_eq!(first_response.status(), StatusCode::OK);
    assert_eq!(second_response.status(), StatusCode::OK);

    let first_body = to_bytes(first_response.into_body(), usize::MAX).await.expect("first body");
    let second_body = to_bytes(second_response.into_body(), usize::MAX).await.expect("second body");
    assert_eq!(first_body.as_ref(), b"platform-a/company-a");
    assert_eq!(second_body.as_ref(), b"platform-b/company-b");
}

#[tokio::test]
async fn session_tenant_scope_rejects_requests_without_extension() {
    let response = scope_response(None).await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn split_stream_model_ref_uses_model_id_when_provider_is_missing() {
    let model_ref = split_stream_model_ref(Some("claude-sonnet-4"));

    assert_eq!(model_ref.provider_id, "");
    assert_eq!(model_ref.model_id, "claude-sonnet-4");
}

#[tokio::test]
async fn persist_stream_chat_turn_creates_user_and_assistant_messages() {
    let temp = tempfile::tempdir().expect("tempdir");
    let session_id = format!("ses_stream_test_{}", uuid::Uuid::new_v4());

    // 流式持久化依赖当前项目实例目录；测试用临时目录隔离真实用户数据。
    let (assistant_id, user_id, stored) =
        project::instance::provide(temp.path(), None, move || {
            Box::pin(async move {
                let usage = ui_models::TokenUsage {
                    input_tokens: 12,
                    output_tokens: 34,
                    cached_tokens: 5,
                    reasoning_tokens: 6,
                };
                let (assistant_id, user_id) = persist_stream_chat_turn(
                    &session_id,
                    "hello from user",
                    "hello from assistant",
                    Some("anthropic/claude-sonnet-4"),
                    &usage,
                    Some("stop"),
                    None,
                    &[],
                )
                .await
                .expect("stream turn should persist");
                let stored = agent_session::message::messages(&session_id, None)
                    .await
                    .expect("stored messages should load");
                Ok::<_, project::Error>((assistant_id, user_id, stored))
            })
        })
        .await
        .expect("instance context should be provided")
        .expect("stream turn should return stored messages");

    assert_eq!(stored.len(), 2);

    // 用户消息也携带模型引用，后续 UI 和审计视图需要用它重建上下文。
    let user_message = stored
        .iter()
        .find(|message| message.info.id() == user_id)
        .expect("user message should be present");
    match &user_message.info {
        agent_session::message::Info::User(info) => {
            assert_eq!(info.model.provider_id, "anthropic");
            assert_eq!(info.model.model_id, "claude-sonnet-4");
        }
        _ => panic!("expected stored user message"),
    }
    let user_text = user_message
        .parts
        .iter()
        .find_map(|part| match part {
            agent_session::message::Part::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .expect("user text part should be present");
    assert_eq!(user_text, "hello from user");

    // 助手消息必须挂到用户消息下面，避免会话树在回放时丢失父子关系。
    let assistant_message = stored
        .iter()
        .find(|message| message.info.id() == assistant_id)
        .expect("assistant message should be present");
    match &assistant_message.info {
        agent_session::message::Info::Assistant(info) => {
            assert_eq!(info.parent_id, user_id);
            assert_eq!(info.provider_id, "anthropic");
            assert_eq!(info.model_id, "claude-sonnet-4");
            assert_eq!(info.finish.as_deref(), Some("stop"));
            assert_eq!(info.tokens.input, 12);
            assert_eq!(info.tokens.output, 34);
            assert_eq!(info.tokens.reasoning, 6);
            assert_eq!(info.tokens.cache.read, 5);
        }
        _ => panic!("expected stored assistant message"),
    }
    let assistant_text = assistant_message
        .parts
        .iter()
        .find_map(|part| match part {
            agent_session::message::Part::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .expect("assistant text part should be present");
    assert_eq!(assistant_text, "hello from assistant");
}

#[tokio::test]
async fn persist_stream_chat_turn_reuses_preallocated_message_ids() {
    let temp = tempfile::tempdir().expect("tempdir");
    let session_id = "ses_stream_preallocated";
    // 预分配 id 来自流式响应开始阶段，落盘必须复用同一组 id 才能和前端事件对齐。
    let preallocated =
        StreamTurnMessageIds::new("msg_assistant_preallocated", "msg_user_preallocated");

    let (assistant_id, user_id, stored) =
        project::instance::provide(temp.path(), None, move || {
            let preallocated = preallocated.clone();
            Box::pin(async move {
                let usage = ui_models::TokenUsage::default();
                let (assistant_id, user_id) = persist_stream_chat_turn(
                    session_id,
                    "hello from user",
                    "hello from assistant",
                    Some("anthropic/claude-sonnet-4"),
                    &usage,
                    Some("stop"),
                    Some(&preallocated),
                    &[],
                )
                .await
                .expect("stream turn should persist");
                let stored = agent_session::message::messages(session_id, None)
                    .await
                    .expect("stored messages should load");
                Ok::<_, project::Error>((assistant_id, user_id, stored))
            })
        })
        .await
        .expect("instance context should be provided")
        .expect("stream turn should return stored messages");

    assert_eq!(assistant_id, "msg_assistant_preallocated");
    assert_eq!(user_id, "msg_user_preallocated");
    assert!(stored.iter().any(|message| message.info.id() == assistant_id));
    assert!(stored.iter().any(|message| message.info.id() == user_id));
}

#[tokio::test]
async fn persist_stream_chat_turn_stores_extra_assistant_patch_parts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let session_id = "ses_stream_patch_part";
    let preallocated = StreamTurnMessageIds::new("msg_assistant_patch", "msg_user_patch");
    let patch_part = agent_session::message::Part::Patch(agent_session::message::PatchPart {
        base: agent_session::message::PartBase {
            id: "prt_patch".to_string(),
            session_id: session_id.to_string(),
            message_id: "msg_assistant_patch".to_string(),
        },
        hash: "tree_hash_before_tools".to_string(),
        files: vec!["/tmp/project/src/main.rs".to_string()],
        additions: Some(3),
        deletions: Some(1),
    });

    let stored = project::instance::provide(temp.path(), None, move || {
        let preallocated = preallocated.clone();
        let patch_part = patch_part.clone();
        Box::pin(async move {
            let usage = ui_models::TokenUsage::default();
            persist_stream_chat_turn(
                session_id,
                "change file",
                "changed",
                None,
                &usage,
                Some("stop"),
                Some(&preallocated),
                &[patch_part],
            )
            .await
            .expect("stream turn should persist");
            agent_session::message::messages(session_id, None).await.map_err(|err| err.to_string())
        })
    })
    .await
    .expect("instance context should be provided")
    .expect("stored messages should load");

    let assistant = stored
        .iter()
        .find(|message| message.info.id() == "msg_assistant_patch")
        .expect("assistant message should be present");
    assert!(assistant.parts.iter().any(|part| {
        matches!(
            part,
            agent_session::message::Part::Patch(p)
                if p.hash == "tree_hash_before_tools"
                    && p.files == vec!["/tmp/project/src/main.rs".to_string()]
                    && p.additions == Some(3)
                    && p.deletions == Some(1)
        )
    }));
}
