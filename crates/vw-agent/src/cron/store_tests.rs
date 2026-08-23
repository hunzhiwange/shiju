// 覆盖定时任务存储的增删改查、运行记录与持久化边界。

use super::store::{
    add_agent_job, add_job, get_job, list_jobs, remove_job, update_job, with_connection,
};
use super::{CronJobPatch, DeliveryConfig, JobType, Schedule, SessionTarget};
use crate::app::agent::config::Config;
use chrono::{Duration as ChronoDuration, Utc};
use rusqlite::params;
use tempfile::TempDir;

#[path = "tests/store.rs"]
mod legacy_store_tests;

fn test_config(tmp: &TempDir) -> Config {
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    config
}

// 固定时间与临时存储夹具用于隔离真实时钟和持久化状态，保证调度断言可重复。
#[test]
fn add_agent_job_persists_metadata_and_defaults() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let delivery = DeliveryConfig {
        mode: "announce".into(),
        channel: Some("telegram".into()),
        to: Some("123".into()),
        best_effort: false,
    };

    let job = add_agent_job(
        &config,
        Some("daily-agent".into()),
        Schedule::Every { every_ms: 60_000 },
        "summarize the project",
        SessionTarget::Main,
        Some("gpt-test".into()),
        Some(delivery.clone()),
        true,
    )
    .unwrap();

    assert_eq!(job.name.as_deref(), Some("daily-agent"));
    assert_eq!(job.job_type, JobType::Agent);
    assert_eq!(job.command, "");
    assert_eq!(job.prompt.as_deref(), Some("summarize the project"));
    assert_eq!(job.session_target, SessionTarget::Main);
    assert_eq!(job.model.as_deref(), Some("gpt-test"));
    assert_eq!(job.delivery, delivery);
    assert!(job.delete_after_run);
    assert!(job.enabled);

    let stored = get_job(&config, &job.id).unwrap();
    assert_eq!(stored.id, job.id);
    assert_eq!(stored.job_type, JobType::Agent);
    assert_eq!(stored.schedule, Schedule::Every { every_ms: 60_000 });
}

#[test]
fn update_job_applies_extended_patch_and_normalizes_fallbacks() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let job = add_job(&config, "*/5 * * * *", "echo original").unwrap();

    let patched = update_job(
        &config,
        &job.id,
        CronJobPatch {
            job_type: Some(JobType::Agent),
            schedule: Some(Schedule::Every { every_ms: 120_000 }),
            command: Some("echo changed".into()),
            prompt: Some("new prompt".into()),
            name: Some("renamed".into()),
            enabled: Some(false),
            delivery: Some(DeliveryConfig {
                mode: "announce".into(),
                channel: Some("slack".into()),
                to: Some("#ops".into()),
                best_effort: true,
            }),
            model: Some("model-a".into()),
            session_target: Some(SessionTarget::Main),
            delete_after_run: Some(true),
            agent: Some("delegate".into()),
            acp_agent: Some("acp".into()),
            project_path: Some("/tmp/project".into()),
            wake: Some(true),
            fallbacks: Some(vec![
                " fallback-a ".into(),
                "".into(),
                "fallback-a".into(),
                "fallback-b".into(),
            ]),
            full_access: Some(true),
            task_pool: Some(true),
        },
    )
    .unwrap();

    assert_eq!(patched.job_type, JobType::Agent);
    assert_eq!(patched.schedule, Schedule::Every { every_ms: 120_000 });
    assert_eq!(patched.expression, "");
    assert_eq!(patched.command, "echo changed");
    assert_eq!(patched.prompt.as_deref(), Some("new prompt"));
    assert_eq!(patched.name.as_deref(), Some("renamed"));
    assert!(!patched.enabled);
    assert_eq!(patched.session_target, SessionTarget::Main);
    assert_eq!(patched.agent.as_deref(), Some("delegate"));
    assert_eq!(patched.acp_agent.as_deref(), Some("acp"));
    assert_eq!(patched.project_path.as_deref(), Some("/tmp/project"));
    assert!(patched.wake);
    assert_eq!(patched.fallbacks, vec!["fallback-a".to_string(), "fallback-b".to_string()]);
    assert!(patched.full_access);
    assert!(patched.task_pool);
    assert!(patched.next_run > job.created_at);
}

#[test]
fn list_jobs_orders_by_next_run() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let later = Utc::now() + ChronoDuration::hours(2);
    let earlier = Utc::now() + ChronoDuration::hours(1);

    let later_job =
        super::add_shell_job(&config, None, Schedule::At { at: later }, "echo later").unwrap();
    let earlier_job =
        super::add_shell_job(&config, None, Schedule::At { at: earlier }, "echo earlier").unwrap();

    let jobs = list_jobs(&config).unwrap();
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].id, earlier_job.id);
    assert_eq!(jobs[1].id, later_job.id);
}

// 不存在的任务与存储错误必须显式返回，不能被调度器当成一次成功的空操作。
#[test]
fn get_and_remove_missing_jobs_report_not_found() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);

    let get_err = get_job(&config, "missing").unwrap_err().to_string();
    assert!(get_err.contains("not found"));

    let remove_err = remove_job(&config, "missing").unwrap_err().to_string();
    assert!(remove_err.contains("not found"));
}

#[test]
fn invalid_persisted_schedule_json_is_reported() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let now = Utc::now();

    with_connection(&config, |conn| {
        conn.execute(
            "INSERT INTO cron_jobs (id, expression, command, schedule, created_at, next_run)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "bad-schedule",
                "*/5 * * * *",
                "echo bad",
                "{not-json",
                now.to_rfc3339(),
                (now + ChronoDuration::minutes(5)).to_rfc3339(),
            ],
        )?;
        Ok(())
    })
    .unwrap();

    let err = get_job(&config, "bad-schedule").unwrap_err().to_string();
    assert!(err.contains("Failed to parse cron schedule JSON"));
}

#[test]
fn missing_schedule_and_empty_legacy_expression_are_reported() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let now = Utc::now();

    with_connection(&config, |conn| {
        conn.execute(
            "INSERT INTO cron_jobs (id, expression, command, schedule, created_at, next_run)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5)",
            params![
                "missing-schedule",
                "",
                "echo missing",
                now.to_rfc3339(),
                (now + ChronoDuration::minutes(5)).to_rfc3339(),
            ],
        )?;
        Ok(())
    })
    .unwrap();

    let err = get_job(&config, "missing-schedule").unwrap_err().to_string();
    assert!(err.contains("Missing schedule and legacy expression"));
}

#[test]
fn invalid_persisted_delivery_and_fallback_json_are_reported() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let delivery_job = add_job(&config, "*/5 * * * *", "echo delivery").unwrap();
    let fallback_job = add_job(&config, "*/5 * * * *", "echo fallback").unwrap();

    with_connection(&config, |conn| {
        conn.execute(
            "UPDATE cron_jobs SET delivery = ?1 WHERE id = ?2",
            params!["{not-json", &delivery_job.id],
        )?;
        conn.execute(
            "UPDATE cron_jobs SET fallbacks = ?1 WHERE id = ?2",
            params!["{not-json", &fallback_job.id],
        )?;
        Ok(())
    })
    .unwrap();

    let delivery_err = get_job(&config, &delivery_job.id).unwrap_err().to_string();
    assert!(delivery_err.contains("Failed to parse cron delivery JSON"));

    let fallback_err = get_job(&config, &fallback_job.id).unwrap_err().to_string();
    assert!(fallback_err.contains("Failed to parse cron fallbacks JSON"));
}
