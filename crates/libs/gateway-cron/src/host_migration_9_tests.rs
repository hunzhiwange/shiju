//! Cron HTTP policy and persistence tests owned by gateway-cron.
use crate::handlers::*;
use crate::requests::{CronAddBody, CronUpdateBody};
use crate::state::CronState as AppState;
use agent_config::runtime::Config;
use agent_cron::{CronJob, CronRun, DeliveryConfig, JobType, Schedule, SessionTarget};
use axum::{body::to_bytes, extract::{State, Path}, http::{HeaderMap, StatusCode}, response::{IntoResponse, Json as JsonResponse}};
use chrono::{TimeZone, Utc};
use std::sync::Arc;

struct NoDelivery;
#[async_trait::async_trait]
impl agent_cron::delivery::DeliveryHost for NoDelivery {
    fn validate_webhook(&self, _: &str) -> anyhow::Result<()> { anyhow::bail!("unexpected webhook validation") }
    async fn latest_config(&self, config: &Config) -> anyhow::Result<Config> { Ok(config.clone()) }
    async fn send_validated(&self, _: &Config, _: &str, _: &str, _: &str) -> anyhow::Result<()> { anyhow::bail!("unexpected delivery") }
}

fn state_with_config(config: Arc<parking_lot::Mutex<Config>>, require_pairing: bool) -> AppState {
    AppState {
        config,
        authenticate: Arc::new(move |_| {
            if require_pairing {
                Err((StatusCode::UNAUTHORIZED, JsonResponse(serde_json::json!({"error":"Unauthorized"}))))
            } else { Ok(()) }
        }),
        delivery: Arc::new(NoDelivery),
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

fn job(schedule: Schedule) -> CronJob {
    let now = Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap();
    CronJob {
        id: "job-1".to_string(),
        expression: "0 0 * * * *".to_string(),
        schedule,
        command: "echo ok".to_string(),
        prompt: Some("summarize".to_string()),
        name: Some("Nightly".to_string()),
        job_type: JobType::Agent,
        session_target: SessionTarget::Main,
        model: Some("model-a".to_string()),
        agent: Some("agent-a".to_string()),
        acp_agent: Some("acp-a".to_string()),
        project_path: Some("/tmp/project".to_string()),
        wake: true,
        fallbacks: vec!["fallback-a".to_string()],
        full_access: true,
        task_pool: true,
        enabled: true,
        delivery: DeliveryConfig {
            mode: "announce".to_string(),
            channel: Some("telegram".to_string()),
            to: Some("me".to_string()),
            best_effort: false,
        },
        delete_after_run: true,
        created_at: now,
        starts_at: None,
        ends_at: None,
        next_run: now,
        last_run: Some(now),
        last_status: Some("ok".to_string()),
        last_output: Some("done".to_string()),
    }
}


async fn history_request(state: AppState) -> (StatusCode, serde_json::Value) {
    response_json(handle_api_cron_history(State(state), HeaderMap::new()).await).await
}


mod policy;
mod lifecycle;
mod history;
mod delivery;
mod schedule;
