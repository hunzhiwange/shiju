//! 专用测试进程：真实 Gateway/processor + 本地 OpenAI HTTP 故障边界。
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::Notify;

#[derive(Default)]
struct ModelState {
    calls: Mutex<HashMap<String, usize>>,
    release: Notify,
}

async fn model(
    State(state): State<Arc<ModelState>>,
    Json(body): Json<Value>,
) -> axum::response::Response {
    let query = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|message| message["role"] == "user")
        .unwrap()["content"]
        .as_str()
        .unwrap()
        .to_string();
    let scenario = ["transient", "exhausted", "cancel"]
        .into_iter()
        .find(|name| query.contains(name))
        .expect("known scenario");
    let call = {
        let mut calls = state.calls.lock().unwrap();
        let theme = if query.contains("-dark") { "dark" } else { "light" };
        let count = calls.entry(format!("{scenario}-{theme}")).or_default();
        *count += 1;
        *count
    };
    if scenario == "transient" && call > 1 {
        state.release.notified().await;
        let delta = json!({"id":"fixture","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":"瞬态故障后恢复成功"},"finish_reason":null}]});
        let end = json!({"id":"fixture","object":"chat.completion.chunk","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}});
        return (
            [("content-type", "text/event-stream")],
            format!("data: {delta}\n\ndata: {end}\n\ndata: [DONE]\n\n"),
        )
            .into_response();
    }
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({"error":{"message":"本地模型暂时不可用","type":"server_error"}})),
    )
        .into_response()
}

/// 只由显式集成测试命令选择；普通单测不启动常驻服务。
#[tokio::test(flavor = "multi_thread")]
async fn recovery_window_gateway_fixture() {
    let Some(root) = std::env::var_os("VW_MODEL_RECOVERY_TEST_ROOT").map(std::path::PathBuf::from)
    else {
        return;
    };
    assert!(root.is_absolute() && root.is_dir());
    let models = Arc::new(ModelState::default());
    let release = Arc::clone(&models);
    let stats = Arc::clone(&models);
    let router = Router::new()
        .route("/v1/chat/completions", post(model))
        .route(
            "/release",
            post(move || {
                let state = Arc::clone(&release);
                async move {
                    state.release.notify_one();
                    Json(json!({"ok":true}))
                }
            }),
        )
        .route(
            "/stats",
            get(move || {
                let state = Arc::clone(&stats);
                async move { Json(json!(state.calls.lock().unwrap().clone())) }
            }),
        )
        .with_state(models);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let model_port = listener.local_addr().unwrap().port();
    let model_server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let tenant = vw_api_types::id::TenantScope::new("recovery-window", "validation").unwrap();
    crate::config::update_global_for_tenant(&tenant, json!({
        "providers": {"recovery-window": {"api":format!("http://127.0.0.1:{model_port}/v1"), "env":["VW_RECOVERY_WINDOW_API_KEY"], "models":{"original":{"status":"active"}}}},
        "default_provider":"recovery-window", "default_model":"recovery-window/original",
        "reliability":{"provider_retries":2, "provider_backoff_ms":3000},
        "memory":{"backend":"none","auto_save":false}
    })).await.unwrap();
    let config = crate::config::get_global_for_tenant(&tenant).await.unwrap();
    let (event_tx, _) = tokio::sync::broadcast::channel(64);
    let state = super::AppState {
        config: Arc::new(parking_lot::Mutex::new(config)),
        provider: Arc::from(
            crate::providers::create_provider_with_url(
                "openai",
                Some("fixture-only"),
                Some(&format!("http://127.0.0.1:{model_port}/v1")),
            )
            .unwrap(),
        ),
        model: "original".into(),
        temperature: 0.0,
        mem: Arc::new(crate::memory::NoneMemory::new()),
        auto_save: false,
        pairing: Arc::new(crate::security::pairing::PairingGuard::new(
            true,
            &["window-fixture-token".into()],
        )),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(super::GatewayRateLimiter::new(1000, 1000, 1000)),
        idempotency_store: Arc::new(super::IdempotencyStore::new(
            std::time::Duration::from_secs(300),
            1000,
        )),
        channels: Default::default(),
        observer: Arc::new(crate::observability::NoopObserver),
        tools_registry: Arc::new(vec![]),
        tools_registry_exec: Arc::new(vec![]),
        multimodal: Default::default(),
        max_iterations: 3,
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: None,
    };
    let gateway_router =
        super::bootstrap::client_router(state.clone(), Router::new(), Router::new())
            .with_state(state);
    let gateway_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = gateway_listener.local_addr().unwrap();
    let gateway = tokio::spawn(async move {
        axum::serve(gateway_listener, gateway_router).await.unwrap();
    });
    std::fs::write(
        root.join("ready.json"),
        json!({"gateway_port":address.port(), "model_port":model_port}).to_string(),
    )
    .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(180), async {
        while !root.join("stop").exists() {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("window driver must stop fixture");
    gateway.abort();
    model_server.abort();
    let _ = gateway.await;
    let _ = model_server.await;
}
