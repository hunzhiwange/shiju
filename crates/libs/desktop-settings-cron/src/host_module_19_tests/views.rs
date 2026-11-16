//! 验证定时任务设置的表单、调度预览、运行记录与模态层状态。
//! 作业 fixture 组合代理与命令类型、无效日期和缺失输出以覆盖显示回退。

use super::*;
use crate::state::{CronAddJobType, CronAddScheduleKind, CronSettingsTab};

use gateway_cron::CronRunDto;
use iced::Element;
use iced::widget::text;

// 设置页 fixture 从统一默认应用出发，各用例只覆写当前页面关心的字段。
fn test_app() -> Fixture {
    fixture()
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
fn add_tab_covers_shell_every_and_agent_delivery_with_provider_model_options() {
    let mut app = test_app();
    app.providers = vec![("openai".into(), vec!["gpt-5".into(), "gpt-4.1".into()])];
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
