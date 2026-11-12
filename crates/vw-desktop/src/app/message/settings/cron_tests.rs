// 验证定时任务加载、选择、历史格式化与兼容字段推断。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use crate::app::state::{CronAddJobType, CronAddScheduleKind, CronListFilter, CronSettingsTab};
use std::sync::Arc;
use vw_gateway_client::{CronJobDto, CronRunDto};

fn app() -> App {
    App::new().0
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

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn format_run_history_text_formats_multiple_runs_and_missing_values() {
    let runs = vec![run(1, Some("hello"), Some(12)), run(2, None, None)];

    assert_eq!(
        format_run_history_text(vw_config_types::ui::DesktopLocale::SimplifiedChinese, &runs),
        concat!(
            "#1\n状态: ok\n开始: start-1\n结束: end-1\n",
            "耗时: 12 ms\n输出:\nhello\n\n",
            "#2\n状态: ok\n开始: start-2\n结束: end-2\n",
            "耗时: 无\n输出:\n无输出"
        )
    );
    assert_eq!(
        format_run_history_text(vw_config_types::ui::DesktopLocale::SimplifiedChinese, &[]),
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

#[test]
fn jobs_loaded_reconciles_selection_and_preserves_jobs_on_error() {
    let mut app = app();
    app.cron_settings.jobs_loading = true;
    app.cron_settings.selected_job_ids =
        vec!["a".to_string(), "missing".to_string(), "b".to_string()];
    app.cron_settings.expanded_job_ids = vec!["missing".to_string(), "b".to_string()];
    app.cron_settings.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::CronJobsLoaded(Ok(vec![job("a"), job("b")])));

    assert!(!app.cron_settings.jobs_loading);
    assert_eq!(app.cron_settings.selected_job_ids, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(app.cron_settings.expanded_job_ids, vec!["b".to_string()]);
    assert_eq!(app.cron_settings.jobs.len(), 2);
    assert_eq!(app.cron_settings.save_error, None);

    let _ = update(&mut app, SettingsMessage::CronJobsLoaded(Err("offline".to_string())));

    assert_eq!(app.cron_settings.jobs.len(), 2);
    assert_eq!(app.cron_settings.save_error.as_deref(), Some("offline"));
}

#[test]
fn job_selection_is_idempotent_and_select_all_can_be_cleared() {
    let mut app = app();
    app.cron_settings.jobs = vec![job("a"), job("b")];

    let _ = update(&mut app, SettingsMessage::CronJobSelectionToggled("a".to_string(), true));
    let _ = update(&mut app, SettingsMessage::CronJobSelectionToggled("a".to_string(), true));
    assert_eq!(app.cron_settings.selected_job_ids, vec!["a".to_string()]);

    let _ = update(&mut app, SettingsMessage::CronJobSelectionToggled("a".to_string(), false));
    assert!(app.cron_settings.selected_job_ids.is_empty());

    let _ = update(&mut app, SettingsMessage::CronJobsSelectAllToggled(true));
    assert_eq!(app.cron_settings.selected_job_ids, vec!["a".to_string(), "b".to_string()]);
    let _ = update(&mut app, SettingsMessage::CronJobsSelectAllToggled(false));
    assert!(app.cron_settings.selected_job_ids.is_empty());
}

#[test]
fn job_details_are_collapsed_by_default_and_toggle_idempotently() {
    let mut app = app();
    assert!(app.cron_settings.expanded_job_ids.is_empty());

    let _ = update(&mut app, SettingsMessage::CronJobDetailsToggled("a".to_string(), true));
    let _ = update(&mut app, SettingsMessage::CronJobDetailsToggled("a".to_string(), true));
    assert_eq!(app.cron_settings.expanded_job_ids, vec!["a".to_string()]);

    let _ = update(&mut app, SettingsMessage::CronJobDetailsToggled("a".to_string(), false));
    assert!(app.cron_settings.expanded_job_ids.is_empty());
}

#[test]
fn run_history_ignores_stale_results_and_handles_error_and_close() {
    let mut app = app();
    app.cron_settings.runs_modal_job_id = Some("current".to_string());
    app.cron_settings.runs_modal_loading = true;
    app.cron_settings.runs_modal = vec![run(1, Some("existing"), Some(10))];

    let _ = update(
        &mut app,
        SettingsMessage::CronJobRunsLoaded("stale".to_string(), Ok(vec![run(2, None, None)])),
    );
    assert!(app.cron_settings.runs_modal_loading);
    assert_eq!(app.cron_settings.runs_modal[0].id, 1);

    let _ = update(
        &mut app,
        SettingsMessage::CronJobRunsLoaded(
            "current".to_string(),
            Err("history unavailable".to_string()),
        ),
    );
    assert!(!app.cron_settings.runs_modal_loading);
    assert!(app.cron_settings.runs_modal.is_empty());
    assert_eq!(app.cron_settings.runs_modal_editor.text(), "");
    assert_eq!(app.cron_settings.runs_modal_error.as_deref(), Some("history unavailable"));

    let _ = update(&mut app, SettingsMessage::CronJobRunsClose);
    assert_eq!(app.cron_settings.runs_modal_job_id, None);
    assert_eq!(app.cron_settings.runs_modal_error, None);
}

// 行为契约：使用代表性场景固定该分支的可观察结果。
// 断言聚焦输入、状态与返回任务，不复述内部控制流程。
#[test]
fn run_history_success_populates_read_only_text_state() {
    let mut app = app();
    app.cron_settings.runs_modal_job_id = Some("job-1".to_string());
    app.cron_settings.runs_modal_loading = true;
    app.cron_settings.runs_modal_error = Some("old".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::CronJobRunsLoaded(
            "job-1".to_string(),
            Ok(vec![run(1, Some("hello"), Some(12))]),
        ),
    );

    assert!(!app.cron_settings.runs_modal_loading);
    assert_eq!(app.cron_settings.runs_modal.len(), 1);
    assert!(app.cron_settings.runs_modal_editor.text().contains("输出:\nhello"));
    assert_eq!(app.cron_settings.runs_modal_error, None);
}

#[test]
fn edit_started_maps_api_job_into_draft_and_missing_job_is_ignored() {
    let mut app = app();
    let mut value = job("agent");
    value.name = None;
    value.job_type = "legacy".to_string();
    value.schedule_kind = "legacy".to_string();
    value.expression.clear();
    value.at = Some("2026-07-18T00:00:00Z".to_string());
    value.every_ms = Some(30_000);
    value.command.clear();
    value.prompt = Some("summarize".to_string());
    value.model = Some("anthropic/claude".to_string());
    value.agent = None;
    value.acp_agent = None;
    value.project_path = None;
    value.wake = true;
    value.fallbacks = vec!["email".to_string(), "slack".to_string()];
    value.full_access = true;
    value.task_pool = true;
    value.delivery_mode = "announce".to_string();
    value.delivery_channel = None;
    value.delivery_to = None;
    value.delivery_best_effort = false;
    value.delete_after_run = true;
    app.cron_settings.jobs = vec![value];
    app.cron_settings.save_error = Some("old".to_string());
    app.cron_settings.action_status = Some("old status".to_string());

    let _ = update(&mut app, SettingsMessage::CronJobEditStarted("missing".to_string()));
    assert_eq!(app.cron_settings.editing_job_id, None);
    assert_eq!(app.cron_settings.save_error.as_deref(), Some("old"));

    let _ = update(&mut app, SettingsMessage::CronJobEditStarted("agent".to_string()));

    let draft = &app.cron_settings.edit_draft;
    assert_eq!(app.cron_settings.editing_job_id.as_deref(), Some("agent"));
    assert_eq!(draft.name, "");
    assert_eq!(draft.job_type, CronAddJobType::Agent);
    assert_eq!(draft.schedule_kind, CronAddScheduleKind::Every);
    assert_eq!(draft.at, "2026-07-18T00:00:00Z");
    assert_eq!(draft.every_ms, "30000");
    assert_eq!(draft.command_editor.text(), "");
    assert_eq!(draft.prompt_editor.text(), "summarize");
    assert_eq!(draft.session_target, "isolated");
    assert_eq!(draft.agent, "main");
    assert_eq!(draft.model_provider, "anthropic");
    assert_eq!(draft.fallbacks, "email\nslack");
    assert!(draft.wake && draft.full_access && draft.task_pool);
    assert!(draft.delivery_enabled && draft.delete_after_run);
    assert!(!draft.delivery_best_effort);
    assert_eq!(app.cron_settings.save_error, None);
    assert_eq!(app.cron_settings.action_status, None);
}

#[test]
fn edit_schedule_kind_clears_incompatible_fields_and_cancel_resets_draft() {
    let mut app = app();
    app.cron_settings.editing_job_id = Some("job-1".to_string());
    app.cron_settings.edit_draft.schedule = "cron".to_string();
    app.cron_settings.edit_draft.at = "at".to_string();
    app.cron_settings.edit_draft.every_ms = "1000".to_string();

    let _ =
        update(&mut app, SettingsMessage::CronJobEditScheduleKindChanged(CronAddScheduleKind::At));
    assert_eq!(app.cron_settings.edit_draft.schedule, "");
    assert_eq!(app.cron_settings.edit_draft.at, "at");
    assert_eq!(app.cron_settings.edit_draft.every_ms, "");

    app.cron_settings.edit_draft.schedule = "cron".to_string();
    app.cron_settings.edit_draft.every_ms = "1000".to_string();
    let _ = update(
        &mut app,
        SettingsMessage::CronJobEditScheduleKindChanged(CronAddScheduleKind::Every),
    );
    assert_eq!(app.cron_settings.edit_draft.schedule, "");
    assert_eq!(app.cron_settings.edit_draft.at, "");
    assert_eq!(app.cron_settings.edit_draft.every_ms, "1000");

    app.cron_settings.edit_draft.at = "at".to_string();
    let _ = update(
        &mut app,
        SettingsMessage::CronJobEditScheduleKindChanged(CronAddScheduleKind::Cron),
    );
    assert_eq!(app.cron_settings.edit_draft.at, "");
    assert_eq!(app.cron_settings.edit_draft.every_ms, "");

    let _ = update(&mut app, SettingsMessage::CronJobEditCanceled);
    assert_eq!(app.cron_settings.editing_job_id, None);
    assert_eq!(app.cron_settings.edit_draft.agent, "main");
    assert_eq!(app.cron_settings.edit_draft.schedule_kind, CronAddScheduleKind::Cron);
}

#[test]
fn add_draft_messages_update_fields_and_enforce_schedule_exclusivity() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronAddNameChanged("nightly".to_string()));
    let _ = update(&mut app, SettingsMessage::CronAddJobTypeChanged(CronAddJobType::Agent));
    let _ = update(&mut app, SettingsMessage::CronAddScheduleChanged("cron".to_string()));
    let _ = update(&mut app, SettingsMessage::CronAddAtChanged("at".to_string()));
    let _ = update(&mut app, SettingsMessage::CronAddEveryMsChanged("1000".to_string()));
    let _ = update(&mut app, SettingsMessage::CronAddCommandChanged("echo hi".to_string()));
    let _ = update(&mut app, SettingsMessage::CronAddPromptChanged("prompt".to_string()));
    let _ = update(&mut app, SettingsMessage::CronAddWakeToggled(true));
    let _ = update(&mut app, SettingsMessage::CronAddFullAccessToggled(true));
    let _ = update(&mut app, SettingsMessage::CronAddDeliveryEnabledToggled(true));

    assert_eq!(app.cron_settings.add_draft.name, "nightly");
    assert_eq!(app.cron_settings.add_draft.job_type, CronAddJobType::Agent);
    assert_eq!(app.cron_settings.add_draft.command_editor.text(), "echo hi");
    assert_eq!(app.cron_settings.add_draft.prompt_editor.text(), "prompt");
    assert!(app.cron_settings.add_draft.wake);
    assert!(!app.cron_settings.add_draft.full_access);
    assert!(app.cron_settings.add_draft.full_access_confirmation);
    assert!(app.cron_settings.add_draft.delivery_enabled);

    let _ = update(&mut app, SettingsMessage::CronAddScheduleKindChanged(CronAddScheduleKind::At));
    assert_eq!(app.cron_settings.add_draft.schedule, "");
    assert_eq!(app.cron_settings.add_draft.at, "at");
    assert_eq!(app.cron_settings.add_draft.every_ms, "");

    app.cron_settings.add_draft.schedule = "cron".to_string();
    app.cron_settings.add_draft.every_ms = "1000".to_string();
    let _ =
        update(&mut app, SettingsMessage::CronAddScheduleKindChanged(CronAddScheduleKind::Every));
    assert_eq!(app.cron_settings.add_draft.schedule, "");
    assert_eq!(app.cron_settings.add_draft.at, "");
    assert_eq!(app.cron_settings.add_draft.every_ms, "1000");

    app.cron_settings.add_draft.at = "at".to_string();
    let _ =
        update(&mut app, SettingsMessage::CronAddScheduleKindChanged(CronAddScheduleKind::Cron));
    assert_eq!(app.cron_settings.add_draft.at, "");
    assert_eq!(app.cron_settings.add_draft.every_ms, "");
}

#[test]
fn guarded_actions_report_errors_without_selected_or_edited_jobs() {
    let mut app = app();

    let _ = update(&mut app, SettingsMessage::CronJobEditSave);
    assert_eq!(app.cron_settings.save_error.as_deref(), Some("没有正在编辑的定时任务"));

    let _ = update(&mut app, SettingsMessage::CronSelectedJobsEnable);
    assert_eq!(app.cron_settings.save_error.as_deref(), Some("请先选择要启用的定时任务"));
    let _ = update(&mut app, SettingsMessage::CronSelectedJobsDisable);
    assert_eq!(app.cron_settings.save_error.as_deref(), Some("请先选择要禁用的定时任务"));
    let _ = update(&mut app, SettingsMessage::CronSelectedJobsDelete);
    assert_eq!(app.cron_settings.save_error.as_deref(), Some("请先选择要删除的定时任务"));
}

#[test]
fn mutation_completion_resets_drafts_on_success_and_preserves_state_on_error() {
    let mut app = app();
    app.cron_settings.editing_job_id = Some("job-1".to_string());
    app.cron_settings.edit_draft.name = "edited".to_string();
    app.cron_settings.add_draft.name = "new".to_string();
    app.cron_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::CronJobMutationCompleted(Ok("saved".to_string())));

    assert_eq!(app.cron_settings.action_status.as_deref(), Some("saved"));
    assert_eq!(app.cron_settings.save_error, None);
    assert_eq!(app.cron_settings.editing_job_id, None);
    assert_eq!(app.cron_settings.edit_draft.name, "");
    assert_eq!(app.cron_settings.add_draft.name, "");
    assert!(app.cron_settings.jobs_loading);

    app.cron_settings.action_status = Some("keep".to_string());
    app.cron_settings.jobs_loading = false;
    let _ = update(&mut app, SettingsMessage::CronJobMutationCompleted(Err("failed".to_string())));
    assert_eq!(app.cron_settings.save_error.as_deref(), Some("failed"));
    assert_eq!(app.cron_settings.action_status.as_deref(), Some("keep"));
    assert!(!app.cron_settings.jobs_loading);
}

#[test]
fn tabs_refresh_and_help_messages_reset_transient_state() {
    let mut app = app();
    app.cron_settings.save_error = Some("old".to_string());
    app.cron_settings.action_status = Some("old status".to_string());

    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Config));
    assert_eq!(app.cron_settings.active_tab, CronSettingsTab::Config);
    assert!(!app.cron_settings.jobs_loading);
    assert_eq!(app.cron_settings.save_error, None);
    assert_eq!(app.cron_settings.action_status, None);

    let _ = update(&mut app, SettingsMessage::CronJobsRefresh);
    assert!(app.cron_settings.jobs_loading);

    let _ = update(&mut app, SettingsMessage::CronHelpOpen);
    assert!(app.cron_settings.show_help_modal);
    let _ = update(&mut app, SettingsMessage::CronHelpClose);
    assert!(!app.cron_settings.show_help_modal);
}

#[test]
fn config_messages_apply_values_and_clamp_history_bounds() {
    let mut app = app();
    app.cron_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::CronEnabledToggled(false));
    assert!(!app.cron_settings.enabled);
    assert_eq!(app.cron_settings.save_error, None);

    let _ = update(&mut app, SettingsMessage::CronMaxRunHistoryChanged(0));
    assert_eq!(app.cron_settings.max_run_history, 1);

    let _ = update(&mut app, SettingsMessage::CronMaxRunHistoryChanged(20_000));
    assert_eq!(app.cron_settings.max_run_history, 10_000);
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn run_history_open_resets_previous_modal_state() {
    let mut app = app();
    app.cron_settings.runs_modal_job_id = Some("old".to_string());
    app.cron_settings.runs_modal_error = Some("old error".to_string());
    app.cron_settings.runs_modal = vec![run(1, Some("old output"), Some(4))];
    app.cron_settings.runs_modal_editor = text_editor::Content::with_text("old text");

    let _ = update(&mut app, SettingsMessage::CronJobRunsOpen("job-2".to_string()));

    assert_eq!(app.cron_settings.runs_modal_job_id.as_deref(), Some("job-2"));
    assert!(app.cron_settings.runs_modal_loading);
    assert_eq!(app.cron_settings.runs_modal_error, None);
    assert!(app.cron_settings.runs_modal.is_empty());
    assert_eq!(app.cron_settings.runs_modal_editor.text(), "");
}

#[test]
fn run_history_editor_allows_navigation_but_ignores_text_edits() {
    let mut app = app();
    app.cron_settings.runs_modal_editor = text_editor::Content::with_text("history");

    let _ =
        update(&mut app, SettingsMessage::CronJobRunsEditorAction(text_editor::Action::SelectAll));
    let _ = update(
        &mut app,
        SettingsMessage::CronJobRunsEditorAction(text_editor::Action::Edit(
            text_editor::Edit::Paste(Arc::new("replacement".to_string())),
        )),
    );

    assert_eq!(app.cron_settings.runs_modal_editor.text(), "history");
}

#[test]
fn edit_draft_editor_actions_keep_text_fields_in_sync() {
    let mut app = app();
    app.cron_settings.edit_draft.command_editor = text_editor::Content::with_text("old command");
    app.cron_settings.edit_draft.prompt_editor = text_editor::Content::with_text("old prompt");

    let _ = update(
        &mut app,
        SettingsMessage::CronJobEditCommandEditorAction(text_editor::Action::SelectAll),
    );
    let _ = update(
        &mut app,
        SettingsMessage::CronJobEditCommandEditorAction(text_editor::Action::Edit(
            text_editor::Edit::Paste(Arc::new("new command".to_string())),
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::CronJobEditPromptEditorAction(text_editor::Action::SelectAll),
    );
    let _ = update(
        &mut app,
        SettingsMessage::CronJobEditPromptEditorAction(text_editor::Action::Edit(
            text_editor::Edit::Paste(Arc::new("new prompt".to_string())),
        )),
    );

    assert_eq!(app.cron_settings.edit_draft.command, "new command");
    assert_eq!(app.cron_settings.edit_draft.prompt, "new prompt");
}

#[test]
fn add_draft_editor_actions_keep_text_fields_in_sync() {
    let mut app = app();
    app.cron_settings.add_draft.command_editor = text_editor::Content::with_text("old command");
    app.cron_settings.add_draft.prompt_editor = text_editor::Content::with_text("old prompt");

    let _ = update(
        &mut app,
        SettingsMessage::CronAddCommandEditorAction(text_editor::Action::SelectAll),
    );
    let _ = update(
        &mut app,
        SettingsMessage::CronAddCommandEditorAction(text_editor::Action::Edit(
            text_editor::Edit::Paste(Arc::new("new command".to_string())),
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::CronAddPromptEditorAction(text_editor::Action::SelectAll),
    );
    let _ = update(
        &mut app,
        SettingsMessage::CronAddPromptEditorAction(text_editor::Action::Edit(
            text_editor::Edit::Paste(Arc::new("new prompt".to_string())),
        )),
    );

    assert_eq!(app.cron_settings.add_draft.command, "new command");
    assert_eq!(app.cron_settings.add_draft.prompt, "new prompt");
}

#[test]
fn jobs_tab_selection_starts_loading_and_clears_transient_messages() {
    let mut app = app();
    app.cron_settings.save_error = Some("old error".to_string());
    app.cron_settings.action_status = Some("old status".to_string());

    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Jobs));

    assert_eq!(app.cron_settings.active_tab, CronSettingsTab::Jobs);
    assert!(app.cron_settings.jobs_loading);
    assert_eq!(app.cron_settings.save_error, None);
    assert_eq!(app.cron_settings.action_status, None);
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn mutation_actions_clear_errors_when_they_can_start() {
    let mut app = app();
    app.cron_settings.edit_draft.prompt = "report".into();
    app.cron_settings.add_draft.prompt = "report".into();

    app.cron_settings.save_error = Some("old".to_string());
    app.cron_settings.editing_job_id = Some("job-1".to_string());
    let _ = update(&mut app, SettingsMessage::CronJobEditSave);
    assert_eq!(app.cron_settings.save_error, None);
    assert!(app.cron_settings.edit_draft.submitting);
    let _ = update(&mut app, SettingsMessage::CronJobMutationCompleted(Err("offline".into())));
    assert!(!app.cron_settings.edit_draft.submitting);

    app.cron_settings.save_error = Some("old".to_string());
    let _ = update(&mut app, SettingsMessage::CronJobEnabledChanged("job-1".to_string(), false));
    assert_eq!(app.cron_settings.save_error, None);

    app.cron_settings.save_error = Some("old".to_string());
    let _ = update(&mut app, SettingsMessage::CronJobDelete("job-1".to_string()));
    assert_eq!(app.cron_settings.save_error, None);

    app.cron_settings.jobs = vec![job("job-1")];
    app.cron_settings.selected_job_ids = vec!["job-1".to_string()];
    app.cron_settings.save_error = Some("old".to_string());
    let _ = update(&mut app, SettingsMessage::CronSelectedJobsEnable);
    assert_eq!(app.cron_settings.save_error, None);

    app.cron_settings.save_error = Some("old".to_string());
    let _ = update(&mut app, SettingsMessage::CronSelectedJobsDisable);
    assert_eq!(app.cron_settings.save_error, None);

    app.cron_settings.save_error = Some("old".to_string());
    let _ = update(&mut app, SettingsMessage::CronSelectedJobsDelete);
    assert_eq!(app.cron_settings.save_error, None);

    app.cron_settings.save_error = Some("old".to_string());
    let _ = update(&mut app, SettingsMessage::CronAddSubmit);
    assert_eq!(app.cron_settings.save_error, None);
}

#[test]
fn common_frequency_selection_updates_both_drafts_and_advanced_preserves_expression() {
    let mut app = app();
    for frequency in CronFrequency::ALL {
        let _ = update(&mut app, SettingsMessage::CronAddFrequencyChanged(frequency));
        let _ = update(&mut app, SettingsMessage::CronJobEditFrequencyChanged(frequency));
        assert_eq!(app.cron_settings.add_draft.frequency, frequency);
        assert_eq!(app.cron_settings.edit_draft.frequency, frequency);
        if frequency != CronFrequency::Advanced {
            assert_eq!(app.cron_settings.add_draft.schedule, frequency.default_expression());
            assert_eq!(app.cron_settings.edit_draft.schedule, frequency.default_expression());
        } else {
            assert_eq!(app.cron_settings.add_draft.schedule, "0 9 1 * *");
            assert_eq!(app.cron_settings.edit_draft.schedule, "0 9 1 * *");
        }
    }
    let expression = "0 9 * * MON-FRI";
    let _ = update(&mut app, SettingsMessage::CronAddScheduleChanged(expression.to_string()));
    let _ = update(&mut app, SettingsMessage::CronAddFrequencyChanged(CronFrequency::Advanced));
    assert_eq!(app.cron_settings.add_draft.schedule, expression);
    assert_eq!(app.cron_settings.add_draft.frequency, CronFrequency::Advanced);
}

#[test]
fn model_selection_stays_consistent_with_provider_in_both_drafts() {
    let mut app = app();
    for edit in [false, true] {
        let model_message = |value: &str| {
            if edit {
                SettingsMessage::CronJobEditModelChanged(value.into())
            } else {
                SettingsMessage::CronAddModelChanged(value.into())
            }
        };
        let provider_message = |value: &str| {
            if edit {
                SettingsMessage::CronJobEditModelProviderChanged(value.into())
            } else {
                SettingsMessage::CronAddModelProviderChanged(value.into())
            }
        };
        let _ = update(&mut app, model_message("provider-a/model-a"));
        let _ = update(&mut app, provider_message("provider-a"));
        let draft = if edit { &app.cron_settings.edit_draft } else { &app.cron_settings.add_draft };
        assert_eq!(draft.model_provider, "provider-a");
        assert_eq!(draft.model, "provider-a/model-a");

        let _ = update(&mut app, provider_message("provider-b"));
        let draft = if edit { &app.cron_settings.edit_draft } else { &app.cron_settings.add_draft };
        assert_eq!(draft.model_provider, "provider-b");
        assert!(draft.model.is_empty());

        let _ = update(&mut app, model_message("provider-b/model-b"));
        let draft = if edit { &app.cron_settings.edit_draft } else { &app.cron_settings.add_draft };
        assert_eq!(draft.model_provider, "provider-b");
        assert_eq!(draft.model, "provider-b/model-b");
    }
}

#[test]
fn cron_list_batch_rejects_stale_selection_without_request() {
    let mut app = app();
    app.cron_settings.selected_job_ids = vec!["missing".into()];
    for message in [
        SettingsMessage::CronSelectedJobsEnable,
        SettingsMessage::CronSelectedJobsDisable,
        SettingsMessage::CronSelectedJobsDelete,
    ] {
        let task = update(&mut app, message);
        assert!(iced_runtime::task::into_stream(task).is_none());
        assert!(app.cron_settings.save_error.is_some());
    }
}

fn cron_list_jobs() -> Vec<CronJobDto> {
    let mut paused = job("paused");
    paused.enabled = false;
    vec![job("enabled-2"), paused, job("enabled-1")]
}

#[test]
fn cron_list_filter_counts_enabled_and_paused() {
    let mut app = app();
    assert_eq!(app.cron_settings.list_filter, CronListFilter::All);
    assert!(!app.cron_settings.batch_mode);
    app.cron_settings.jobs = cron_list_jobs();
    for (filter, count) in
        [(CronListFilter::All, 3), (CronListFilter::Enabled, 2), (CronListFilter::Paused, 1)]
    {
        let _ = update(&mut app, SettingsMessage::CronListFilterChanged(filter));
        assert_eq!(app.cron_settings.job_count(filter), count);
        assert_eq!(app.cron_settings.job_count(CronListFilter::All), 3);
        assert_eq!(app.cron_settings.visible_jobs().count(), count);
    }
}

#[test]
fn cron_list_select_all_targets_visible_jobs() {
    let mut app = app();
    app.cron_settings.jobs = cron_list_jobs();
    let _ = update(&mut app, SettingsMessage::CronBatchModeToggled(true));
    for (filter, expected) in [
        (CronListFilter::Enabled, vec!["enabled-2", "enabled-1"]),
        (CronListFilter::Paused, vec!["paused"]),
        (CronListFilter::All, vec!["enabled-2", "paused", "enabled-1"]),
    ] {
        let _ = update(&mut app, SettingsMessage::CronListFilterChanged(filter));
        let task = update(&mut app, SettingsMessage::CronJobsSelectAllToggled(true));
        assert!(iced_runtime::task::into_stream(task).is_none());
        assert_eq!(app.cron_settings.selected_job_ids, expected);
        let _ = update(&mut app, SettingsMessage::CronJobsSelectAllToggled(false));
        assert!(app.cron_settings.selected_job_ids.is_empty());
    }
}

#[test]
fn cron_list_filter_change_clears_selection() {
    let mut app = app();
    app.cron_settings.jobs = cron_list_jobs();
    let _ = update(&mut app, SettingsMessage::CronJobsSelectAllToggled(true));
    let _ = update(&mut app, SettingsMessage::CronListFilterChanged(CronListFilter::Paused));
    assert!(app.cron_settings.selected_job_ids.is_empty());
    let _ = update(&mut app, SettingsMessage::CronJobSelectionToggled("enabled-2".into(), true));
    let _ = update(&mut app, SettingsMessage::CronJobSelectionToggled("missing".into(), true));
    assert!(app.cron_settings.selected_job_ids.is_empty());
    let _ = update(&mut app, SettingsMessage::CronJobSelectionToggled("paused".into(), true));
    assert_eq!(app.cron_settings.selected_job_ids, vec!["paused"]);
}

#[test]
fn cron_list_batch_exit_clears_selection() {
    let mut app = app();
    app.cron_settings.jobs = cron_list_jobs();
    let _ = update(&mut app, SettingsMessage::CronBatchModeToggled(true));
    assert!(app.cron_settings.batch_mode);
    let _ = update(&mut app, SettingsMessage::CronJobsSelectAllToggled(true));
    let task = update(&mut app, SettingsMessage::CronBatchModeToggled(false));
    assert!(iced_runtime::task::into_stream(task).is_none());
    assert!(!app.cron_settings.batch_mode);
    assert!(app.cron_settings.selected_job_ids.is_empty());
}

#[test]
fn cron_list_reload_prunes_hidden_selection() {
    let mut app = app();
    app.cron_settings.jobs = cron_list_jobs();
    let _ = update(&mut app, SettingsMessage::CronListFilterChanged(CronListFilter::Enabled));
    let _ = update(&mut app, SettingsMessage::CronJobsSelectAllToggled(true));
    let mut changed = job("enabled-2");
    changed.enabled = false;
    let task = update(&mut app, SettingsMessage::CronJobsLoaded(Ok(vec![changed, job("new")])));
    assert!(iced_runtime::task::into_stream(task).is_none());
    assert!(app.cron_settings.selected_job_ids.is_empty());
    assert_eq!(
        app.cron_settings.visible_jobs().map(|job| job.id.as_str()).collect::<Vec<_>>(),
        vec!["new"]
    );
}

#[test]
fn cron_list_batch_limits_requests_to_visible_valid_selection() {
    let mut app = app();
    app.cron_settings.jobs = cron_list_jobs();
    let _ = update(&mut app, SettingsMessage::CronListFilterChanged(CronListFilter::Enabled));
    for message in [
        SettingsMessage::CronSelectedJobsEnable,
        SettingsMessage::CronSelectedJobsDisable,
        SettingsMessage::CronSelectedJobsDelete,
    ] {
        app.cron_settings.selected_job_ids = vec!["paused".into(), "missing".into()];
        let task = update(&mut app, message);
        assert!(iced_runtime::task::into_stream(task).is_none());
        assert!(app.cron_settings.selected_job_ids.is_empty());
    }
    for message in [
        SettingsMessage::CronSelectedJobsEnable,
        SettingsMessage::CronSelectedJobsDisable,
        SettingsMessage::CronSelectedJobsDelete,
    ] {
        app.cron_settings.selected_job_ids =
            vec!["paused".into(), "enabled-1".into(), "missing".into()];
        let task = update(&mut app, message);
        // Drop the unpolled stream: verify the request boundary without contacting a gateway.
        assert!(iced_runtime::task::into_stream(task).is_some());
        assert_eq!(app.cron_settings.selected_job_ids, vec!["enabled-1"]);
        assert!(app.cron_settings.save_error.is_none());
    }
}

#[test]
fn wb_schedule_draft_late_preview_ignored() {
    let mut app = app();
    let old = app.cron_settings.add_draft.schedule_revision;
    let _ = update(
        &mut app,
        SettingsMessage::CronScheduleDraftChanged { editing: false, draft: Default::default() },
    );
    let _ = update(
        &mut app,
        SettingsMessage::CronSchedulePreviewLoaded {
            editing: false,
            revision: old,
            result: Err("stale".into()),
        },
    );
    assert!(app.cron_settings.add_draft.preview.is_none());
    let revision = app.cron_settings.add_draft.schedule_revision;
    let _ = update(
        &mut app,
        SettingsMessage::CronSchedulePreviewLoaded {
            editing: false,
            revision,
            result: Err("current".into()),
        },
    );
    assert!(matches!(&app.cron_settings.add_draft.preview, Some(Err(e)) if e == "current"));
}

#[test]
fn wb_schedule_draft_cancel_restores_persisted_job() {
    let mut app = app();
    let mut original = job("calendar");
    original.schedule_kind = "biweekly".into();
    original.anchor_date = Some("2028-01-03".into());
    original.weekdays = Some(vec![1, 3]);
    original.hour = Some(9);
    original.minute = Some(30);
    original.timezone = Some("Asia/Shanghai".into());
    original.starts_at = Some("2028-01-01T00:00:00Z".into());
    app.cron_settings.jobs = vec![original.clone()];
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted(original.id.clone()));
    let before = schedule_request(&app.cron_settings.edit_draft).unwrap();
    let _ = update(
        &mut app,
        SettingsMessage::CronScheduleDraftChanged { editing: true, draft: Default::default() },
    );
    let _ = update(&mut app, SettingsMessage::CronJobEditCanceled);
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted(original.id.clone()));
    assert_eq!(
        serde_json::to_value(schedule_request(&app.cron_settings.edit_draft).unwrap()).unwrap(),
        serde_json::to_value(before).unwrap()
    );
    assert_eq!(app.cron_settings.jobs[0].starts_at, original.starts_at);
}

#[test]
fn wb_schedule_draft_failure_keeps_rule_and_releases_submission() {
    let mut app = app();
    let rule = crate::app::state::CronScheduleDraft::default();
    app.cron_settings.add_draft.schedule_draft = Some(rule.clone());
    app.cron_settings.add_draft.submitting = true;
    let _ = update(&mut app, SettingsMessage::CronJobMutationCompleted(Err("offline".into())));
    assert!(!app.cron_settings.add_draft.submitting);
    assert_eq!(app.cron_settings.add_draft.schedule_draft, Some(rule));
}

#[test]
fn wb_schedule_draft_duplicate_submit_ignored() {
    let mut app = app();
    app.cron_settings.add_draft.submitting = true;
    app.cron_settings.save_error = Some("sentinel".into());
    let _ = update(&mut app, SettingsMessage::CronAddSubmit);
    assert_eq!(app.cron_settings.save_error.as_deref(), Some("sentinel"));
    assert!(app.cron_settings.add_draft.submitting);
}

#[test]
fn wb_schedule_draft_legacy_edit_preserves_timezone_and_window_patch() {
    let mut app = app();
    let rule = crate::app::state::CronScheduleDraft {
        timezone: "Asia/Tokyo".into(),
        start_date: Some(None),
        ..Default::default()
    };
    app.cron_settings.edit_draft.schedule_draft = Some(rule);
    let _ = update(&mut app, SettingsMessage::CronJobEditScheduleChanged("30 10 * * *".into()));
    let r = schedule_request(&app.cron_settings.edit_draft).unwrap();
    assert_eq!(r.timezone.as_deref(), Some("Asia/Tokyo"));
    assert_eq!(r.start_date, Some(None));
    assert_eq!(r.schedule.as_deref(), Some("30 10 * * *"));
}

#[test]
fn wb_schedule_draft_invalid_legacy_input_keeps_timezone_when_corrected() {
    let mut app = app();
    let mut rule = crate::app::state::CronScheduleDraft {
        timezone: "Asia/Tokyo".into(),
        ..Default::default()
    };
    rule.rule = crate::app::state::CronScheduleRule::Every {
        amount: 2,
        unit: crate::app::state::IntervalUnit::Hours,
    };
    app.cron_settings.add_draft.schedule_kind = CronAddScheduleKind::Every;
    app.cron_settings.add_draft.schedule_draft = Some(rule);
    let _ = update(&mut app, SettingsMessage::CronAddEveryMsChanged("".into()));
    assert!(schedule_request(&app.cron_settings.add_draft).is_err());
    let _ = update(&mut app, SettingsMessage::CronAddEveryMsChanged("60000".into()));
    let r = schedule_request(&app.cron_settings.add_draft).unwrap();
    assert_eq!(r.timezone.as_deref(), Some("Asia/Tokyo"));
    assert_eq!(r.every_ms, Some(60000));
}

#[test]
fn wb_schedule_draft_unsupported_job_does_not_select_stale_edit_draft() {
    let mut app = app();
    let mut invalid = job("unsupported");
    invalid.schedule_kind = "unsupported".into();
    app.cron_settings.jobs = vec![invalid.clone()];
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted(invalid.id));
    assert!(app.cron_settings.editing_job_id.is_none());
    assert!(app.cron_settings.save_error.is_some());
}

#[test]
fn wb_schedule_picker_cancel_preserves_draft() {
    let mut app = app();
    app.cron_settings.add_draft.at = "2030-01-02T10:11:00Z".into();
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: false });
    let _ = update(
        &mut app,
        SettingsMessage::CronPickerDate(chrono::NaiveDate::from_ymd_opt(2031, 3, 4).unwrap()),
    );
    let _ = update(&mut app, SettingsMessage::CronPickerCancel);
    assert!(app.cron_settings.date_time_picker.is_none());
    assert_eq!(app.cron_settings.add_draft.at, "2030-01-02T10:11:00Z");
    assert!(app.cron_settings.add_draft.schedule_draft.is_none());
    assert_eq!(app.cron_settings.add_draft.schedule_revision, 0);
}

#[test]
fn wb_schedule_picker_confirm_commits_and_invalidates_preview() {
    let mut app = app();
    app.cron_settings.add_draft.preview = Some(Err("stale".into()));
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: false });
    let date = chrono::NaiveDate::from_ymd_opt(2031, 3, 4).unwrap();
    let _ = update(&mut app, SettingsMessage::CronPickerDate(date));
    let _ = update(&mut app, SettingsMessage::CronPickerTime { hour: 23, minute: 59 });
    let _ = update(&mut app, SettingsMessage::CronPickerConfirm);
    assert!(app.cron_settings.date_time_picker.is_none());
    let request = schedule_request(&app.cron_settings.add_draft).unwrap();
    assert_eq!(request.local_at.as_deref(), Some("2031-03-04T23:59"));
    assert_eq!(request.timezone.as_deref(), Some("UTC"));
    assert_eq!(app.cron_settings.add_draft.schedule_revision, 1);
    assert!(app.cron_settings.add_draft.preview.is_none());
}

#[test]
fn wb_schedule_picker_only_one_overlay_and_edit_isolation() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: false });
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: true });
    assert!(!app.cron_settings.date_time_picker.as_ref().unwrap().editing);
    let _ = update(&mut app, SettingsMessage::CronPickerCancel);
    app.cron_settings.editing_job_id = Some("job".into());
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: true });
    let _ = update(&mut app, SettingsMessage::CronPickerConfirm);
    assert!(app.cron_settings.edit_draft.schedule_draft.is_some());
    assert!(app.cron_settings.add_draft.schedule_draft.is_none());
}

#[test]
fn wb_schedule_picker_invalid_input_and_submitting_are_safe() {
    let mut app = app();
    app.cron_settings.add_draft.at = "invalid".into();
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: false });
    assert!(app.cron_settings.date_time_picker.is_some());
    let _ = update(&mut app, SettingsMessage::CronPickerCancel);
    app.cron_settings.add_draft.submitting = true;
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: false });
    assert!(app.cron_settings.date_time_picker.is_none());
}

#[test]
fn wb_schedule_picker_existing_absolute_uses_selected_timezone() {
    let mut app = app();
    let mut schedule = crate::app::state::CronScheduleDraft::default();
    schedule.timezone = "Asia/Shanghai".into();
    schedule.rule = crate::app::state::CronScheduleRule::AbsoluteAt("2030-12-31T23:35:42Z".into());
    app.cron_settings.add_draft.schedule_draft = Some(schedule);
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: false });
    let picker = app.cron_settings.date_time_picker.as_ref().unwrap();
    assert_eq!(picker.date.to_string(), "2031-01-01");
    assert_eq!(picker.time.to_string(), "07:35:00");
    let _ = update(&mut app, SettingsMessage::CronPickerConfirm);
    let request = schedule_request(&app.cron_settings.add_draft).unwrap();
    assert_eq!(request.timezone.as_deref(), Some("Asia/Shanghai"));
    assert_eq!(request.local_at.as_deref(), Some("2031-01-01T07:35"));
}

#[test]
fn wb_schedule_picker_stale_confirmation_preserves_newer_draft() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: false });
    let _ = update(&mut app, SettingsMessage::CronAddAtChanged("2035-01-01T00:00:00Z".into()));
    let _ = update(&mut app, SettingsMessage::CronPickerConfirm);
    assert_eq!(app.cron_settings.add_draft.at, "2035-01-01T00:00:00Z");
    assert!(app.cron_settings.add_draft.schedule_draft.is_none());
}

#[test]
fn wb_schedule_modal_new_opens_above_jobs() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    assert_eq!(app.cron_settings.active_tab, CronSettingsTab::Jobs);
    assert!(app.cron_settings.add_draft.schedule_draft.is_some());
}
#[test]
fn wb_schedule_modal_cancel_does_not_submit() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    let task = update(&mut app, SettingsMessage::CronJobEditCanceled);
    assert!(iced_runtime::task::into_stream(task).is_none());
    assert!(!app.cron_settings.show_add_modal);
    assert!(!app.cron_settings.add_draft.submitting);
    assert!(app.cron_settings.jobs.is_empty());
    assert_eq!(app.cron_settings.active_tab, CronSettingsTab::Jobs);
}

#[test]
fn wb_schedule_modal_edit_reads_timezone_and_window() {
    let mut app = app();
    let mut j = job("modal");
    j.expression = "30 9 * * MON,WED".into();
    j.timezone = Some("Asia/Shanghai".into());
    j.ends_at = Some("2099-01-01T00:00:00Z".into());
    app.cron_settings.jobs.push(j);
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted("modal".into()));
    let request = schedule_request(&app.cron_settings.edit_draft).unwrap();
    assert_eq!(request.timezone.as_deref(), Some("Asia/Shanghai"));
    assert_eq!(request.ends_at, Some(Some("2099-01-01T00:00:00+00:00".into())));
}

#[test]
fn wb_schedule_modal_empty_week_rejected() {
    let mut draft = crate::app::state::CronJobDraft::default();
    draft.schedule_draft = Some(crate::app::state::CronScheduleDraft {
        rule: crate::app::state::CronScheduleRule::Weekly {
            weekdays: vec![],
            time: "09:00:00".parse().unwrap(),
        },
        ..Default::default()
    });
    assert!(schedule_request(&draft).is_err());
}

#[test]
fn wb_schedule_modal_empty_month_rejected() {
    let mut draft = crate::app::state::CronJobDraft::default();
    draft.schedule_draft = Some(crate::app::state::CronScheduleDraft {
        rule: crate::app::state::CronScheduleRule::Monthly {
            days: vec![],
            time: "09:00:00".parse().unwrap(),
        },
        ..Default::default()
    });
    assert!(schedule_request(&draft).is_err());
}

#[test]
fn wb_schedule_modal_changed_rule_invalidates_preview() {
    let mut app = app();
    app.cron_settings.add_draft.preview = Some(Err("old error".into()));
    let before = app.cron_settings.add_draft.schedule_revision;
    let _ = update(
        &mut app,
        SettingsMessage::CronScheduleDraftChanged { editing: false, draft: Default::default() },
    );
    assert!(app.cron_settings.add_draft.preview.is_none());
    assert!(app.cron_settings.add_draft.schedule_revision > before);
}

#[test]
fn wb_schedule_modal_stale_preview_ignored() {
    let mut app = app();
    app.cron_settings.add_draft.schedule_revision = 8;
    let _ = update(
        &mut app,
        SettingsMessage::CronSchedulePreviewLoaded {
            editing: false,
            revision: 7,
            result: Err("stale".into()),
        },
    );
    assert!(app.cron_settings.add_draft.preview.is_none());
}

#[test]
fn wb_schedule_modal_failure_preserves_draft() {
    let mut app = app();
    app.cron_settings.add_draft.name = "保留草稿".into();
    let _ = update(&mut app, SettingsMessage::CronJobMutationCompleted(Err("network".into())));
    assert_eq!(app.cron_settings.add_draft.name, "保留草稿");
    assert_eq!(app.cron_settings.save_error.as_deref(), Some("network"));
}

#[test]
fn wb_schedule_modal_success_refreshes_jobs() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    let _ =
        update(&mut app, SettingsMessage::CronJobMutationCompleted(Ok("定时任务已新增".into())));
    assert!(app.cron_settings.jobs_loading);
    assert_eq!(app.cron_settings.active_tab, CronSettingsTab::Jobs);
    assert!(app.cron_settings.editing_job_id.is_none());
}

#[test]
fn wb_schedule_modal_missing_prompt_does_not_submit() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    let _ = update(&mut app, SettingsMessage::CronAddSubmit);
    assert!(!app.cron_settings.add_draft.submitting);
    assert!(app.cron_settings.save_error.is_some());
}

#[test]
fn wb_schedule_modal_unknown_timezone_does_not_submit() {
    let mut app = app();
    app.cron_settings.add_draft.schedule_draft = Some(crate::app::state::CronScheduleDraft {
        timezone: "Unknown/Nowhere".into(),
        ..Default::default()
    });
    assert!(schedule_request(&app.cron_settings.add_draft).is_err());
}

#[test]
fn wb_schedule_modal_biweekly_reopen_preserves_rule_and_window() {
    let mut app = app();
    let mut original = job("biweekly-modal");
    original.schedule_kind = "biweekly".into();
    original.anchor_date = Some("2099-01-01".into());
    original.weekdays = Some(vec![1, 3]);
    original.hour = Some(9);
    original.minute = Some(30);
    original.starts_at = Some("2098-12-31T16:00:00Z".into());
    original.ends_at = Some("2099-12-31T16:00:00Z".into());
    original.timezone = Some("Asia/Shanghai".into());
    app.cron_settings.jobs = vec![original.clone()];
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted(original.id.clone()));
    let before = schedule_request(&app.cron_settings.edit_draft).unwrap();
    let _ = update(&mut app, SettingsMessage::CronJobEditCanceled);
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted(original.id));
    let after = schedule_request(&app.cron_settings.edit_draft).unwrap();
    assert_eq!(serde_json::to_value(before).unwrap(), serde_json::to_value(after).unwrap());
}

#[test]
fn wb_schedule_modal_preview_request_never_submits_job() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    let task = update(&mut app, SettingsMessage::CronSchedulePreview { editing: false });
    assert!(iced_runtime::task::into_stream(task).is_some());
    assert!(app.cron_settings.add_draft.preview_loading);
    assert!(!app.cron_settings.add_draft.submitting);
    assert!(app.cron_settings.jobs.is_empty());
}

#[test]
fn wb_schedule_modal_no_future_trigger_rejects_save() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    app.cron_settings.add_draft.prompt = "检查项目".into();
    app.cron_settings.add_draft.preview = Some(Ok(vw_gateway_client::CronPreviewResponse {
        normalized: Default::default(),
        timezone: "UTC".into(),
        candidates: Vec::new(),
    }));
    let _ = update(&mut app, SettingsMessage::CronAddSubmit);
    assert!(!app.cron_settings.add_draft.submitting);
    assert_eq!(app.cron_settings.add_draft.prompt, "检查项目");
    assert!(app.cron_settings.save_error.as_deref().unwrap().contains("预览"));
}

#[test]
fn wb_schedule_feedback_cancel_keeps_restricted_access() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    let _ = update(&mut app, SettingsMessage::CronAddFullAccessToggled(true));
    assert!(!app.cron_settings.add_draft.full_access, "授权确认前必须保持受限");
    let _ = update(&mut app, SettingsMessage::CronFullAccessCancel { editing: false });
    assert!(!app.cron_settings.add_draft.full_access);
    assert!(!app.cron_settings.add_draft.full_access_confirmation);
    let _ = update(&mut app, SettingsMessage::CronFullAccessConfirm { editing: false });
    assert!(!app.cron_settings.add_draft.full_access);
}

#[test]
fn wb_schedule_feedback_frequency_keeps_restricted_access() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronAddFrequencyChanged(CronFrequency::Weekly));
    assert!(!app.cron_settings.add_draft.full_access);
}

#[test]
fn wb_schedule_feedback_missing_target_rejected() {
    let mut app = app();
    app.cron_settings.add_draft.prompt = "report".into();
    app.cron_settings.add_draft.delivery_enabled = true;
    app.cron_settings.add_draft.delivery_channel = "slack".into();
    let _ = update(&mut app, SettingsMessage::CronAddSubmit);
    assert!(!app.cron_settings.add_draft.submitting);
    assert!(app.cron_settings.save_error.as_deref().unwrap().contains("投递目标"));
}

#[test]
fn wb_schedule_feedback_unsupported_channel_preserved_and_rejected() {
    let mut app = app();
    let mut existing = job("unsupported");
    existing.delivery_mode = "announce".into();
    existing.delivery_channel = Some("wecom".into());
    app.cron_settings.jobs.push(existing);
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted("unsupported".into()));
    assert_eq!(app.cron_settings.edit_draft.delivery_channel, "wecom");
    app.cron_settings.edit_draft.schedule_draft = None;
    let _ = update(&mut app, SettingsMessage::CronJobEditSave);
    assert!(!app.cron_settings.edit_draft.submitting);
    assert!(app.cron_settings.save_error.as_deref().unwrap().contains("不支持"));
}

#[test]
fn wb_schedule_feedback_execution_error_readable_in_history() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronJobRunsOpen("job-1".into()));
    let mut denied = run(1, Some("blocked by security policy: read-only"), Some(1));
    denied.status = "error".into();
    let _ = update(&mut app, SettingsMessage::CronJobRunsLoaded("job-1".into(), Ok(vec![denied])));
    assert!(app.cron_settings.runs_modal_editor.text().contains("blocked by security policy"));
}

#[test]
fn wb_schedule_feedback_confirm_then_cancel_form_does_not_save_authority() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    let _ = update(&mut app, SettingsMessage::CronAddFullAccessToggled(true));
    let _ = update(&mut app, SettingsMessage::CronFullAccessConfirm { editing: false });
    assert!(app.cron_settings.add_draft.full_access);
    assert!(!app.cron_settings.add_draft.full_access_confirmation);
    let _ = update(&mut app, SettingsMessage::CronAddFrequencyChanged(CronFrequency::Weekly));
    assert!(app.cron_settings.add_draft.full_access);
    let _ = update(&mut app, SettingsMessage::CronJobEditCanceled);
    assert!(!app.cron_settings.add_draft.full_access);
    assert!(app.cron_settings.jobs.is_empty());
    let _ = update(&mut app, SettingsMessage::CronFullAccessConfirm { editing: false });
    assert!(!app.cron_settings.add_draft.full_access);
}

#[test]
fn wb_schedule_feedback_existing_authority_does_not_prompt_again() {
    let mut app = app();
    let mut existing = job("authorized");
    existing.full_access = true;
    app.cron_settings.jobs.push(existing);
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted("authorized".into()));
    assert!(app.cron_settings.edit_draft.full_access);
    assert!(!app.cron_settings.edit_draft.full_access_confirmation);
    let _ = update(&mut app, SettingsMessage::CronJobEditFrequencyChanged(CronFrequency::Weekly));
    assert!(app.cron_settings.edit_draft.full_access);
    let _ = update(&mut app, SettingsMessage::CronJobEditFullAccessToggled(false));
    let _ = update(&mut app, SettingsMessage::CronJobEditFullAccessToggled(true));
    assert!(!app.cron_settings.edit_draft.full_access);
    assert!(app.cron_settings.edit_draft.full_access_confirmation);
    let _ = update(&mut app, SettingsMessage::CronFullAccessCancel { editing: true });
    assert!(!app.cron_settings.edit_draft.full_access);
    let _ = update(&mut app, SettingsMessage::CronJobEditFullAccessToggled(true));
    let _ = update(&mut app, SettingsMessage::CronFullAccessConfirm { editing: true });
    assert!(app.cron_settings.edit_draft.full_access);
    let _ = update(&mut app, SettingsMessage::CronJobEditCanceled);
    assert!(app.cron_settings.jobs[0].full_access);
}

#[test]
fn wb_schedule_feedback_unconfigured_channel_is_explicit_error() {
    let mut app = app();
    app.channels_settings.slack = None;
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
fn wb_schedule_feedback_save_error_preserves_draft_and_shows_toast() {
    let mut app = app();
    app.cron_settings.add_draft.prompt = "report".into();
    app.cron_settings.add_draft.submitting = true;
    let _ = update(
        &mut app,
        SettingsMessage::CronJobMutationCompleted(Err("permission denied".into())),
    );
    assert_eq!(app.cron_settings.save_error.as_deref(), Some("permission denied"));
    let toast = app.active_toast.as_ref().expect("error toast");
    assert_eq!(toast.message, "permission denied");
    assert_eq!(toast.kind, crate::app::state::ToastKind::Error);
    assert_eq!(app.cron_settings.add_draft.prompt, "report");
    assert!(!app.cron_settings.add_draft.submitting);
}

fn history_app() -> App {
    let mut app = app();
    app.show_system_settings = true;
    app.system_settings_tab = crate::app::components::system_settings::SystemTab::Cron;
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::History));
    app
}

fn history_loaded(
    app: &mut App,
    generation: u64,
    result: Result<(vw_gateway_client::CronHistoryResponse, Vec<CronJobDto>), String>,
) {
    let _ = super::super::cron_history::update(
        app,
        super::super::cron_history::CronHistoryMessage::Loaded { generation, result },
    );
}

fn history_response() -> vw_gateway_client::CronHistoryResponse {
    vw_gateway_client::CronHistoryResponse {
        runs: vec![vw_gateway_client::CronHistoryRunDto {
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

#[test]
fn cron_history_stale_response_is_ignored() {
    let mut app = history_app();
    let generation = app.cron_settings.history.generation;
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Jobs));
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::History));
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
fn cron_history_refresh_error_preserves_rows() {
    use super::super::cron_history::{self, CronHistoryMessage};
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
fn cron_history_close_and_workspace_change_invalidate_responses() {
    use super::super::cron_history;
    let mut app = history_app();
    let first = app.cron_settings.history.generation;
    app.show_system_settings = false;
    let _ = app.update(Message::None);
    assert!(app.cron_settings.history.generation > first);
    history_loaded(&mut app, first, Ok((history_response(), vec![job("old")])));
    assert!(app.cron_settings.history.runs.is_empty());
    app.show_system_settings = true;
    let _ = app.update(Message::None);
    let second = app.cron_settings.history.generation;
    app.project_path = Some("/other".into());
    let _ = app.update(Message::None);
    assert!(app.cron_settings.history.generation > second);
    history_loaded(&mut app, second, Ok((history_response(), vec![job("old")])));
    assert!(app.cron_settings.history.runs.is_empty());
    assert!(cron_history::visible(&app));
}

#[test]
fn cron_history_candidates_include_unrun_jobs_and_disambiguate_ids() {
    let mut app = app();
    let mut first = job("a");
    first.name = Some("备份".into());
    let mut second = job("b");
    second.name = Some("备份".into());
    let mut third = job("c");
    third.name = Some(" DAILY ".into());
    app.cron_settings.jobs = vec![first, second, third];
    let history = &mut app.cron_settings.history;
    assert!(history.runs.is_empty());
    let candidates = history.task_candidates(&app.cron_settings.jobs);
    assert_eq!(candidates.len(), 3);
    assert_eq!(candidates[0], ("a", "备份 (a)".into()));
    assert_eq!(candidates[1], ("b", "备份 (b)".into()));
    history.task_search = " DaI ".into();
    assert_eq!(history.task_candidates(&app.cron_settings.jobs), vec![("c", "DAILY".into())]);
}

#[test]
fn cron_history_repeated_polling_errors_do_not_repeat_toast() {
    use super::super::cron_history::{self, CronHistoryMessage};
    let mut app = history_app();
    let generation = app.cron_settings.history.generation;
    history_loaded(&mut app, generation, Err("offline".into()));
    let toast_id = app.next_toast_id;
    let _ = cron_history::update(&mut app, CronHistoryMessage::Refresh);
    let generation = app.cron_settings.history.generation;
    history_loaded(&mut app, generation, Err("offline".into()));
    assert_eq!(app.next_toast_id, toast_id);
    let _ = cron_history::update(&mut app, CronHistoryMessage::Tick(generation));
    let current = app.cron_settings.history.generation;
    assert!(current > generation);
    let _ = cron_history::update(&mut app, CronHistoryMessage::Tick(generation));
    assert_eq!(app.cron_settings.history.generation, current);
}
