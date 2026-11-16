use crate::state::*;
use crate::update::cron::{self, schedule_request};
use crate::update::messages::{Message, SettingsMessage};
use gateway_cron::{CronJobDto, CronRunDto};
use std::sync::Arc;
use iced::widget::text_editor;
fn app() -> CronSettingsState { CronSettingsState::default() }
fn update(state: &mut CronSettingsState, message: SettingsMessage) -> iced::Task<Message> {
    let config = desktop_config::ConfigAccess::new(gateway_endpoint::GatewayEndpoint::new("127.0.0.1", 9), None);
    cron::update(&mut cron::Inputs {
        cron_settings: state,
        desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
        delivery: Default::default(),
        client: Err("fixture gateway unavailable".into()),
        notices: &mut Vec::new(),
        config: &config,
    }, message)
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
fn cron_list_jobs() -> Vec<CronJobDto> {
    let mut paused = job("paused");
    paused.enabled = false;
    vec![job("enabled-2"), paused, job("enabled-1")]
}


mod list;

mod editor;

mod schedule;
