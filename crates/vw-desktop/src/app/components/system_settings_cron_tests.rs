//! 验证定时任务设置的表单、调度预览、运行记录与模态层状态。
//! 作业 fixture 组合代理与命令类型、无效日期和缺失输出以覆盖显示回退。

use super::*;
use crate::app::state::{CronAddJobType, CronAddScheduleKind, CronSettingsTab};
use crate::app::{App, Message};
use iced::Element;
use iced::widget::text;
use vw_gateway_client::CronRunDto;

// 设置页 fixture 从统一默认应用出发，各用例只覆写当前页面关心的字段。
fn test_app() -> App {
    App::new().0
}

// 保留元素即可完成无窗口构建检查，避免把布局细节误当成业务契约。
fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

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

pub(super) fn run(status: &str) -> CronRunDto {
    CronRunDto {
        id: 1,
        job_id: "job-1".to_string(),
        started_at: "2026-06-12T01:02:03Z".to_string(),
        finished_at: "2026-06-12T01:02:04Z".to_string(),
        status: status.to_string(),
        output: Some("ok".to_string()),
        duration_ms: Some(1000),
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

    assert!(project_options(&app, "/tmp/selected").contains(&"/tmp/selected".to_string()));
    assert!(project_options(&app, "").contains(&"/tmp/current".to_string()));
    assert!(models_for_provider(&app, "missing").is_empty());
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
    assert_eq!(job_project_label(&app, &item), "默认工作区");

    app.project_path = Some("/workspace/current".into());
    assert_eq!(job_project_label(&app, &item), "当前项目: /workspace/current");

    item.project_path = Some("/workspace/job".into());
    assert_eq!(job_project_label(&app, &item), "/workspace/job");
}

#[test]
fn with_selected_option_skips_already_present_selection() {
    let result = with_selected_option(vec!["a".to_string(), "b".to_string()], "a");
    assert_eq!(result, vec!["a", "b"]);
}

#[test]
fn models_for_provider_returns_models_when_provider_found() {
    let mut app = test_app();
    app.agents_settings.provider_models = vec![crate::app::state::ProviderModelsSummary {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        models: vec![crate::app::state::ModelSummary {
            id: "gpt-5".to_string(),
            name: "GPT-5".to_string(),
            enabled: true,
            toolcall: true,
            attachment: false,
            context_limit: 100000,
            detail: serde_json::Value::Null,
        }],
    }];
    assert_eq!(models_for_provider(&app, "openai"), vec!["gpt-5".to_string()]);
    assert!(models_for_provider(&app, "missing").is_empty());
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
    assert_eq!(job_project_label(&app, &item), "当前项目: /workspace/current");
}

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
fn tabs_and_view_cover_loading_empty_jobs_config_and_add_forms() {
    let mut app = test_app();
    app.cron_settings.jobs_loading = true;
    keep_element(jobs_tab(&app));
    keep_element(view(&app));

    app.cron_settings.jobs_loading = false;
    keep_element(jobs_tab(&app));

    app.cron_settings.jobs = vec![job("job-1"), job("job-2")];
    app.cron_settings.selected_job_ids = vec!["job-1".to_string(), "job-2".to_string()];
    keep_element(jobs_tab(&app));

    app.cron_settings.active_tab = CronSettingsTab::Config;
    app.cron_settings.action_status = Some("已保存".to_string());
    app.cron_settings.save_error = Some("保存失败".to_string());
    keep_element(config_tab(&app));
    keep_element(view(&app));

    app.scheduler_settings.enabled = false;
    app.scheduler_settings.max_tasks = 64;
    app.scheduler_settings.max_concurrent = 4;
    keep_element(config_tab(&app));

    app.cron_settings.active_tab = CronSettingsTab::Scheduler;
    keep_element(view(&app));

    app.cron_settings.active_tab = CronSettingsTab::Add;
    app.cron_settings.add_draft.job_type = CronAddJobType::Shell;
    app.cron_settings.add_draft.schedule_kind = CronAddScheduleKind::Cron;
    keep_element(add_tab(&app));
    keep_element(view(&app));
}

#[test]
fn cron_save_error_banner_follows_tabs() {
    let mut app = test_app();

    assert_eq!(active_save_error(&app), None);
    assert_eq!(view(&app).as_widget().children().len(), 3);

    app.cron_settings.save_error = Some("cron save failed".to_string());
    app.scheduler_settings.save_error = Some("scheduler save failed".to_string());
    assert_eq!(active_save_error(&app), Some("cron save failed"));
    assert_eq!(view(&app).as_widget().children().len(), 4);

    app.cron_settings.save_error = None;
    assert_eq!(active_save_error(&app), Some("scheduler save failed"));
    assert_eq!(view(&app).as_widget().children().len(), 4);
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
fn overlays_cover_no_modal_runs_modal_states_and_help() {
    let mut app = test_app();
    keep_element(view_overlays(&app, text("dialog").into()));

    app.cron_settings.runs_modal_job_id = Some("job-1".to_string());
    app.cron_settings.runs_modal_loading = true;
    keep_element(view_overlays(&app, text("dialog").into()));

    app.cron_settings.runs_modal_loading = false;
    app.cron_settings.runs_modal_error = Some("加载失败".to_string());
    keep_element(view_overlays(&app, text("dialog").into()));

    app.cron_settings.runs_modal_error = None;
    keep_element(view_overlays(&app, text("dialog").into()));

    app.cron_settings.jobs = vec![job("job-1")];
    app.cron_settings.runs_modal = vec![run("success")];
    keep_element(view_overlays(&app, text("dialog").into()));

    app.cron_settings.show_help_modal = true;
    keep_element(view_overlays(&app, text("dialog").into()));
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
    app.acp_agents = vec!["codex".to_string(), "claude".to_string()];
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
fn add_tab_covers_shell_every_and_agent_delivery_with_provider_model_options() {
    let mut app = test_app();
    app.agents_settings.provider_models = vec![crate::app::state::ProviderModelsSummary {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
        models: vec![
            crate::app::state::ModelSummary {
                id: "gpt-5".to_string(),
                name: "GPT-5".to_string(),
                enabled: true,
                toolcall: true,
                attachment: true,
                context_limit: 100000,
                detail: serde_json::Value::Null,
            },
            crate::app::state::ModelSummary {
                id: "gpt-4.1".to_string(),
                name: "GPT-4.1".to_string(),
                enabled: true,
                toolcall: true,
                attachment: false,
                context_limit: 64000,
                detail: serde_json::Value::Null,
            },
        ],
    }];
    app.cron_settings.active_tab = CronSettingsTab::Add;
    app.cron_settings.add_draft.job_type = CronAddJobType::Shell;
    app.cron_settings.add_draft.schedule_kind = CronAddScheduleKind::Every;
    app.cron_settings.add_draft.every_ms = "60000".to_string();
    keep_element(add_tab(&app));

    app.cron_settings.add_draft.job_type = CronAddJobType::Agent;
    app.cron_settings.add_draft.schedule_kind = CronAddScheduleKind::Cron;
    app.cron_settings.add_draft.model_provider = "openai".to_string();
    app.cron_settings.add_draft.model = "openai/gpt-4.1".to_string();
    app.cron_settings.add_draft.delivery_enabled = true;
    app.cron_settings.add_draft.delivery_channel = "email".to_string();
    app.cron_settings.add_draft.delivery_to = "ops@example.com".to_string();
    app.cron_settings.add_draft.delivery_best_effort = false;
    keep_element(add_tab(&app));
}

#[test]
fn runs_modal_falls_back_to_job_id_and_wraps_help_after_runs_modal() {
    let mut app = test_app();
    app.cron_settings.runs_modal_job_id = Some("missing-job".to_string());
    app.cron_settings.runs_modal_loading = false;
    app.cron_settings.runs_modal_error = None;
    app.cron_settings.runs_modal = Vec::new();

    keep_element(view_overlays(&app, text("dialog").into()));

    app.cron_settings.show_help_modal = true;
    keep_element(view_overlays(&app, text("dialog").into()));
}

#[cfg(not(target_arch = "wasm32"))]
mod modal_render {
    use super::*;
    use crate::app::message::SettingsMessage;
    use crate::app::state::{CronScheduleDraft, CronScheduleRule as R};
    use iced::advanced::{
        clipboard,
        renderer::{Headless, Style},
    };
    use iced::{Event, Font, Rectangle, Renderer, Size, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};

    #[derive(Default)]
    struct Geometry {
        labels: Vec<(String, Rectangle)>,
        scrolls: Vec<Rectangle>,
        inputs: Vec<Rectangle>,
    }
    impl iced::advanced::widget::Operation for Geometry {
        fn traverse(
            &mut self,
            children: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation),
        ) {
            children(self);
        }
        fn text(&mut self, _: Option<&iced::advanced::widget::Id>, bounds: Rectangle, value: &str) {
            self.labels.push((value.into(), bounds));
        }
        fn text_input(
            &mut self,
            _: Option<&iced::advanced::widget::Id>,
            bounds: Rectangle,
            _: &mut dyn iced::advanced::widget::operation::TextInput,
        ) {
            self.inputs.push(bounds);
        }
        fn scrollable(
            &mut self,
            _: Option<&iced::advanced::widget::Id>,
            bounds: Rectangle,
            _: Rectangle,
            _: iced::Vector,
            _: &mut dyn iced::advanced::widget::operation::Scrollable,
        ) {
            self.scrolls.push(bounds);
        }
    }

    fn isolated(test: &str) -> bool {
        const CHILD: &str = "SHIJU_CRON_MODAL_RENDER_CHILD";
        if std::env::var_os(CHILD).is_some() {
            return true;
        }
        let home = tempfile::tempdir().unwrap();
        let config = vw_config_types::ui::GatewayClientSystemSettingsConfig {
            host: "[".into(),
            servers: Vec::new(),
            ..Default::default()
        };
        std::fs::write(
            home.path().join("gateway-client-bootstrap.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &format!("{}::{test}", module_path!().split_once("::").unwrap().1)])
            .env(CHILD, "1")
            .env("HOME", home.path())
            .env("VIBEWINDOW_CONFIG_DIR", home.path())
            .env("VIBEWINDOW_WORKSPACE", home.path())
            .current_dir(home.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        false
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn wb_schedule_modal_renders_all_rules_and_fixed_actions() {
        if !isolated("wb_schedule_modal_renders_all_rules_and_fixed_actions") {
            return;
        }
        {
            let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
            for font in crate::fonts::load_all() {
                fonts.load_font(font);
            }
        }
        let mut renderer =
            Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
                .await
                .unwrap();
        let mut app = test_app();
        app.desktop_locale = vw_config_types::ui::DesktopLocale::SimplifiedChinese;
        let time = "09:30:00".parse().unwrap();
        let date = "2099-01-01".parse().unwrap();
        let rules = [
            R::At { date, time },
            R::Every { amount: 2, unit: crate::app::state::IntervalUnit::Hours },
            R::Daily { time },
            R::Weekly { weekdays: vec![1, 3], time },
            R::Biweekly { anchor: date, weekdays: vec![1, 5], time },
            R::Monthly { days: vec![1, 15, 31], time },
            R::Yearly { month: 2, day: 29, time },
            R::Advanced("*/5 * * * *".into()),
        ];
        let directory =
            std::env::var_os("SHIJU_CRON_MODAL_SCREENSHOTS").map(std::path::PathBuf::from);
        for (index, rule) in rules.iter().enumerate() {
            for editing in [false, true] {
                for (theme_name, theme) in [("light", Theme::Light), ("dark", Theme::Dark)] {
                    for (size_name, size) in
                        [("narrow", Size::new(380, 680)), ("wide", Size::new(1100, 900))]
                    {
                        app.app_theme = theme.clone();
                        app.cron_settings.show_add_modal = !editing;
                        app.cron_settings.editing_job_id = editing.then(|| "render".into());
                        let draft = if editing {
                            &mut app.cron_settings.edit_draft
                        } else {
                            &mut app.cron_settings.add_draft
                        };
                        draft.schedule_draft = Some(CronScheduleDraft {
                            rule: rule.clone(),
                            timezone: "Asia/Shanghai".into(),
                            start_date: Some(Some(date)),
                            end_date: Some(Some("2099-12-31".parse().unwrap())),
                            ..Default::default()
                        });
                        draft.prompt = "检查项目并生成中文报告\n".repeat(40);
                        draft.prompt_editor =
                            iced::widget::text_editor::Content::with_text(&draft.prompt);
                        draft.preview = Some(Ok(vw_gateway_client::CronPreviewResponse {
                            normalized: Default::default(),
                            timezone: "Asia/Shanghai".into(),
                            candidates: (2..=if index == 0 { 2 } else { 6 })
                                .map(|day| format!("2099-01-{day:02}T01:30:00Z"))
                                .collect(),
                        }));
                        let mut ui = UserInterface::build(
                            view_overlays(
                                &app,
                                iced::widget::container(text("任务列表"))
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                                    .into(),
                            ),
                            Size::new(size.width as f32, size.height as f32),
                            Cache::new(),
                            &mut renderer,
                        );
                        let mut redraw_messages = Vec::new();
                        ui.update(
                            &[Event::Window(iced::window::Event::RedrawRequested(
                                iced::time::Instant::now(),
                            ))],
                            mouse::Cursor::Unavailable,
                            &mut renderer,
                            &mut clipboard::Null,
                            &mut redraw_messages,
                        );
                        let mut geometry = Geometry::default();
                        ui.operate(&renderer, &mut geometry);
                        let save = geometry
                            .labels
                            .iter()
                            .find(|(label, _)| label == "保存")
                            .expect("fixed save button")
                            .1;
                        assert!(
                            save.y >= 0.0 && save.y + save.height <= size.height as f32,
                            "save clipped: {save:?}"
                        );
                        assert!(
                            geometry
                                .labels
                                .iter()
                                .any(|(label, _)| label.contains("时区：Asia/Shanghai"))
                        );
                        assert!(!geometry.scrolls.is_empty());
                        assert_eq!(
                            geometry.labels.iter().any(|(label, _)| label == "有效期"),
                            index != 0
                        );
                        if index == 3 || index == 4 {
                            assert!(
                                geometry
                                    .labels
                                    .iter()
                                    .any(|(label, _)| label == "星期（至少选择一天）")
                            );
                        }
                        if index == 5 {
                            assert!(
                                geometry
                                    .labels
                                    .iter()
                                    .any(|(label, _)| label == "日期（至少选择一天）")
                            );
                        }
                        for input in &geometry.inputs {
                            assert!(
                                input.x >= 0.0 && input.x + input.width <= size.width as f32,
                                "input clipped: {input:?}"
                            );
                        }

                        ui.draw(
                            &mut renderer,
                            &theme,
                            &Style { text_color: theme.palette().text },
                            mouse::Cursor::Unavailable,
                        );
                        let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
                        assert_eq!(pixels.len(), (size.width * size.height * 4) as usize);
                        if let Some(directory) = &directory {
                            std::fs::create_dir_all(directory).unwrap();
                            ::image::save_buffer(
                                directory.join(format!(
                                    "rule-{index}-{}-{theme_name}-{size_name}.png",
                                    if editing { "edit" } else { "add" }
                                )),
                                &pixels,
                                size.width,
                                size.height,
                                ::image::ColorType::Rgba8,
                            )
                            .unwrap();
                        }
                        let mut ignored = Vec::new();
                        ui.update(
                            &[Event::Mouse(mouse::Event::WheelScrolled {
                                delta: mouse::ScrollDelta::Lines { x: 0.0, y: -80.0 },
                            })],
                            mouse::Cursor::Available(geometry.scrolls[0].center()),
                            &mut renderer,
                            &mut clipboard::Null,
                            &mut ignored,
                        );
                        ui.update(
                            &[Event::Window(iced::window::Event::RedrawRequested(
                                iced::time::Instant::now(),
                            ))],
                            mouse::Cursor::Unavailable,
                            &mut renderer,
                            &mut clipboard::Null,
                            &mut redraw_messages,
                        );
                        let mut scrolled = Geometry::default();
                        ui.operate(&renderer, &mut scrolled);
                        assert_eq!(
                            save,
                            scrolled.labels.iter().find(|(label, _)| label == "保存").unwrap().1
                        );
                        ui.draw(
                            &mut renderer,
                            &theme,
                            &Style { text_color: theme.palette().text },
                            mouse::Cursor::Unavailable,
                        );
                        if index == 0 {
                            if let Some(directory) = &directory {
                                let pixels =
                                    renderer.screenshot(size, 1.0, theme.palette().background);
                                ::image::save_buffer(
                                    directory.join(format!(
                                        "scrolled-{}-{theme_name}-{size_name}.png",
                                        if editing { "edit" } else { "add" }
                                    )),
                                    &pixels,
                                    size.width,
                                    size.height,
                                    ::image::ColorType::Rgba8,
                                )
                                .unwrap();
                            }
                        }
                        let mut messages = Vec::new();
                        let point = save.center();
                        ui.update(
                            &[
                                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                            ],
                            mouse::Cursor::Available(point),
                            &mut renderer,
                            &mut clipboard::Null,
                            &mut messages,
                        );
                        assert!(messages.iter().any(|m| matches!(
                            m,
                            Message::Settings(
                                SettingsMessage::CronAddSubmit | SettingsMessage::CronJobEditSave
                            )
                        )));
                    }
                }
            }
        }
    }
    #[tokio::test(flavor = "multi_thread")]
    async fn wb_schedule_feedback_renders_permissions_and_delivery() {
        if !isolated("wb_schedule_feedback_renders_permissions_and_delivery") {
            return;
        }
        {
            let mut fonts = iced::advanced::graphics::text::font_system().write().unwrap();
            for font in crate::fonts::load_all() {
                fonts.load_font(font);
            }
        }
        let mut renderer =
            Renderer::new(Font::with_name("Noto Sans CJK SC"), 14.0.into(), Some("tiny-skia"))
                .await
                .unwrap();
        let mut app = test_app();
        app.desktop_locale = vw_config_types::ui::DesktopLocale::SimplifiedChinese;
        app.cron_settings.show_add_modal = true;
        let draft = &mut app.cron_settings.add_draft;
        draft.schedule_draft = Some(Default::default());
        draft.job_type = CronAddJobType::Agent;
        draft.delivery_enabled = true;
        draft.delivery_channel = "wecom".into();
        draft.delivery_to = "旧目标".into();
        for confirm in [false, true] {
            app.cron_settings.add_draft.full_access_confirmation = confirm;
            for (name, theme) in [("light", Theme::Light), ("dark", Theme::Dark)] {
                for size in [Size::new(380, 680), Size::new(1100, 900)] {
                    app.app_theme = theme.clone();
                    let mut ui = UserInterface::build(
                        view_overlays(
                            &app,
                            iced::widget::container(text("任务列表"))
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .into(),
                        ),
                        Size::new(size.width as f32, size.height as f32),
                        Cache::new(),
                        &mut renderer,
                    );
                    let mut messages = Vec::new();
                    ui.update(
                        &[Event::Window(iced::window::Event::RedrawRequested(
                            iced::time::Instant::now(),
                        ))],
                        mouse::Cursor::Unavailable,
                        &mut renderer,
                        &mut clipboard::Null,
                        &mut messages,
                    );
                    let mut geometry = Geometry::default();
                    ui.operate(&renderer, &mut geometry);
                    if confirm {
                        let label = geometry
                            .labels
                            .iter()
                            .find(|(label, _)| label == "确认授权")
                            .expect("confirmation button");
                        assert!(label.1.x >= 0.0 && label.1.x + label.1.width <= size.width as f32);
                        assert!(
                            label.1.y >= 0.0 && label.1.y + label.1.height <= size.height as f32
                        );
                        assert!(
                            geometry
                                .labels
                                .iter()
                                .any(|(label, _)| label.contains("仅为当前 Agent"))
                        );
                    } else {
                        assert!(
                            geometry.labels.iter().any(|(label, _)| label.contains("默认受限："))
                        );
                        assert!(
                            geometry.labels.iter().any(|(label, _)| label.contains("ACP 未获授权"))
                        );
                        assert!(
                            geometry
                                .labels
                                .iter()
                                .any(|(label, _)| label.contains("投递通道 wecom 未配置"))
                        );
                        assert!(
                            geometry
                                .labels
                                .iter()
                                .any(|(label, _)| label.contains("保存任务不代表已送达"))
                        );
                        let scroll = geometry.scrolls[0];
                        ui.update(
                            &[Event::Mouse(mouse::Event::WheelScrolled {
                                delta: mouse::ScrollDelta::Lines { x: 0.0, y: -100.0 },
                            })],
                            mouse::Cursor::Available(scroll.center()),
                            &mut renderer,
                            &mut clipboard::Null,
                            &mut messages,
                        );
                    }
                    ui.draw(
                        &mut renderer,
                        &theme,
                        &Style { text_color: theme.palette().text },
                        mouse::Cursor::Unavailable,
                    );
                    let pixels = renderer.screenshot(size, 1.0, theme.palette().background);
                    assert_eq!(pixels.len(), (size.width * size.height * 4) as usize);
                    let colors: std::collections::HashSet<_> = pixels.chunks_exact(4).collect();
                    assert!(colors.len() > 100, "modal must paint more than its backdrop");
                    if let Some(dir) = std::env::var_os("SHIJU_CRON_MODAL_SCREENSHOTS") {
                        let dir = std::path::PathBuf::from(dir);
                        std::fs::create_dir_all(&dir).unwrap();
                        ::image::save_buffer(
                            dir.join(format!("feedback-{confirm}-{name}-{}.png", size.width)),
                            &pixels,
                            size.width,
                            size.height,
                            ::image::ColorType::Rgba8,
                        )
                        .unwrap();
                    }
                }
            }
        }
    }
}

#[test]
fn wb_schedule_modal_expired_card_ignores_retained_next_run() {
    let mut job = job("ended");
    job.ends_at = Some("2026-01-02T00:00:00Z".into());
    job.next_run = "2099-01-01T00:00:00Z".into();
    let now = "2026-01-02T00:00:00Z".parse().unwrap();
    assert!(list::job_has_ended(&job, now));
    job.ends_at = Some("2026-01-03T00:00:00Z".into());
    assert!(!list::job_has_ended(&job, now));
}

#[test]
fn cron_history_view_covers_loading_error_and_empty() {
    let mut app = test_app();
    assert_eq!(history::empty_label(&app.cron_settings.history), Some("此范围内暂无运行记录"));
    app.cron_settings.history.loading = true;
    assert_eq!(history::empty_label(&app.cron_settings.history), Some("正在加载运行记录…"));
    let _ = history::view(&app);
    app.cron_settings.history.loading = false;
    app.cron_settings.history.error = Some("连接失败".into());
    assert_eq!(history::empty_label(&app.cron_settings.history), None);
    let _ = history::view(&app);
    app.cron_settings.history.error = None;
    app.cron_settings.history.search = "不存在".into();
    assert_eq!(history::empty_label(&app.cron_settings.history), Some("此范围内无匹配记录"));
    let _ = history::view(&app);
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
