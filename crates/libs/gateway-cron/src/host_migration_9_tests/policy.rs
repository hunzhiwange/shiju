use super::*;

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
    body.at = None;
    body.every_ms = Some(0);
    assert!(cron_add_schedule(&body).unwrap_err().contains("greater than 0"));

    body.schedule_kind = Some("at".to_string());
    body.every_ms = None;
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
    assert!(cron_update_schedule(&update_body(), None).unwrap().is_none());

    let mut body = update_body();
    body.schedule = Some("*/5 * * * * *".to_string());
    assert!(matches!(cron_update_schedule(&body, None).unwrap(), Some(Schedule::Cron { .. })));

    let mut body = update_body();
    body.schedule_kind = Some("指定时间".to_string());
    body.at = Some("2026-01-02T03:04:05Z".to_string());
    assert!(matches!(cron_update_schedule(&body, None).unwrap(), Some(Schedule::At { .. })));

    let mut body = update_body();
    body.schedule_kind = Some("固定间隔".to_string());
    body.every_ms = Some(10);
    assert_eq!(cron_update_schedule(&body, None).unwrap(), Some(Schedule::Every { every_ms: 10 }));

    let mut body = update_body();
    body.schedule_kind = Some("cron".to_string());
    assert!(cron_update_schedule(&body, None).unwrap_err().contains("Cron expression"));

    let mut body = update_body();
    body.schedule_kind = Some("weird".to_string());
    assert!(cron_update_schedule(&body, None).unwrap_err().contains("Unsupported"));
}

#[test]
fn cron_update_schedule_rejects_invalid_at_and_every_values() {
    let mut body = update_body();
    body.schedule_kind = Some("at".to_string());
    body.at = Some("tomorrow".to_string());
    assert!(cron_update_schedule(&body, None).unwrap_err().contains("Invalid RFC3339"));

    let mut body = update_body();
    body.schedule_kind = Some("every".to_string());
    body.every_ms = Some(0);
    assert_eq!(cron_update_schedule(&body, None).unwrap_err(), "every_ms must be greater than 0");

    let mut body = update_body();
    body.schedule_kind = Some("Cron".to_string());
    body.schedule = Some(" */10 * * * * * ".to_string());
    assert_eq!(
        cron_update_schedule(&body, None).unwrap(),
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
    assert_eq!(cron_update_schedule(&body, None).unwrap(), None);

    body.schedule = Some(" 0 */15 * * * * ".to_string());
    assert_eq!(
        cron_update_schedule(&body, None).unwrap(),
        Some(Schedule::Cron { expr: "0 */15 * * * *".to_string(), tz: None })
    );

    let mut body = update_body();
    body.schedule_kind = Some("at".to_string());
    assert_eq!(
        cron_update_schedule(&body, None).unwrap_err(),
        "RFC3339 time is required for at schedule"
    );

    let mut body = update_body();
    body.schedule_kind = Some("every".to_string());
    assert_eq!(cron_update_schedule(&body, None).unwrap_err(), "every_ms must be greater than 0");
}
