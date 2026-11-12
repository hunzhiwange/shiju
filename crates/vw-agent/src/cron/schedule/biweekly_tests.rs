use super::{next_run_for_schedule, schedule_cron_expression, validate_schedule};
use crate::app::agent::cron::Schedule;
use chrono::{DateTime, NaiveDate, Utc};

fn utc(value: &str) -> DateTime<Utc> {
    value.parse().unwrap()
}

fn rule(anchor: &str, weekdays: &[u32], hour: u32, minute: u32, timezone: &str) -> Schedule {
    Schedule::Biweekly {
        anchor_date: anchor.parse::<NaiveDate>().unwrap(),
        weekdays: weekdays.to_vec(),
        hour,
        minute,
        timezone: timezone.into(),
    }
}

fn next(schedule: &Schedule, from: &str, expected: &str) {
    assert_eq!(next_run_for_schedule(schedule, utc(from)).unwrap(), utc(expected));
}

#[test]
fn wb_schedule_biweekly_preserves_anchor_across_year() {
    let schedule = rule("2020-12-28", &[1], 9, 0, "UTC");
    next(&schedule, "2020-12-28T09:00:00Z", "2021-01-11T09:00:00Z");
    next(&schedule, "2021-01-11T09:00:00Z", "2021-01-25T09:00:00Z");
}

#[test]
fn wb_schedule_biweekly_selects_multiple_weekdays_in_same_week() {
    let schedule = rule("2026-11-12", &[5, 1, 3, 3], 9, 30, "Asia/Shanghai");
    next(&schedule, "2026-11-09T01:29:59Z", "2026-11-09T01:30:00Z");
    next(&schedule, "2026-11-09T01:30:00Z", "2026-11-11T01:30:00Z");
    next(&schedule, "2026-11-11T01:30:00Z", "2026-11-13T01:30:00Z");
    next(&schedule, "2026-11-13T01:30:00Z", "2026-11-23T01:30:00Z");
}

#[test]
fn wb_schedule_biweekly_starts_at_anchor_week() {
    let schedule = rule("2026-11-12", &[1], 9, 0, "UTC");
    next(&schedule, "2020-01-01T00:00:00Z", "2026-11-09T09:00:00Z");
}

#[test]
fn wb_schedule_biweekly_skips_missed_active_week() {
    let schedule = rule("2026-11-09", &[1, 7], 9, 0, "UTC");
    next(&schedule, "2026-11-16T00:00:00Z", "2026-11-23T09:00:00Z");
    next(&schedule, "2030-01-01T00:00:00Z", "2030-01-06T09:00:00Z");
}

#[test]
fn wb_schedule_biweekly_skips_nonexistent_dst_time() {
    let schedule = rule("2026-03-02", &[7], 2, 30, "America/New_York");
    next(&schedule, "2026-03-01T00:00:00Z", "2026-03-22T06:30:00Z");
}

#[test]
fn wb_schedule_biweekly_orders_ambiguous_dst_times() {
    let schedule = rule("2026-10-26", &[7], 1, 30, "America/New_York");
    next(&schedule, "2026-11-01T05:29:59Z", "2026-11-01T05:30:00Z");
    next(&schedule, "2026-11-01T05:30:00Z", "2026-11-01T06:30:00Z");
    next(&schedule, "2026-11-01T06:15:00Z", "2026-11-01T06:30:00Z");
    next(&schedule, "2026-11-01T06:30:00Z", "2026-11-15T06:30:00Z");
}

#[test]
fn wb_schedule_biweekly_rejects_invalid_fields() {
    for schedule in [
        rule("2026-11-09", &[], 9, 0, "UTC"),
        rule("2026-11-09", &[0], 9, 0, "UTC"),
        rule("2026-11-09", &[8], 9, 0, "UTC"),
        rule("2026-11-09", &[1], 24, 0, "UTC"),
        rule("2026-11-09", &[1], 9, 60, "UTC"),
        rule("2026-11-09", &[1], 9, 0, "Invalid/Zone"),
    ] {
        assert!(validate_schedule(&schedule, utc("2026-01-01T00:00:00Z")).is_err());
        assert!(next_run_for_schedule(&schedule, utc("2026-01-01T00:00:00Z")).is_err());
    }
}

#[test]
fn wb_schedule_biweekly_has_no_cron_expression() {
    assert_eq!(schedule_cron_expression(&rule("2026-11-09", &[1], 9, 0, "UTC")), None);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn wb_schedule_biweekly_round_trips_sqlite() {
    use crate::app::agent::{
        config::Config,
        cron::{add_shell_job, get_job},
    };
    let tmp = tempfile::TempDir::new().unwrap();
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("config.json"),
        ..Config::default()
    };
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    let schedule = rule("2026-11-09", &[1, 5], 9, 15, "Asia/Shanghai");
    let job = add_shell_job(&config, None, schedule.clone(), "echo test").unwrap();
    let stored = get_job(&config, &job.id).unwrap();
    assert_eq!(stored.schedule, schedule);
    assert!(stored.expression.is_empty());
    assert!(!stored.delete_after_run);
}

#[test]
fn wb_schedule_biweekly_preserves_wall_clock_across_dst() {
    let schedule = rule("2026-02-23", &[1], 9, 0, "America/New_York");
    next(&schedule, "2026-02-23T14:00:00Z", "2026-03-09T13:00:00Z");
}

#[test]
fn wb_schedule_biweekly_rejects_exhausted_date_range() {
    let schedule = rule("2026-11-09", &[1, 2, 3, 4, 5, 6, 7], 23, 59, "UTC");
    assert!(next_run_for_schedule(&schedule, DateTime::<Utc>::MAX_UTC).is_err());
}
