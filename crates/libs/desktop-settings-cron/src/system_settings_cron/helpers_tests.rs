use super::*;
use crate::state::*;
use iced::widget::text;

pub(super) struct TestApp {
    pub(super) cron_settings: CronSettingsState,
    pub(super) app_theme: Theme,
    project_path: Option<String>, recent_projects: Vec<String>, acp_agents: Vec<String>,
    agents_settings: Agents,
    scheduler_settings: Scheduler,
}
#[derive(Default)] struct Scheduler { save_error: Option<String> }
#[derive(Default)] struct Agents { provider_models: Vec<Provider> }
struct Provider { id: String, models: Vec<Model> }
struct Model { id: String }
fn test_app() -> TestApp { TestApp::default() }
pub(super) fn data(app: &TestApp) -> ViewData<'_> {
    ViewData { cron_settings: &app.cron_settings, desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
    app_theme: &app.app_theme, project_path: app.project_path.as_deref(), recent_projects: &app.recent_projects,
    agents: vec![], providers: app.agents_settings.provider_models.iter().map(|p| ProviderOptions { id: &p.id, models: p.models.iter().map(|m|m.id.as_str()).collect() }).collect(),
    delivery: DeliveryAvailability::default(), scheduler_error: app.scheduler_settings.save_error.as_deref(), last_copied_code_hash: None,
    help_content: |_, body, _, _| text(body).into() }
}
fn keep_element(element: Element<'_,Message>) { std::hint::black_box(element); }
pub(super) fn job(id: &str) -> CronJobDto {
    CronJobDto {
        timezone: None,
        anchor_date: None,
        weekdays: None,
        hour: None,
        minute: None,
        starts_at: None,
        ends_at: None,

        id: id.to_string(),
        name: None,
        job_type: String::new(),
        schedule_kind: "cron".to_string(),
        expression: "0 9 * * *".to_string(),
        at: None,
        every_ms: None,
        command: "echo ok".to_string(),
        prompt: None,
        model: None,
        agent: None,
        acp_agent: None,
        project_path: None,
        wake: false,
        fallbacks: Vec::new(),
        full_access: false,
        task_pool: false,
        delivery_mode: String::new(),
        delivery_channel: None,
        delivery_to: None,
        delivery_best_effort: true,
        delete_after_run: false,
        next_run: "2026-06-12T01:02:03Z".to_string(),
        last_run: None,
        last_status: None,
        last_output: None,
        enabled: true,
    }
}

#[test]
fn small_building_blocks_return_elements() {
    keep_element(field_row("标签", "说明", text("control")));
    keep_element(tab_button(CronSettingsTab::Jobs, CronSettingsTab::Jobs, "任务"));
    keep_element(tab_button(CronSettingsTab::Scheduler, CronSettingsTab::Jobs, "调度器"));
    keep_element(tab_button(CronSettingsTab::Add, CronSettingsTab::Jobs, "添加"));
    keep_element(segment_button("Shell", true, Message::None));
    keep_element(segment_button("Agent", false, Message::None));
    keep_element(small_button("删除", Message::None, true));
    keep_element(small_button("刷新", Message::None, false));
    keep_element(primary_button("保存", Message::None));
    keep_element(secondary_button("取消", Message::None));
    keep_element(text_field("placeholder", "value", |_| Message::None));
    keep_element(pick_field(vec!["a".to_string()], Some("a".to_string()), |_| Message::None));
}
#[test]
fn option_helpers_preserve_selected_values() {
    assert_eq!(
        with_selected_option(vec!["b".to_string(), "a".to_string()], "c"),
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
    assert_eq!(
        with_selected_option(vec!["b".to_string(), "b".to_string()], " "),
        vec!["b".to_string()]
    );
}
#[test]
fn app_backed_option_helpers_include_current_selection() {
    let mut app = test_app();
    app.recent_projects = vec!["/tmp/a".to_string()];
    app.project_path = Some("/tmp/current".into());
    app.acp_agents = vec!["acp-a".to_string()];

    assert!(project_options(&data(&app), "/tmp/selected").contains(&"/tmp/selected".to_string()));
    assert!(project_options(&data(&app), "").contains(&"/tmp/current".to_string()));
    assert!(models_for_provider(&data(&app), "missing").is_empty());
}
#[test]
fn job_helpers_handle_fallback_labels_and_types() {
    let mut shell = job("shell");
    assert_eq!(job_title(&shell), "shell");
    assert!(!job_is_agent(&shell));

    shell.name = Some("  ".to_string());
    assert_eq!(job_title(&shell), "shell");

    let mut agent = job("agent");
    agent.job_type = "agent".to_string();
    agent.prompt = Some("Summarize".to_string());
    agent.command.clear();
    assert!(job_is_agent(&agent));

    let mut inferred_agent = job("inferred");
    inferred_agent.prompt = Some("Do work".to_string());
    inferred_agent.command.clear();
    assert!(job_is_agent(&inferred_agent));
}
#[test]
fn date_and_preview_helpers_cover_valid_invalid_and_empty_values() {
    assert_ne!(format_cron_datetime("2026-06-12T01:02:03Z"), "2026-06-12T01:02:03Z");
    assert_eq!(format_cron_datetime("not-a-date"), "not-a-date");
    assert_eq!(format_optional_cron_datetime(None), "无");
    assert_eq!(format_optional_cron_datetime(Some("not-a-date")), "not-a-date");
    assert_eq!(preview_lines("", 3), "");
    assert_eq!(preview_lines("a\nb\nc", 3), "a\nb\nc");
    assert_eq!(preview_lines("a\nb\nc\nd", 3), "a\nb\nc\n...");
}
#[test]
fn job_project_label_prefers_job_project_then_current_project_then_default() {
    let mut app = test_app();
    let mut item = job("job-1");
    assert_eq!(job_project_label(&data(&app), &item), "默认工作区");

    app.project_path = Some("/workspace/current".into());
    assert_eq!(job_project_label(&data(&app), &item), "当前项目: /workspace/current");

    item.project_path = Some("/workspace/job".into());
    assert_eq!(job_project_label(&data(&app), &item), "/workspace/job");
}
#[test]
fn with_selected_option_skips_already_present_selection() {
    let result = with_selected_option(vec!["a".to_string(), "b".to_string()], "a");
    assert_eq!(result, vec!["a", "b"]);
}
#[test]
fn models_for_provider_returns_models_when_provider_found() {
    let mut app = test_app();
    app.agents_settings.provider_models = vec![Provider {
        id: "openai".to_string(),
        models: vec![Model {
            id: "gpt-5".to_string(),
        }],
    }];
    assert_eq!(models_for_provider(&data(&app), "openai"), vec!["gpt-5".to_string()]);
    assert!(models_for_provider(&data(&app), "missing").is_empty());
}
#[test]
fn job_title_returns_name_when_valid_or_id_as_fallback() {
    let mut job = job("id-fallback");
    job.name = Some("My Cron Job".to_string());
    assert_eq!(job_title(&job), "My Cron Job");
}
#[test]
fn job_is_agent_with_both_prompt_and_command_not_agent() {
    let mut job = job("both");
    job.prompt = Some("Do the thing".to_string());
    job.command = "npm run build".to_string();
    assert!(!job_is_agent(&job));
}
#[test]
fn preview_lines_with_zero_max_returns_ellipsis() {
    assert_eq!(preview_lines("a\nb", 0), "...");
}
#[test]
fn job_project_label_falls_through_empty_job_project() {
    let mut app = test_app();
    app.project_path = Some("/workspace/current".into());
    let mut item = job("job-1");
    item.project_path = Some("  ".to_string());
    assert_eq!(job_project_label(&data(&app), &item), "当前项目: /workspace/current");
}
#[test]
fn cron_save_error_banner_follows_tabs() {
    let mut app = test_app();

    assert_eq!(active_save_error(&data(&app)), None);
    assert_eq!(view(&data(&app), text("").into(), std::convert::identity).as_widget().children().len(), 3);

    app.cron_settings.save_error = Some("cron save failed".to_string());
    app.scheduler_settings.save_error = Some("scheduler save failed".to_string());
    assert_eq!(active_save_error(&data(&app)), Some("cron save failed"));
    assert_eq!(view(&data(&app), text("").into(), std::convert::identity).as_widget().children().len(), 4);

    app.cron_settings.save_error = None;
    assert_eq!(active_save_error(&data(&app)), Some("scheduler save failed"));
    assert_eq!(view(&data(&app), text("").into(), std::convert::identity).as_widget().children().len(), 4);
}

impl Default for TestApp { fn default() -> Self { Self { cron_settings: Default::default(), app_theme: iced::Theme::Light, project_path: Default::default(), recent_projects: Default::default(), acp_agents: Default::default(), agents_settings: Default::default(), scheduler_settings: Default::default() } } }
