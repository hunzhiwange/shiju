use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::cron::{CronJob, CronRun, DeliveryConfig, JobType, Schedule, SessionTarget};
use crate::app::agent::gateway::api::types::{CronAddBody, CronUpdateBody};
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use axum::body::to_bytes;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use chrono::{TimeZone, Utc};
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
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(require_pairing, &[])),
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
        max_tool_iterations: 10,
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: None,
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
        next_run: now,
        last_run: Some(now),
        last_status: Some("ok".to_string()),
        last_output: Some("done".to_string()),
    }
}

#[test]
fn cron_json_serializes_schedule_variants_and_delivery_fields() {
    let at = Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap();
    let cron = cron_job_json(&job(Schedule::Cron { expr: "* * * * * *".to_string(), tz: None }));
    assert_eq!(cron["schedule_kind"], "cron");
    assert_eq!(cron["job_type"], "agent");
    assert_eq!(cron["delivery_mode"], "announce");
    assert_eq!(cron["delivery_best_effort"], false);
    assert_eq!(cron["wake"], true);
    assert_eq!(cron["fallbacks"], serde_json::json!(["fallback-a"]));

    let at_job = cron_job_json(&job(Schedule::At { at }));
    assert_eq!(at_job["schedule_kind"], "at");
    assert_eq!(at_job["at"], at.to_rfc3339());

    let every_job = cron_job_json(&job(Schedule::Every { every_ms: 1234 }));
    assert_eq!(every_job["schedule_kind"], "every");
    assert_eq!(every_job["every_ms"], 1234);

    let run = CronRun {
        id: 7,
        job_id: "job-1".to_string(),
        started_at: at,
        finished_at: at + chrono::Duration::seconds(2),
        status: "success".to_string(),
        output: Some("ok".to_string()),
        duration_ms: Some(2000),
    };
    let value = cron_run_json(&run);
    assert_eq!(value["id"], 7);
    assert_eq!(value["job_id"], "job-1");
    assert_eq!(value["duration_ms"], 2000);
}

#[test]
fn cron_json_serializes_absent_optional_job_and_run_fields_as_null() {
    let at = Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap();
    let mut cron_job = job(Schedule::At { at });
    cron_job.agent = None;
    cron_job.acp_agent = None;
    cron_job.project_path = None;
    cron_job.delivery.channel = None;
    cron_job.delivery.to = None;
    cron_job.last_run = None;
    cron_job.last_status = None;
    cron_job.last_output = None;

    let value = cron_job_json(&cron_job);
    assert!(value["agent"].is_null());
    assert!(value["acp_agent"].is_null());
    assert!(value["project_path"].is_null());
    assert!(value["delivery_channel"].is_null());
    assert!(value["delivery_to"].is_null());
    assert!(value["last_run"].is_null());
    assert!(value["last_status"].is_null());
    assert!(value["last_output"].is_null());

    let run = CronRun {
        id: 8,
        job_id: "job-1".to_string(),
        started_at: at,
        finished_at: at,
        status: "running".to_string(),
        output: None,
        duration_ms: None,
    };
    let value = cron_run_json(&run);
    assert!(value["output"].is_null());
    assert!(value["duration_ms"].is_null());
}

#[test]
fn cron_add_schedule_infers_and_validates_kinds() {
    let mut body = add_body();
    assert!(matches!(cron_add_schedule(&body).unwrap(), Schedule::Cron { .. }));
    assert_eq!(effective_schedule_kind(&body), "cron");

    body.schedule = None;
    body.every_ms = Some(500);
    assert_eq!(effective_schedule_kind(&body), "every");
    assert_eq!(cron_add_schedule(&body).unwrap(), Schedule::Every { every_ms: 500 });

    body.every_ms = None;
    body.at = Some("2026-01-02T03:04:05Z".to_string());
    assert_eq!(effective_schedule_kind(&body), "at");
    assert!(matches!(cron_add_schedule(&body).unwrap(), Schedule::At { .. }));

    body.schedule_kind = Some("fixed".to_string());
    assert!(cron_add_schedule(&body).unwrap_err().contains("Unsupported"));

    body.schedule_kind = Some("every".to_string());
    body.every_ms = Some(0);
    assert!(cron_add_schedule(&body).unwrap_err().contains("greater than 0"));

    body.schedule_kind = Some("at".to_string());
    body.at = Some("not-time".to_string());
    assert!(cron_add_schedule(&body).unwrap_err().contains("Invalid RFC3339"));
}

#[test]
fn cron_add_schedule_reports_missing_values_and_accepts_localized_kinds() {
    let mut body = add_body();
    body.schedule = Some("   ".to_string());
    assert_eq!(cron_add_schedule(&body).unwrap_err(), "Cron expression is required");

    body.schedule_kind = Some("指定时间".to_string());
    body.at = Some(" 2026-01-02T03:04:05Z ".to_string());
    assert_eq!(
        cron_add_schedule(&body).unwrap(),
        Schedule::At { at: Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap() }
    );

    body.schedule_kind = Some("固定间隔".to_string());
    body.at = None;
    body.every_ms = Some(250);
    assert_eq!(cron_add_schedule(&body).unwrap(), Schedule::Every { every_ms: 250 });

    body.schedule_kind = Some("at".to_string());
    body.every_ms = None;
    assert_eq!(cron_add_schedule(&body).unwrap_err(), "RFC3339 time is required for at schedule");

    body.schedule_kind = Some("every".to_string());
    assert_eq!(cron_add_schedule(&body).unwrap_err(), "every_ms must be greater than 0");
}

#[test]
fn cron_update_schedule_supports_partial_updates_and_localized_kinds() {
    assert!(cron_update_schedule(&update_body()).unwrap().is_none());

    let mut body = update_body();
    body.schedule = Some("*/5 * * * * *".to_string());
    assert!(matches!(cron_update_schedule(&body).unwrap(), Some(Schedule::Cron { .. })));

    let mut body = update_body();
    body.schedule_kind = Some("指定时间".to_string());
    body.at = Some("2026-01-02T03:04:05Z".to_string());
    assert!(matches!(cron_update_schedule(&body).unwrap(), Some(Schedule::At { .. })));

    let mut body = update_body();
    body.schedule_kind = Some("固定间隔".to_string());
    body.every_ms = Some(10);
    assert_eq!(cron_update_schedule(&body).unwrap(), Some(Schedule::Every { every_ms: 10 }));

    let mut body = update_body();
    body.schedule_kind = Some("cron".to_string());
    assert!(cron_update_schedule(&body).unwrap_err().contains("Cron expression"));

    let mut body = update_body();
    body.schedule_kind = Some("weird".to_string());
    assert!(cron_update_schedule(&body).unwrap_err().contains("Unsupported"));
}

#[test]
fn cron_update_schedule_rejects_invalid_at_and_every_values() {
    let mut body = update_body();
    body.schedule_kind = Some("at".to_string());
    body.at = Some("tomorrow".to_string());
    assert!(cron_update_schedule(&body).unwrap_err().contains("Invalid RFC3339"));

    let mut body = update_body();
    body.schedule_kind = Some("every".to_string());
    body.every_ms = Some(0);
    assert_eq!(cron_update_schedule(&body).unwrap_err(), "every_ms must be greater than 0");

    let mut body = update_body();
    body.schedule_kind = Some("Cron".to_string());
    body.schedule = Some(" */10 * * * * * ".to_string());
    assert_eq!(
        cron_update_schedule(&body).unwrap(),
        Some(Schedule::Cron { expr: "*/10 * * * * *".to_string(), tz: None })
    );
}

#[test]
fn cron_add_job_type_delivery_patch_and_log_fields_normalize_input() {
    let mut body = add_body();
    body.job_type = Some(" Agent ".to_string());
    body.prompt = Some(" work ".to_string());
    body.command = Some(" ".to_string());
    body.delivery_mode = Some(" announce ".to_string());
    body.delivery_channel = Some(" telegram ".to_string());
    body.delivery_to = Some(" chat ".to_string());
    body.delivery_best_effort = Some(false);
    body.agent = Some(" agent-key ".to_string());
    body.acp_agent = Some("acp".to_string());
    body.project_path = Some(" /tmp/project ".to_string());
    body.wake = Some(true);
    body.fallbacks = Some(vec![" a ".to_string(), "a".to_string(), "".to_string()]);
    body.full_access = Some(true);
    body.task_pool = Some(true);
    body.delete_after_run = Some(true);

    assert_eq!(cron_add_job_type(&body), "agent");
    let delivery = cron_add_delivery(&body);
    assert_eq!(delivery.mode, "announce");
    assert_eq!(delivery.channel.as_deref(), Some("telegram"));
    assert_eq!(delivery.to.as_deref(), Some("chat"));
    assert!(!delivery.best_effort);

    let patch = cron_add_patch(&body);
    assert_eq!(patch.agent.as_deref(), Some("agent-key"));
    assert_eq!(patch.acp_agent.as_deref(), Some("acp"));
    assert_eq!(patch.project_path.as_deref(), Some("/tmp/project"));
    assert_eq!(patch.fallbacks.unwrap(), vec!["a"]);
    assert_eq!(patch.full_access, Some(true));
    assert_eq!(patch.task_pool, Some(true));
    assert_eq!(patch.delete_after_run, Some(true));

    let fields = cron_add_body_log_fields(&body);
    assert_eq!(fields.0, "Agent");
    assert_eq!(fields.1, "cron");
    assert!(fields.3);
    assert!(!fields.5);
    assert!(fields.6);
}

#[test]
fn cron_add_delivery_and_patch_apply_safe_defaults_for_blank_values() {
    let mut body = add_body();
    body.delivery_mode = Some("  ".to_string());
    body.delivery_channel = Some("  ".to_string());
    body.delivery_to = Some("  ".to_string());
    body.agent = Some("  ".to_string());
    body.project_path = Some("  ".to_string());
    body.fallbacks = Some(vec![" ".to_string(), "model-a".to_string(), "model-a".to_string()]);

    let delivery = cron_add_delivery(&body);
    assert_eq!(delivery.mode, "none");
    assert_eq!(delivery.channel, None);
    assert_eq!(delivery.to, None);
    assert!(delivery.best_effort);

    let patch = cron_add_patch(&body);
    assert_eq!(patch.delivery, Some(delivery));
    assert_eq!(patch.agent, None);
    assert_eq!(patch.project_path, None);
    assert_eq!(patch.fallbacks, Some(vec!["model-a".to_string()]));
}

#[test]
fn cron_add_job_type_defaults_from_prompt_and_command() {
    let mut body = add_body();
    body.job_type = Some(" ".to_string());
    body.command = None;
    body.prompt = Some("do it".to_string());
    assert_eq!(cron_add_job_type(&body), "agent");

    body.prompt = None;
    assert_eq!(cron_add_job_type(&body), "shell");
    assert_eq!(trimmed_optional(Some("  value  ".to_string())).as_deref(), Some("value"));
    assert_eq!(trimmed_optional(Some("  ".to_string())), None);
    assert_eq!(trimmed_optional(None), None);
}

#[test]
fn cron_update_schedule_treats_blank_kind_as_default_and_requires_matching_value() {
    let mut body = update_body();
    body.schedule_kind = Some("   ".to_string());
    assert_eq!(cron_update_schedule(&body).unwrap(), None);

    body.schedule = Some(" 0 */15 * * * * ".to_string());
    assert_eq!(
        cron_update_schedule(&body).unwrap(),
        Some(Schedule::Cron { expr: "0 */15 * * * *".to_string(), tz: None })
    );

    let mut body = update_body();
    body.schedule_kind = Some("at".to_string());
    assert_eq!(
        cron_update_schedule(&body).unwrap_err(),
        "RFC3339 time is required for at schedule"
    );

    let mut body = update_body();
    body.schedule_kind = Some("every".to_string());
    assert_eq!(cron_update_schedule(&body).unwrap_err(), "every_ms must be greater than 0");
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
async fn cron_handlers_manage_shell_job_lifecycle() {
    let dir = tempfile::tempdir().unwrap();
    let config = temp_config(dir.path());
    let headers = HeaderMap::new();
    let mut body = add_body();
    body.schedule_kind = Some("every".to_string());
    body.schedule = None;
    body.every_ms = Some(60_000);
    body.command = Some("echo ok".to_string());

    let (status, add_json) = response_json(
        handle_api_cron_add(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            JsonResponse(body),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(add_json["status"], "ok");
    assert_eq!(add_json["job"]["schedule_kind"], "every");
    let job_id = add_json["job"]["id"].as_str().unwrap().to_string();

    let (status, list_json) = response_json(
        handle_api_cron_list(State(state_with_config(config.clone(), false)), headers.clone())
            .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list_json["jobs"].as_array().unwrap().len(), 1);

    let mut update = update_body();
    update.name = Some("Updated".to_string());
    update.schedule_kind = Some("cron".to_string());
    update.schedule = Some("0 */5 * * * *".to_string());
    update.command = Some("echo updated".to_string());
    update.enabled = Some(false);
    let (status, update_json) = response_json(
        handle_api_cron_update(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            Path(job_id.clone()),
            JsonResponse(update),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(update_json["job"]["name"], "Updated");
    assert_eq!(update_json["job"]["enabled"], false);

    let (status, runs_json) = response_json(
        handle_api_cron_runs(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            Path(job_id.clone()),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(runs_json["runs"].as_array().unwrap().is_empty());

    let (status, delete_json) = response_json(
        handle_api_cron_delete(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            Path(job_id),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(delete_json["status"], "ok");

    let (status, list_json) =
        response_json(handle_api_cron_list(State(state_with_config(config, false)), headers).await)
            .await;
    assert_eq!(status, StatusCode::OK);
    assert!(list_json["jobs"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn cron_add_handler_covers_agent_and_bad_request_branches() {
    let dir = tempfile::tempdir().unwrap();
    let config = temp_config(dir.path());
    let headers = HeaderMap::new();

    let mut missing_command = add_body();
    missing_command.command = Some(" ".to_string());
    let (status, body) = response_json(
        handle_api_cron_add(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            JsonResponse(missing_command),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("Shell command"));

    let mut bad_schedule = add_body();
    bad_schedule.schedule_kind = Some("every".to_string());
    bad_schedule.schedule = None;
    bad_schedule.every_ms = Some(0);
    let (status, body) = response_json(
        handle_api_cron_add(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            JsonResponse(bad_schedule),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("every_ms"));

    let mut invalid_cron = add_body();
    invalid_cron.schedule = Some("not a cron".to_string());
    let (status, body) = response_json(
        handle_api_cron_add(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            JsonResponse(invalid_cron),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("Failed to add cron job"));

    let mut missing_prompt = add_body();
    missing_prompt.job_type = Some("agent".to_string());
    missing_prompt.command = None;
    missing_prompt.prompt = Some(" ".to_string());
    let (status, body) = response_json(
        handle_api_cron_add(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            JsonResponse(missing_prompt),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("Agent prompt"));

    let mut agent = add_body();
    agent.job_type = Some("agent".to_string());
    agent.command = None;
    agent.prompt = Some("summarize daily state".to_string());
    agent.session_target = Some("main".to_string());
    agent.model = Some("model-a".to_string());
    agent.delivery_mode = Some("announce".to_string());
    agent.delivery_channel = Some("telegram".to_string());
    agent.delivery_to = Some("me".to_string());
    agent.delete_after_run = Some(true);
    let (status, body) = response_json(
        handle_api_cron_add(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            JsonResponse(agent),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["job"]["job_type"], "agent");
    assert_eq!(body["job"]["delivery_mode"], "announce");
    assert_eq!(body["job"]["delete_after_run"], true);

    let mut unsupported = add_body();
    unsupported.job_type = Some("python".to_string());
    let (status, body) = response_json(
        handle_api_cron_add(
            State(state_with_config(config, false)),
            headers,
            JsonResponse(unsupported),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("Unsupported job_type"));
}

#[tokio::test]
async fn cron_update_and_delete_handlers_report_errors() {
    let dir = tempfile::tempdir().unwrap();
    let config = temp_config(dir.path());
    let headers = HeaderMap::new();

    let mut update = update_body();
    update.schedule_kind = Some("at".to_string());
    let (status, body) = response_json(
        handle_api_cron_update(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            Path("missing".to_string()),
            JsonResponse(update),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("RFC3339"));

    let mut update = update_body();
    update.job_type = Some("invalid".to_string());
    let (status, body) = response_json(
        handle_api_cron_update(
            State(state_with_config(config.clone(), false)),
            headers.clone(),
            Path("missing".to_string()),
            JsonResponse(update),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("Invalid job type"));

    let (status, body) = response_json(
        handle_api_cron_delete(
            State(state_with_config(config, false)),
            headers,
            Path("missing".to_string()),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(body["error"].as_str().unwrap().contains("Failed to remove"));
}

#[tokio::test]
async fn cron_update_handler_reports_missing_job_after_valid_patch() {
    let dir = tempfile::tempdir().unwrap();
    let config = temp_config(dir.path());
    let mut update = update_body();
    update.name = Some("Updated".to_string());
    update.delivery_channel = Some(" telegram ".to_string());

    let (status, body) = response_json(
        handle_api_cron_update(
            State(state_with_config(config, false)),
            HeaderMap::new(),
            Path("missing".to_string()),
            JsonResponse(update),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(body["error"].as_str().unwrap().contains("Failed to update cron job"));
}
