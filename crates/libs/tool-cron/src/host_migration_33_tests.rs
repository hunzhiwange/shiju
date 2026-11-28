//! Security denials must not reach the host execution port.
use crate::cron_run::{CronRunTool, JobExecutionFuture};
use tool_types::Tool;
use agent_config::runtime::Config;
use agent_cron::{add_job, list_runs};
use agent_security::{AutonomyLevel, SecurityPolicy};
use std::sync::Arc;
use serde_json::json;
use tempfile::TempDir;
fn execute_job_now<'a>(_: &'a Config, _: &'a agent_cron::CronJob) -> JobExecutionFuture<'a> { panic!("denied job reached execution port") }
async fn test_config(tmp: &TempDir) -> Arc<Config> {
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    tokio::fs::create_dir_all(&config.workspace_dir).await.unwrap();
    Arc::new(config)
}

fn test_security(cfg: &Config) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy::from_config(&cfg.autonomy, &cfg.workspace_dir))
}

#[tokio::test]
async fn errors_for_missing_job() {
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;
    let tool = CronRunTool::new(cfg.clone(), test_security(&cfg), execute_job_now);

    let result = tool.execute(json!({ "job_id": "missing-job-id" })).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap_or_default().contains("not found"));
}
#[tokio::test]
async fn blocks_run_in_read_only_mode() {
    let tmp = TempDir::new().unwrap();
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.autonomy.level = AutonomyLevel::ReadOnly;
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let cfg = Arc::new(config);
    let job = add_job(&cfg, "*/5 * * * *", "echo run-now").unwrap();
    let tool = CronRunTool::new(cfg.clone(), test_security(&cfg), execute_job_now);

    let result = tool.execute(json!({ "job_id": job.id })).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap_or_default().contains("read-only"));
}
#[tokio::test]
async fn blocks_run_when_rate_limited() {
    let tmp = TempDir::new().unwrap();
    let mut config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.autonomy.level = AutonomyLevel::Full;
    config.autonomy.max_actions_per_hour = 0;
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let cfg = Arc::new(config);
    let job = add_job(&cfg, "*/5 * * * *", "echo run-now").unwrap();
    let tool = CronRunTool::new(cfg.clone(), test_security(&cfg), execute_job_now);

    let result = tool.execute(json!({ "job_id": job.id })).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap_or_default().contains("Rate limit exceeded"));
    assert!(list_runs(&cfg, &job.id, 10).unwrap().is_empty());
}

#[tokio::test]
async fn window_rejection_never_reaches_executor_or_records_run() {
    let tmp = TempDir::new().unwrap();
    let cfg = test_config(&tmp).await;
    let start = chrono::Utc::now() + chrono::Duration::days(1);
    let job = agent_cron::add_shell_job_with_window(
        &cfg,
        None,
        agent_cron::Schedule::Every { every_ms: 60_000 },
        "echo run-now",
        Some(start),
        None,
    ).unwrap();
    let tool = CronRunTool::new(cfg.clone(), test_security(&cfg), execute_job_now);

    let result = tool.execute(json!({"job_id": job.id})).await.unwrap();
    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("Execution window has not started"));
    assert!(list_runs(&cfg, &job.id, 10).unwrap().is_empty());
    assert!(agent_cron::active_runs::snapshot(&cfg).is_empty());
}
