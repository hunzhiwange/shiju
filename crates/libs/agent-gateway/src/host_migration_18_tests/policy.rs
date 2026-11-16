use super::*;
use axum::{Router, body::Body, http::{Request, StatusCode, HeaderValue, header}, middleware::{from_fn, from_fn_with_state}, routing::{any, get}};
use tower::ServiceExt;
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
        observer: Arc::new(observer_backends::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: agent_config::runtime::MultimodalConfig::default(),
        max_iterations: 10,
        event_tx: tokio::sync::broadcast::channel(16).0,
        session_query_engines: Default::default(),
        hook_runtime: None,
        channels: Default::default(),
    }
}

fn auth_app(state: AppState) -> Router {
    Router::new()
        .route("/protected", any(|| async { StatusCode::CREATED }))
        .route("/v1/global/health", any(|| async { StatusCode::NO_CONTENT }))
        .route("/global/health", any(|| async { StatusCode::NO_CONTENT }))
        .route("/v1/global/health/", any(|| async { StatusCode::CREATED }))
        .route("/global/healthcheck", any(|| async { StatusCode::CREATED }))
        .with_state(state.clone())
        .layer(from_fn_with_state(state, skey_auth_middleware))
}

fn bearer_request(path: &str, token: &str) -> Request<Body> {
    let value = format!("Bearer {token}");
    Request::get(path)
        .header(header::AUTHORIZATION, HeaderValue::from_str(&value).unwrap())
        .body(Body::empty())
        .unwrap()
}

#[test]
fn workflow_chat_messages_path_matches_collection_and_application_routes() {
    assert!(is_workflow_chat_messages_path("/v1/workflow/applications/chat-messages"));
    assert!(is_workflow_chat_messages_path("/v1/workflow/applications/demo/chat-messages"));
    assert!(is_workflow_chat_messages_path(
        "/v1/workflow/applications/demo/sub-path/chat-messages"
    ));
}

#[test]
fn workflow_chat_messages_path_rejects_empty_or_partial_matches() {
    assert!(!is_workflow_chat_messages_path("/v1/workflow/applications//chat-messages"));
    assert!(!is_workflow_chat_messages_path("/v1/workflow/applications/   /chat-messages"));
    assert!(!is_workflow_chat_messages_path("/v1/workflow/applications/demo/chat-messages/extra"));
    assert!(!is_workflow_chat_messages_path("/v1/workflow/application/demo/chat-messages"));
}

#[test]
fn timeout_for_path_uses_extended_window_for_long_running_sse() {
    assert_eq!(
        request_timeout_for_path("/v1/workflow/applications/chat-messages"),
        Duration::from_secs(WORKFLOW_CHAT_MESSAGES_TIMEOUT_SECS)
    );
    assert_eq!(
        request_timeout_for_path("/v1/workflow/applications/demo/chat-messages"),
        Duration::from_secs(WORKFLOW_CHAT_MESSAGES_TIMEOUT_SECS)
    );
    assert_eq!(
        request_timeout_for_path("/v1/chat/stream"),
        Duration::from_secs(WORKFLOW_CHAT_MESSAGES_TIMEOUT_SECS)
    );
    assert_eq!(
        request_timeout_for_path("/v1/task-plan-run/stream"),
        Duration::from_secs(WORKFLOW_CHAT_MESSAGES_TIMEOUT_SECS)
    );
    assert_eq!(
        request_timeout_for_path("/v1/workflow/applications/demo/messages"),
        Duration::from_secs(REQUEST_TIMEOUT_SECS)
    );
}

#[tokio::test]
async fn request_timeout_middleware_returns_inner_response_before_deadline() {
    let app = Router::new()
        .route("/ok", get(|| async { StatusCode::CREATED }))
        .layer(from_fn(request_timeout_middleware));

    let response = app.oneshot(Request::get("/ok").body(Body::empty()).unwrap()).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test(start_paused = true)]
async fn request_timeout_middleware_returns_timeout_after_deadline() {
    let app = Router::new()
        .route(
            "/slow",
            get(|| async {
                tokio::time::sleep(Duration::from_secs(REQUEST_TIMEOUT_SECS + 1)).await;
                StatusCode::OK
            }),
        )
        .layer(from_fn(request_timeout_middleware));

    let response = app.oneshot(Request::get("/slow").body(Body::empty()).unwrap()).await.unwrap();

    assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
}

#[tokio::test(start_paused = true)]
async fn request_timeout_middleware_preserves_workflow_response_past_default_deadline() {
    let app = Router::new()
        .route(
            "/v1/workflow/applications/demo/chat-messages",
            get(|| async {
                tokio::time::sleep(Duration::from_secs(REQUEST_TIMEOUT_SECS + 1)).await;
                StatusCode::ACCEPTED
            }),
        )
        .layer(from_fn(request_timeout_middleware));

    let response = app
        .oneshot(
            Request::get("/v1/workflow/applications/demo/chat-messages")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
}

#[tokio::test(start_paused = true)]
async fn request_timeout_middleware_times_out_workflow_after_extended_deadline() {
    let app = Router::new()
        .route(
            "/v1/workflow/applications/chat-messages",
            get(|| async {
                tokio::time::sleep(Duration::from_secs(WORKFLOW_CHAT_MESSAGES_TIMEOUT_SECS + 1))
                    .await;
                StatusCode::OK
            }),
        )
        .layer(from_fn(request_timeout_middleware));

    let response = app
        .oneshot(
            Request::get("/v1/workflow/applications/chat-messages").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
}

#[test]
fn public_skey_auth_path_matches_only_global_health_routes() {
    assert!(is_public_skey_auth_path("/v1/global/health"));
    assert!(is_public_skey_auth_path("/global/health"));
    assert!(!is_public_skey_auth_path("/v1/health"));
    assert!(!is_public_skey_auth_path("/v1/global/health/"));
    assert!(!is_public_skey_auth_path("/global/healthcheck"));
}

#[tokio::test]
async fn skey_auth_middleware_allows_options_and_public_paths_without_token() {
    let state = auth_state(true, &[String::from("paired-token")]);
    let app = auth_app(state);

    let response = app
        .clone()
        .oneshot(Request::options("/protected").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .clone()
        .oneshot(Request::get("/v1/global/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response =
        app.oneshot(Request::get("/global/health").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn skey_auth_middleware_rejects_public_path_lookalikes_without_token() {
    let app = auth_app(auth_state(true, &[String::from("paired-token")]));

    for path in ["/v1/global/health/", "/global/healthcheck"] {
        let response =
            app.clone().oneshot(Request::get(path).body(Body::empty()).unwrap()).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
    }
}

#[tokio::test]
async fn skey_auth_middleware_accepts_valid_skey() {
    let token = String::from("paired-token");
    let app = auth_app(auth_state(true, std::slice::from_ref(&token)));

    let response = app.oneshot(bearer_request("/protected", &token)).await.unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn skey_auth_middleware_rejects_missing_or_invalid_skey() {
    let token = String::from("paired-token");
    let app = auth_app(auth_state(true, &[token]));

    let response =
        app.clone().oneshot(Request::get("/protected").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = app.oneshot(bearer_request("/protected", "wrong-token")).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn acp_install_timeout_covers_installer_deadline_only_for_single_agent_routes() {
    assert_eq!(request_timeout_for_path("/v1/desktop/acp-install/droid"), Duration::from_secs(310));
    for path in [
        "/v1/desktop/acp-install/",
        "/v1/desktop/acp-install/droid/extra",
        "/v1/desktop/acp-diagnostic/droid",
    ] {
        assert_eq!(request_timeout_for_path(path), Duration::from_secs(REQUEST_TIMEOUT_SECS));
    }
}
