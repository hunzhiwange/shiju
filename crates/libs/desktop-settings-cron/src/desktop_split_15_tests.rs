//! Domain-owned Cron view tests. Host retains projection and cross-settings overlays.
use crate::message::Message;
use crate::state::{CronAddJobType, CronAddScheduleKind, CronSettingsState, CronSettingsTab};
use crate::system_settings_cron as domain;
use domain::job_is_agent;
use gateway_cron::CronJobDto;
use iced::widget::text;
use iced::{Element, Length, Theme};
struct Fixture {
    cron_settings: CronSettingsState,
    desktop_locale: config_ui::DesktopLocale,
    app_theme: Theme,
    recent_projects: Vec<String>,
    project_path: Option<String>,
}
fn test_app() -> Fixture {
    Fixture {
        cron_settings: Default::default(),
        desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
        app_theme: Theme::Light,
        recent_projects: Vec::new(),
        project_path: None,
    }
}
fn data(app: &Fixture) -> domain::ViewData<'_> {
    domain::ViewData {
        cron_settings: &app.cron_settings,
        desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme,
        project_path: app.project_path.as_deref(),
        recent_projects: &app.recent_projects,
        agents: vec!["main"],
        providers: Vec::new(),
        delivery: Default::default(),
        scheduler_error: None,
        last_copied_code_hash: None,
        help_content: |title, body, _, _| iced::widget::column![text(title), text(body)].into(),
    }
}
fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}
fn job_row<'a>(app: &'a Fixture, job: &'a CronJobDto) -> Element<'a, Message> {
    domain::list::job_row(&data(app), job)
}
fn jobs_tab(app: &Fixture) -> Element<'_, Message> {
    domain::list::jobs_tab(&data(app))
}
fn edit_form(app: &Fixture) -> Element<'_, Message> {
    domain::edit_form(&data(app))
}
fn add_tab(app: &Fixture) -> Element<'_, Message> {
    domain::add_tab(&data(app))
}
fn with_edit_modal<'a>(app: &'a Fixture, base: Element<'a, Message>) -> Element<'a, Message> {
    domain::with_edit_modal(&data(app), base, |m| m)
}
pub(super) use crate::job_test_fixture::job;

#[test]
fn rows_and_forms_build_shell_agent_and_editing_states() {
    let mut app = test_app();
    let mut shell = job("job-1");
    shell.last_output = Some("line1\nline2\nline3\nline4".to_string());
    keep_element(job_row(&app, &shell));

    app.cron_settings.selected_job_ids.push("job-1".to_string());
    app.cron_settings.expanded_job_ids.push("job-1".to_string());
    app.cron_settings.editing_job_id = Some("job-1".to_string());
    app.cron_settings.edit_draft.job_type = CronAddJobType::Agent;
    app.cron_settings.edit_draft.schedule_kind = CronAddScheduleKind::At;
    app.cron_settings.edit_draft.delivery_enabled = true;
    keep_element(job_row(&app, &shell));

    app.cron_settings.edit_draft.schedule_kind = CronAddScheduleKind::Every;
    keep_element(edit_form(&app));

    app.cron_settings.edit_draft.job_type = CronAddJobType::Shell;
    app.cron_settings.edit_draft.schedule_kind = CronAddScheduleKind::Cron;
    app.cron_settings.edit_draft.delivery_enabled = false;
    keep_element(edit_form(&app));
}

#[test]
fn job_row_builds_disabled_announced_agent_with_complete_execution_metadata() {
    let mut app = test_app();
    let mut item = job("agent-job");
    item.name = Some("Nightly agent".to_string());
    item.job_type = "AGENT".to_string();
    item.command.clear();
    item.prompt = Some("Summarize the workspace".to_string());
    item.agent = Some("reviewer".to_string());
    item.acp_agent = Some("acp-reviewer".to_string());
    item.model = Some("openai/gpt-5".to_string());
    item.project_path = Some("/workspace/nightly".to_string());
    item.fallbacks = vec!["openai/gpt-4.1".to_string()];
    item.full_access = true;
    item.task_pool = true;
    item.wake = true;
    item.delete_after_run = true;
    item.delivery_mode = "announce".to_string();
    item.last_run = Some("2026-06-11T01:02:03Z".to_string());
    item.last_status = Some("success".to_string());
    item.enabled = false;
    app.cron_settings.expanded_job_ids.push(item.id.clone());

    assert!(job_is_agent(&item));
    keep_element(job_row(&app, &item));
}

#[test]
fn add_tab_covers_agent_schedule_and_delivery_variants() {
    let mut app = test_app();
    app.cron_settings.active_tab = CronSettingsTab::Add;
    app.cron_settings.add_draft.job_type = CronAddJobType::Agent;
    app.cron_settings.add_draft.schedule_kind = CronAddScheduleKind::At;
    app.cron_settings.add_draft.model_provider = "openai".to_string();
    app.cron_settings.add_draft.model = "openai/gpt-5".to_string();
    app.cron_settings.add_draft.delivery_enabled = true;
    keep_element(add_tab(&app));

    app.cron_settings.add_draft.schedule_kind = CronAddScheduleKind::Every;
    app.cron_settings.add_draft.model_provider.clear();
    app.cron_settings.add_draft.model = "gpt-5".to_string();
    keep_element(add_tab(&app));
}

#[test]
fn jobs_tab_covers_partial_selection_non_editing_and_job_without_output() {
    let mut app = test_app();
    let mut disabled = job("disabled");
    disabled.enabled = false;
    disabled.last_status = Some("failed".to_string());
    disabled.next_run = "bad-date".to_string();
    disabled.last_run = Some("bad-last-run".to_string());
    disabled.delivery_mode = "none".to_string();
    disabled.last_output = Some("   ".to_string());
    app.cron_settings.jobs = vec![job("job-1"), disabled];
    app.cron_settings.selected_job_ids = vec!["job-1".to_string()];

    keep_element(jobs_tab(&app));
    keep_element(view(&app));
}

#[test]
fn edit_form_covers_agent_delivery_fields_with_blank_provider_and_selected_project_options() {
    let mut app = test_app();
    app.recent_projects = vec!["/workspace/a".to_string(), "/workspace/b".to_string()];
    app.project_path = Some("/workspace/current".to_string());
    app.cron_settings.editing_job_id = Some("job-1".to_string());
    app.cron_settings.edit_draft.job_type = CronAddJobType::Agent;
    app.cron_settings.edit_draft.schedule_kind = CronAddScheduleKind::Cron;
    app.cron_settings.edit_draft.agent = "main".to_string();
    app.cron_settings.edit_draft.acp_agent = "codex".to_string();
    app.cron_settings.edit_draft.model_provider.clear();
    app.cron_settings.edit_draft.model = "custom-model".to_string();
    app.cron_settings.edit_draft.fallbacks = "fallback-a\nfallback-b".to_string();
    app.cron_settings.edit_draft.project_path = "/workspace/selected".to_string();
    app.cron_settings.edit_draft.full_access = true;
    app.cron_settings.edit_draft.task_pool = true;
    app.cron_settings.edit_draft.wake = true;
    app.cron_settings.edit_draft.delete_after_run = true;
    app.cron_settings.edit_draft.delivery_enabled = true;
    app.cron_settings.edit_draft.delivery_channel = "slack".to_string();
    app.cron_settings.edit_draft.delivery_to = "#ops".to_string();
    app.cron_settings.edit_draft.delivery_best_effort = false;

    keep_element(edit_form(&app));
}

#[test]
fn cron_modal_add_and_edit_wrap_base() {
    let mut app = test_app();
    for editing in [false, true] {
        app.cron_settings.show_add_modal = !editing;
        app.cron_settings.editing_job_id = editing.then(|| "job".into());
        keep_element(with_edit_modal(&app, text("任务列表").into()));
    }
}
mod history;

mod modal;
