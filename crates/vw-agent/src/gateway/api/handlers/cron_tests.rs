// 覆盖定时任务 API 的调度解析、认证及任务生命周期操作。

use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::api::types::{CronAddBody, CronUpdateBody};
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use model_types::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::body::to_bytes;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

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

fn temp_config(workspace: &std::path::Path) -> Arc<parking_lot::Mutex<Config>> {
    let mut config = Config::default();
    config.workspace_dir = workspace.to_path_buf();
    Arc::new(parking_lot::Mutex::new(config))
}

async fn response_json<R: IntoResponse>(response: R) -> (StatusCode, serde_json::Value) {
    let response = response.into_response();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&body).unwrap_or_else(|_| serde_json::json!({}));
    (status, value)
}

fn add_body() -> CronAddBody {
    CronAddBody {
        name: Some(" Nightly ".to_string()),
        job_type: None,
        schedule_kind: None,
        schedule: Some("0 0 * * * *".to_string()),
        at: None,
        every_ms: None,
        command: Some(" echo ok ".to_string()),
        prompt: None,
        session_target: None,
        model: None,
        agent: None,
        acp_agent: None,
        project_path: None,
        wake: None,
        fallbacks: None,
        full_access: None,
        task_pool: None,
        delivery_mode: None,
        delivery_channel: None,
        delivery_to: None,
        delivery_best_effort: None,
        delete_after_run: None,
        ..Default::default()
    }
}

fn update_body() -> CronUpdateBody {
    CronUpdateBody {
        name: None,
        job_type: None,
        schedule_kind: None,
        schedule: None,
        at: None,
        every_ms: None,
        command: None,
        prompt: None,
        session_target: None,
        model: None,
        agent: None,
        acp_agent: None,
        project_path: None,
        wake: None,
        fallbacks: None,
        full_access: None,
        task_pool: None,
        delivery_mode: None,
        delivery_channel: None,
        delivery_to: None,
        delivery_best_effort: None,
        delete_after_run: None,
        enabled: None,
        ..Default::default()
    }
}

async fn history_request(state: AppState) -> (StatusCode, serde_json::Value) {
    use tower::ServiceExt;
    let router = crate::app::agent::gateway::bootstrap::client_router(
        state.clone(),
        axum::Router::new(),
        axum::Router::new(),
    )
    .with_state(state);
    response_json(
        router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/v1/cron/history")
                    .header(vw_api_transport::PLATFORM_ID_HEADER, "test-platform")
                    .header(vw_api_transport::COMPANY_ID_HEADER, "test-company")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap(),
    )
    .await
}


#[tokio::test]
async fn cron_handlers_reject_requests_when_pairing_is_required() {
    let dir = tempfile::tempdir().unwrap();
    let config = temp_config(dir.path());
    let headers = HeaderMap::new();

    let (status, body) = response_json(
        handle_api_cron_list(State(state_with_config(config.clone(), true)), headers.clone()).await,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"].as_str().unwrap().contains("Unauthorized"));

    let (status, _) = response_json(
        handle_api_cron_runs(
            State(state_with_config(config.clone(), true)),
            headers.clone(),
            Path("missing".to_string()),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _) = response_json(
        handle_api_cron_delete(
            State(state_with_config(config, true)),
            headers,
            Path("missing".to_string()),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn cron_mutating_handlers_reject_requests_before_writing_when_pairing_is_required() {
    let dir = tempfile::tempdir().unwrap();
    let config = temp_config(dir.path());
    let headers = HeaderMap::new();

    let (status, body) = response_json(
        handle_api_cron_add(
            State(state_with_config(config.clone(), true)),
            headers.clone(),
            JsonResponse(add_body()),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"].as_str().unwrap().contains("Unauthorized"));

    let (status, body) = response_json(
        handle_api_cron_update(
            State(state_with_config(config, true)),
            headers,
            Path("missing".to_string()),
            JsonResponse(update_body()),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body["error"].as_str().unwrap().contains("Unauthorized"));
}

#[tokio::test]
async fn cron_history_endpoint_requires_auth() {
    let tmp = tempfile::tempdir().unwrap();
    let config = temp_config(tmp.path());
    let (status, _) = history_request(state_with_config(config, true)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn cron_wecom_delivery_api_roundtrip_and_partial_edit_validate_merged_target() {
    let tenant =
        vw_api_transport::TenantScope::new("cron-api-test", uuid::Uuid::new_v4().to_string())
            .unwrap();
    crate::session::ui_store::with_tenant_scope(tenant.clone(), async {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = temp_config(tmp.path());
        {
            let mut c = config.lock();
            c.config_path = tmp.path().join("config.json");
            c.channels_config.wecom = Some(serde_json::from_value(serde_json::json!({"bot_id":"bot","secret":"fixture-secret"})).unwrap());
            c.channels_config.wecom_webhook = Some(serde_json::from_value(serde_json::json!({"webhook_url":"https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=fixture-secret"})).unwrap());
        }
        let saved = config.lock().clone();
        crate::config::schema::save_config_for_tenant(&saved, &tenant).await.unwrap();
        let state = state_with_config(config.clone(), false);
        for channel in ["wecom", "wecom_webhook"] {
            let mut body = add_body();
            body.delivery_mode = Some("announce".into());
            body.delivery_channel = Some(channel.into());
            let target = if channel == "wecom" { "user-id" } else { "default" };
            body.delivery_to = Some(target.into());
            body.delivery_best_effort = Some(false);
            let (status, added) = response_json(handle_api_cron_add(State(state.clone()), HeaderMap::new(), JsonResponse(body)).await).await;
            assert_eq!(status, StatusCode::OK, "{added}");
            assert!(!added.to_string().contains("fixture-secret"));
            let id = added["job"]["id"].as_str().unwrap().to_string();
            let patch = CronUpdateBody { delivery_channel: Some(format!(" {channel} ")), delivery_best_effort:Some(true), ..Default::default() };
            let (status, updated) = response_json(handle_api_cron_update(State(state.clone()),HeaderMap::new(),Path(id.clone()),JsonResponse(patch)).await).await;
            assert_eq!(status, StatusCode::OK, "{updated}");
            assert_eq!(updated["job"]["delivery_mode"], "announce");
            assert_eq!(updated["job"]["delivery_channel"], channel);
            assert_eq!(updated["job"]["delivery_to"], target);
            assert_eq!(updated["job"]["delivery_best_effort"], true);
            let patch = CronUpdateBody { delivery_to:Some(" ".into()), ..Default::default() };
            let (status, _) = response_json(handle_api_cron_update(State(state.clone()),HeaderMap::new(),Path(id.clone()),JsonResponse(patch)).await).await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert_eq!(crate::cron::get_job(&config.lock(), &id).unwrap().delivery.to.as_deref(), Some(target));
        }
        let (_, listed) = response_json(handle_api_cron_list(State(state), HeaderMap::new()).await).await;
        assert_eq!(listed["jobs"].as_array().unwrap().len(), 2);
        assert!(!listed.to_string().contains("fixture-secret"));
    }).await;
}

#[tokio::test]
async fn wb_schedule_api_preview_routes_enforce_auth_and_return_candidates() {
    use axum::{Router, body::Body, http::Request};
    use tower::ServiceExt;
    for pairing in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let config = temp_config(dir.path());
        let state = state_with_config(config.clone(), pairing);
        let router =
            crate::gateway::bootstrap::client_router(state.clone(), Router::new(), Router::new())
                .with_state(state);
        for path in ["/api/cron/preview", "/v1/cron/preview"] {
            let request = Request::post(path)
                .header("X-Platform-ID", "cron-preview-test")
                .header("X-Company-ID", "cron-preview-test")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"schedule_kind":"every","every_ms":1000}"#))
                .unwrap();
            let (status, result) =
                response_json(router.clone().oneshot(request).await.unwrap()).await;
            assert_eq!(
                status,
                if pairing { StatusCode::UNAUTHORIZED } else { StatusCode::OK },
                "{result}"
            );
            if !pairing {
                assert_eq!(result["candidates"].as_array().unwrap().len(), 5);
            }
        }
        assert!(crate::cron::list_jobs(&config.lock()).unwrap().is_empty());
    }
}
