//! Cross-module acceptance with real persistence and HTTP protocol boundaries.
use super::*;
use crate::gateway::{AppState, GatewayRateLimiter, IdempotencyStore};
use crate::memory::NoneMemory;
use crate::providers::Provider;
use crate::security::pairing::PairingGuard;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::Request,
};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tower::ServiceExt;
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

fn state_with_config(config: Arc<parking_lot::Mutex<Config>>, require_pairing: bool) -> AppState {
    let (event_tx, _) = broadcast::channel(16);
    AppState {
        config,
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        pairing: Arc::new(PairingGuard::new(require_pairing, &[])),
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

async fn request(router: &Router, method: &str, path: &str, body: Value, company: &str) -> Value {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .header("X-Platform-ID", "wecom-acceptance")
                .header("X-Company-ID", company)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    assert!(status.is_success(), "{status}: {body}");
    assert!(!body.to_string().contains("acceptance-secret"));
    body
}

#[tokio::test]
async fn wecom_delivery_acceptance_saved_api_jobs_reload_and_receipts() {
    let company = uuid::Uuid::new_v4().to_string();
    let tenant = vw_api_types::id::TenantScope::new("wecom-acceptance", &company).unwrap();
    crate::session::ui_store::with_tenant_scope(tenant.clone(), async {
        let tmp = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.workspace_dir = tmp.path().join("workspace");
        config.config_path = tmp.path().join("config.json");
        std::fs::create_dir_all(&config.workspace_dir).unwrap();
        crate::config::schema::save_config_for_tenant(&config, &tenant).await.unwrap();
        let shared = Arc::new(parking_lot::Mutex::new(config.clone()));
        let state = state_with_config(shared, false);
        let router = crate::gateway::wecom_acceptance_router(state.clone(), Router::new(), Router::new()).with_state(state.clone());
        request(&router, "PATCH", "/v1/config", json!({"channels_config":{
            "wecom":{"bot_id":"acceptance-bot","secret":"acceptance-secret"},
            "wecom_webhook":{"webhook_url":"https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=acceptance-secret"}
        }}), &company).await;
        request(&router, "GET", "/v1/config", Value::Null, &company).await;
        let config = state.config.lock().clone();
        let mut jobs = Vec::new();
        for (channel, target) in [("wecom", "explicit-user"), ("wecom_webhook", "default")] {
            let added = request(&router, "POST", "/v1/cron", json!({
                "schedule_kind":"every","every_ms":60000,"command":"echo 完成",
                "delivery_mode":"announce","delivery_channel":channel,"delivery_to":target,
                "delivery_best_effort":false
            }), &company).await;
            let id = added["job"]["id"].as_str().unwrap();
            jobs.push(get_job(&config, id).unwrap());
        }
        assert_eq!(request(&router, "GET", "/v1/cron", Value::Null, &company).await["jobs"].as_array().unwrap().len(), 2);
        // No socket owner: the saved smart-bot task must fail, never claim delivery.
        let (ok, output) = delivery::apply_result(&config, &jobs[0], true, "完成").await;
        assert!(!ok);
        assert!(output.contains("连接"));
        assert!(!output.contains("acceptance-secret"));

        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = count.clone();
        let app = Router::new().route("/", axum::routing::post(move |axum::Json(body):axum::Json<Value>| {
            let seen = seen.clone();
            async move {
                seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                assert_eq!(body["msgtype"], "markdown");
                assert!(body["markdown"]["content"].as_str().unwrap().len() <= 3000);
                axum::Json(json!({"errcode":0}))
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/", listener.local_addr().unwrap()).parse().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
        delivery::TEST_WEBHOOK_ENDPOINT.scope(endpoint, async {
            let long = "中文结果
".repeat(1500);
            let (ok, output) = delivery::apply_result(&config, &jobs[1], true, &long).await;
            assert!(ok);
            assert_eq!(output, long);
            let (ok, output) = scheduler::execute_job_now(&config, &jobs[1]).await;
            assert!(ok, "{output}");
            assert!(output.contains("完成"));
            assert!(count.load(std::sync::atomic::Ordering::SeqCst) > 1);
            let before = count.load(std::sync::atomic::Ordering::SeqCst);
            let mut disabled = jobs[1].clone();
            disabled.delivery.mode = "none".into();
            assert!(delivery::apply_result(&config, &disabled, true, "完成").await.0);
            assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), before);
            // Persisted removal must override the launch snapshot immediately.
            request(&router, "PATCH", "/v1/config", json!({"channels_config":{"wecom_webhook":null}}), &company).await;
            for best_effort in [false,true] {
                let mut job = jobs[1].clone();
                job.delivery.best_effort = best_effort;
                let (ok, output) = delivery::apply_result(&config, &job, true, "完成").await;
                assert_eq!(ok, best_effort);
                assert!(output.contains("未配置"), "{output}");
                assert!(!output.contains("acceptance-secret"));
            }
            assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), before);
        }).await;
        server.abort();
        let _ = server.await;
    }).await;
}
