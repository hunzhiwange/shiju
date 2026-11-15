use super::*;
use gateway_cron::{CronJobDto, CronRunDto};
fn job(id: &str) -> CronJobDto {
    CronJobDto {
        timezone: None,
        anchor_date: None,
        weekdays: None,
        hour: None,
        minute: None,
        starts_at: None,
        ends_at: None,

        id: id.to_string(),
        name: Some(format!("job-{id}")),
        job_type: "shell".to_string(),
        schedule_kind: "cron".to_string(),
        expression: "* * * * *".to_string(),
        at: None,
        every_ms: None,
        command: "echo hi".to_string(),
        prompt: None,
        model: Some("openai/gpt-5".to_string()),
        agent: Some("main".to_string()),
        acp_agent: Some("codex".to_string()),
        project_path: Some("/tmp/project".to_string()),
        wake: false,
        fallbacks: vec!["slack".to_string()],
        full_access: false,
        task_pool: false,
        delivery_mode: "channel".to_string(),
        delivery_channel: Some("slack".to_string()),
        delivery_to: Some("#ops".to_string()),
        delivery_best_effort: true,
        delete_after_run: false,
        next_run: "2026-06-11T00:00:00Z".to_string(),
        last_run: None,
        last_status: None,
        last_output: None,
        enabled: true,
    }
}

fn run(id: i64, output: Option<&str>, duration_ms: Option<i64>) -> CronRunDto {
    CronRunDto {
        id,
        job_id: "job-1".to_string(),
        started_at: format!("start-{id}"),
        finished_at: format!("end-{id}"),
        status: "ok".to_string(),
        output: output.map(str::to_string),
        duration_ms,
    }
}

#[test]
fn format_run_history_text_formats_multiple_runs_and_missing_values() {
    let runs = vec![run(1, Some("hello"), Some(12)), run(2, None, None)];

    assert_eq!(
        format_run_history_text(config_ui::DesktopLocale::SimplifiedChinese, &runs),
        concat!(
            "#1\n状态: ok\n开始: start-1\n结束: end-1\n",
            "耗时: 12 ms\n输出:\nhello\n\n",
            "#2\n状态: ok\n开始: start-2\n结束: end-2\n",
            "耗时: 无\n输出:\n无输出"
        )
    );
    assert_eq!(
        format_run_history_text(config_ui::DesktopLocale::SimplifiedChinese, &[]),
        ""
    );
}

#[test]
fn schedule_kind_from_api_honors_aliases_and_fallback_priority() {
    assert_eq!(schedule_kind_from_api(" AT ", "", None, None), CronAddScheduleKind::At);
    assert_eq!(schedule_kind_from_api("指定时间", "", None, None), CronAddScheduleKind::At);
    assert_eq!(schedule_kind_from_api("Every", "", None, None), CronAddScheduleKind::Every);
    assert_eq!(schedule_kind_from_api("固定间隔", "", None, None), CronAddScheduleKind::Every);
    assert_eq!(
        schedule_kind_from_api("cron", "", Some("10:00"), Some(1_000)),
        CronAddScheduleKind::Cron
    );
    assert_eq!(
        schedule_kind_from_api("unknown", "* * * * *", Some("10:00"), Some(1_000)),
        CronAddScheduleKind::Every
    );
    assert_eq!(
        schedule_kind_from_api("unknown", "* * * * *", Some(" 10:00 "), None),
        CronAddScheduleKind::At
    );
    assert_eq!(
        schedule_kind_from_api("unknown", " * * * * * ", Some(" "), None),
        CronAddScheduleKind::Cron
    );
    assert_eq!(schedule_kind_from_api("", " ", None, None), CronAddScheduleKind::Cron);
}

#[test]
fn job_type_and_model_provider_infer_legacy_values() {
    let mut value = job("1");
    value.job_type = " AGENT ".to_string();
    assert_eq!(job_type_from_api(&value), CronAddJobType::Agent);

    value.job_type = "legacy".to_string();
    value.command = " ".to_string();
    value.prompt = Some(" do work ".to_string());
    assert_eq!(job_type_from_api(&value), CronAddJobType::Agent);

    value.command = "echo hi".to_string();
    assert_eq!(job_type_from_api(&value), CronAddJobType::Shell);
    value.command.clear();
    value.prompt = Some(" ".to_string());
    assert_eq!(job_type_from_api(&value), CronAddJobType::Shell);

    assert_eq!(model_provider_from_model("openai/gpt-5"), "openai");
    assert_eq!(model_provider_from_model("/gpt-5"), "");
    assert_eq!(model_provider_from_model("gpt-5"), "");
}
