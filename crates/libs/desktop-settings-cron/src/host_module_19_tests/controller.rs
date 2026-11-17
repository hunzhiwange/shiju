use super::*;
fn app() -> Fixture {
    fixture()
}
fn history_app() -> Fixture {
    let mut app = fixture();

    select_tab(&mut app, CronSettingsTab::History);
    app
}
fn validate_delivery(app: &Fixture, draft: &CronJobDraft) -> Result<(), String> {
    crate::delivery::validate(draft, app.delivery)
}
mod cron_history {
    use super::*;
    pub use crate::message::HistoryMessage as CronHistoryMessage;
    pub fn update(
        app: &mut Fixture,
        event: CronHistoryMessage,
    ) -> iced::Task<crate::update::messages::Message> {
        super::update(app, SettingsMessage::CronHistory(event))
    }
}
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

fn history_response() -> gateway_cron::CronHistoryResponse {
    gateway_cron::CronHistoryResponse {
        runs: vec![gateway_cron::CronHistoryRunDto {
            id: Some(1),
            execution_id: None,
            job_id: "a".into(),
            name: Some("备份".into()),
            started_at: "start".into(),
            finished_at: Some("end".into()),
            duration_ms: Some(1),
            status: "ok".into(),
            output: None,
        }],
        finished_limit: 200,
        truncated: false,
    }
}

fn history_loaded(
    app: &mut Fixture,
    generation: u64,
    result: Result<(gateway_cron::CronHistoryResponse, Vec<CronJobDto>), String>,
) {
    let _ =
        cron_history::update(app, cron_history::CronHistoryMessage::Loaded { generation, result });
}

#[test]
fn cron_history_refresh_error_preserves_rows() {
    use cron_history::{self, CronHistoryMessage};
    let mut app = history_app();
    let generation = app.cron_settings.history.generation;
    history_loaded(&mut app, generation, Ok((history_response(), vec![job("a")])));
    app.cron_settings.history.search = "备份".into();
    let _ = cron_history::update(&mut app, CronHistoryMessage::Refresh);
    let generation = app.cron_settings.history.generation;
    let _ = cron_history::update(&mut app, CronHistoryMessage::Refresh);
    assert_eq!(generation, app.cron_settings.history.generation);
    history_loaded(&mut app, generation, Err("offline".into()));
    assert_eq!(app.cron_settings.history.runs.len(), 1);
    assert_eq!(app.cron_settings.history.search, "备份");
    assert_eq!(app.cron_settings.history.error.as_deref(), Some("offline"));
    assert!(!app.cron_settings.history.loading);
    assert_eq!(app.cron_settings.jobs[0].id, "a");
}

#[test]
fn cron_history_stale_response_is_ignored() {
    let mut app = history_app();
    let generation = app.cron_settings.history.generation;
    select_tab(&mut app, CronSettingsTab::Jobs);
    select_tab(&mut app, CronSettingsTab::History);
    let current = app.cron_settings.history.generation;
    assert!(current > generation);
    history_loaded(&mut app, generation, Ok((history_response(), vec![job("old")])));
    assert!(app.cron_settings.history.runs.is_empty());
    assert!(app.cron_settings.history.loading);
    history_loaded(&mut app, current, Ok((history_response(), vec![job("a")])));
    assert_eq!(app.cron_settings.history.runs.len(), 1);
    assert_eq!(app.cron_settings.jobs[0].id, "a");
    // Duplicate completion for the same request cannot overwrite accepted data.
    history_loaded(&mut app, current, Err("late".into()));
    assert!(app.cron_settings.history.error.is_none());
}

#[test]
fn wb_schedule_feedback_unconfigured_channel_is_explicit_error() {
    let mut app = app();
    app.delivery.slack = false;
    let draft = &mut app.cron_settings.add_draft;
    draft.prompt = "report".into();
    draft.delivery_enabled = true;
    draft.delivery_channel = "slack".into();
    draft.delivery_to = "target".into();
    assert!(draft.delivery_best_effort);
    let _ = update(&mut app, SettingsMessage::CronAddSubmit);
    assert!(!app.cron_settings.add_draft.submitting);
    assert!(app.cron_settings.save_error.as_deref().unwrap().contains("未配置"));
}

#[test]
fn cron_wecom_form_requires_credentials_and_explicit_target() {
    let mut app = app();
    let mut draft = crate::state::CronJobDraft::default();
    draft.delivery_enabled = true;
    draft.delivery_channel = "wecom".into();
    draft.delivery_to = "user".into();
    assert!(validate_delivery(&app, &draft).is_err());
    app.delivery.bot = true;
    assert!(validate_delivery(&app, &draft).is_ok());
    draft.delivery_to.clear();
    assert!(validate_delivery(&app, &draft).is_err());
    draft.delivery_enabled = false;
    assert!(validate_delivery(&app, &draft).is_ok());
    draft.delivery_enabled = true;
    draft.delivery_channel = "wecom_webhook".into();
    draft.delivery_to = "default".into();
    // 对话机器人凭据不能替代群通知 Webhook。
    assert!(validate_delivery(&app, &draft).unwrap_err().contains("定时任务通知配置"));
    app.delivery.bot = false;
    app.delivery.webhook = true;
    assert!(validate_delivery(&app, &draft).is_ok());
    draft.delivery_to = "other".into();
    assert!(validate_delivery(&app, &draft).is_err());
}

// The desktop host reconciles history visibility after the domain changes tabs.
fn select_tab(app: &mut Fixture, tab: CronSettingsTab) {
    drop(update(app, SettingsMessage::CronTabSelected(tab)));
    if app.cron_settings.history.sync_context(tab == CronSettingsTab::History, None) {
        drop(update(app, SettingsMessage::CronHistory(crate::message::HistoryMessage::Refresh)));
    }
}
