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
            starts_at: None,
            ends_at: None,
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

fn window_time() -> chrono::DateTime<Utc> {
    "2035-01-01T00:00:00Z".parse().unwrap()
}

#[test]
fn wb_schedule_window_creation_round_trips_and_delays_every() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let start = window_time();
    let end = start + ChronoDuration::hours(1);
    let job = super::store::add_shell_job_with_window(
        &config,
        None,
        Schedule::Every { every_ms: 60_000 },
        "echo ok",
        Some(start),
        Some(end),
    )
    .unwrap();
    assert_eq!(job.next_run, start + ChronoDuration::minutes(1));
    let stored = get_job(&config, &job.id).unwrap();
    assert_eq!((stored.starts_at, stored.ends_at), (Some(start), Some(end)));
    with_connection(&config, |conn| {
        let count: i64 = conn.query_row(
            "SELECT count(*) FROM cron_jobs WHERE starts_at IS NOT NULL AND ends_at IS NOT NULL",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(count, 1);
        Ok(())
    })
    .unwrap();
}

#[test]
fn wb_schedule_window_creation_rejects_empty_interval_atomically() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let start = window_time();
    assert!(
        super::store::add_shell_job_with_window(
            &config,
            None,
            Schedule::Every { every_ms: 60_000 },
            "echo ok",
            Some(start),
            Some(start + ChronoDuration::seconds(30))
        )
        .is_err()
    );
    assert!(list_jobs(&config).unwrap().is_empty());
}

#[test]
fn wb_schedule_window_creation_rejects_reversed_and_equal_bounds() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let start = window_time();
    for end in [start, start - ChronoDuration::seconds(1)] {
        assert!(
            super::store::add_shell_job_with_window(
                &config,
                None,
                Schedule::Every { every_ms: 1000 },
                "echo ok",
                Some(start),
                Some(end)
            )
            .is_err()
        );
    }
    assert!(list_jobs(&config).unwrap().is_empty());
}

#[test]
fn wb_schedule_window_patch_preserves_clears_and_sets() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let job = add_job(&config, "0 * * * * *", "echo ok").unwrap();
    let start = window_time();
    let end = start + ChronoDuration::hours(1);
    let patch: CronJobPatch =
        serde_json::from_value(serde_json::json!({"starts_at": start, "ends_at": end})).unwrap();
    let job = update_job(&config, &job.id, patch).unwrap();
    assert_eq!((job.starts_at, job.ends_at), (Some(start), Some(end)));
    assert_eq!(job.next_run, start);
    let job = update_job(
        &config,
        &job.id,
        CronJobPatch { name: Some("renamed".into()), ..Default::default() },
    )
    .unwrap();
    assert_eq!((job.starts_at, job.ends_at), (Some(start), Some(end)));
    let patch: CronJobPatch =
        serde_json::from_value(serde_json::json!({"starts_at": null})).unwrap();
    assert_eq!(serde_json::to_value(&patch).unwrap()["starts_at"], serde_json::Value::Null);
    let job = update_job(&config, &job.id, patch).unwrap();
    assert_eq!((job.starts_at, job.ends_at), (None, Some(end)));
    let patch: CronJobPatch = serde_json::from_value(serde_json::json!({"ends_at": null})).unwrap();
    assert_eq!(update_job(&config, &job.id, patch).unwrap().ends_at, None);
}

#[test]
fn wb_schedule_window_invalid_patch_does_not_write() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let job = add_job(&config, "0 * * * * *", "echo ok").unwrap();
    let start = window_time();
    let patch = CronJobPatch {
        starts_at: Some(Some(start)),
        ends_at: Some(Some(start)),
        ..Default::default()
    };
    assert!(update_job(&config, &job.id, patch).is_err());
    assert_eq!(get_job(&config, &job.id).unwrap().starts_at, None);
}

#[test]
fn wb_schedule_window_due_query_includes_start_excludes_end() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let start = window_time();
    let end = start + ChronoDuration::hours(1);
    let job = super::store::add_shell_job_with_window(
        &config,
        None,
        Schedule::Cron { expr: "0 * * * * *".into(), tz: None },
        "echo ok",
        Some(start),
        Some(end),
    )
    .unwrap();
    assert_eq!(job.next_run, start);
    assert!(
        super::store::due_jobs(&config, start - ChronoDuration::seconds(1)).unwrap().is_empty()
    );
    assert_eq!(super::store::due_jobs(&config, start).unwrap().len(), 1);
    assert!(super::store::due_jobs(&config, end).unwrap().is_empty());
    assert!(!get_job(&config, &job.id).unwrap().enabled);
}

#[test]
fn wb_schedule_window_offline_expiration_retains_history() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let start = window_time();
    let job = super::store::add_shell_job_with_window(
        &config,
        None,
        Schedule::Every { every_ms: 1000 },
        "echo ok",
        Some(start),
        Some(start + ChronoDuration::minutes(1)),
    )
    .unwrap();
    super::store::record_run(&config, &job.id, start, start, "ok", Some("previous"), 0).unwrap();
    assert!(super::store::due_jobs(&config, start + ChronoDuration::days(1)).unwrap().is_empty());
    assert!(!get_job(&config, &job.id).unwrap().enabled);
    assert_eq!(super::store::list_runs(&config, &job.id, 10).unwrap().len(), 1);
}

#[test]
fn wb_schedule_window_expired_resume_stays_disabled() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let job = add_job(&config, "0 * * * * *", "echo ok").unwrap();
    let job = update_job(
        &config,
        &job.id,
        CronJobPatch {
            ends_at: Some(Some(Utc::now() - ChronoDuration::seconds(1))),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!job.enabled);
    let job =
        update_job(&config, &job.id, CronJobPatch { enabled: Some(true), ..Default::default() })
            .unwrap();
    assert!(!job.enabled);
}

#[test]
fn wb_schedule_window_schedule_change_recalculates_with_bounds() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let start = window_time();
    let job = super::store::add_shell_job_with_window(
        &config,
        None,
        Schedule::Every { every_ms: 1000 },
        "echo ok",
        Some(start),
        Some(start + ChronoDuration::minutes(1)),
    )
    .unwrap();
    let job = update_job(
        &config,
        &job.id,
        CronJobPatch {
            schedule: Some(Schedule::Every { every_ms: 120_000 }),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!job.enabled);
    assert_eq!(job.next_run, start + ChronoDuration::seconds(1));
}

#[test]
fn wb_schedule_window_run_finishes_after_end_without_rescheduling() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let job = add_job(&config, "0 * * * * *", "echo ok").unwrap();
    let job = update_job(
        &config,
        &job.id,
        CronJobPatch {
            ends_at: Some(Some(Utc::now() - ChronoDuration::seconds(1))),
            ..Default::default()
        },
    )
    .unwrap();
    super::store::reschedule_after_run(&config, &job, true, "finished").unwrap();
    let stored = get_job(&config, &job.id).unwrap();
    assert!(!stored.enabled);
    assert_eq!(stored.next_run, job.next_run);
    assert_eq!(stored.last_status.as_deref(), Some("ok"));
    assert_eq!(stored.last_output.as_deref(), Some("finished"));
}

#[test]
fn wb_schedule_window_last_calendar_occurrence_has_no_next() {
    let from = window_time();
    let schedule = Schedule::Cron { expr: "0 0 0 1 1 * 2035".into(), tz: None };
    assert_eq!(
        super::store::next_run_in_window(
            &schedule,
            None,
            Some(from + ChronoDuration::days(1)),
            from
        )
        .unwrap(),
        None
    );
}

#[test]
fn wb_schedule_window_candidate_at_end_is_excluded() {
    let from = window_time();
    let schedule = Schedule::Every { every_ms: 60_000 };
    assert_eq!(
        super::store::next_run_in_window(
            &schedule,
            Some(from),
            Some(from + ChronoDuration::minutes(1)),
            from
        )
        .unwrap(),
        None
    );
}

#[test]
fn wb_schedule_window_agent_creation_is_atomic() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);
    let start = window_time();
    let end = start + ChronoDuration::hours(1);
    let job = super::store::add_agent_job_with_window(
        &config,
        None,
        Schedule::Every { every_ms: 60_000 },
        "summarize",
        SessionTarget::Isolated,
        None,
        None,
        false,
        Some(start),
        Some(end),
    )
    .unwrap();
    assert_eq!((job.starts_at, job.ends_at), (Some(start), Some(end)));
    assert_eq!(job.next_run, start + ChronoDuration::minutes(1));
    assert!(
        super::store::add_agent_job_with_window(
            &config,
            None,
            Schedule::Every { every_ms: 3_600_000 },
            "summarize",
            SessionTarget::Isolated,
            None,
            None,
            false,
            Some(start),
            Some(end)
        )
        .is_err()
    );
    assert_eq!(list_jobs(&config).unwrap().len(), 1);
}
