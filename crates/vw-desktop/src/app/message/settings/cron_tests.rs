use super::*;
use crate::app::App;
use crate::app::state::{CronAddJobType, CronAddScheduleKind, CronSettingsTab};
use std::sync::Arc;
use vw_gateway_client::{CronJobDto, CronRunDto};

fn app() -> App {
    App::new().0
}

fn job(id: &str) -> CronJobDto {
    CronJobDto {
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
        format_run_history_text(&runs),
        concat!(
            "#1\n状态: ok\n开始: start-1\n结束: end-1\n",
            "耗时: 12 ms\n输出:\nhello\n\n",
            "#2\n状态: ok\n开始: start-2\n结束: end-2\n",
            "耗时: 无\n输出:\n无输出"
        )
    );
    assert_eq!(format_run_history_text(&[]), "");
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
    assert!(app.cron_settings.add_draft.full_access);
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
fn edit_draft_messages_update_remaining_delivery_and_runtime_fields() {
    let mut app = app();

    let messages = [
        SettingsMessage::CronJobEditNameChanged("edited".to_string()),
        SettingsMessage::CronJobEditJobTypeChanged(CronAddJobType::Agent),
        SettingsMessage::CronJobEditScheduleChanged("0 1 * * *".to_string()),
        SettingsMessage::CronJobEditAtChanged("2026-07-18T00:00:00Z".to_string()),
        SettingsMessage::CronJobEditEveryMsChanged("5000".to_string()),
        SettingsMessage::CronJobEditAgentChanged("worker".to_string()),
        SettingsMessage::CronJobEditAcpAgentChanged("codex".to_string()),
        SettingsMessage::CronJobEditProjectPathChanged("/project".to_string()),
        SettingsMessage::CronJobEditModelProviderChanged("openai".to_string()),
        SettingsMessage::CronJobEditModelChanged("openai/gpt-5".to_string()),
        SettingsMessage::CronJobEditFallbacksChanged("email\nslack".to_string()),
        SettingsMessage::CronJobEditDeliveryChannelChanged("matrix".to_string()),
        SettingsMessage::CronJobEditDeliveryToChanged("!room".to_string()),
    ];
    for message in messages {
        let _ = update(&mut app, message);
    }
    let _ = update(&mut app, SettingsMessage::CronJobEditWakeToggled(true));
    let _ = update(&mut app, SettingsMessage::CronJobEditFullAccessToggled(true));
    let _ = update(&mut app, SettingsMessage::CronJobEditTaskPoolToggled(true));
    let _ = update(&mut app, SettingsMessage::CronJobEditDeliveryEnabledToggled(true));
    let _ = update(&mut app, SettingsMessage::CronJobEditDeliveryBestEffortToggled(false));
    let _ = update(&mut app, SettingsMessage::CronJobEditDeleteAfterRunToggled(true));

    let draft = &app.cron_settings.edit_draft;
    assert_eq!(draft.name, "edited");
    assert_eq!(draft.job_type, CronAddJobType::Agent);
    assert_eq!(draft.schedule, "0 1 * * *");
    assert_eq!(draft.at, "2026-07-18T00:00:00Z");
    assert_eq!(draft.every_ms, "5000");
    assert_eq!(draft.agent, "worker");
    assert_eq!(draft.acp_agent, "codex");
    assert_eq!(draft.project_path, "/project");
    assert_eq!(draft.model_provider, "openai");
    assert_eq!(draft.model, "openai/gpt-5");
    assert_eq!(draft.fallbacks, "email\nslack");
    assert_eq!(draft.delivery_channel, "matrix");
    assert_eq!(draft.delivery_to, "!room");
    assert!(draft.wake && draft.full_access && draft.task_pool && draft.delivery_enabled);
    assert!(!draft.delivery_best_effort);
    assert!(draft.delete_after_run);
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
fn add_draft_messages_update_remaining_delivery_and_runtime_fields() {
    let mut app = app();

    let messages = [
        SettingsMessage::CronAddSessionTargetChanged("isolated".to_string()),
        SettingsMessage::CronAddAgentChanged("worker".to_string()),
        SettingsMessage::CronAddAcpAgentChanged("codex".to_string()),
        SettingsMessage::CronAddProjectPathChanged("/project".to_string()),
        SettingsMessage::CronAddModelProviderChanged("openai".to_string()),
        SettingsMessage::CronAddModelChanged("openai/gpt-5".to_string()),
        SettingsMessage::CronAddFallbacksChanged("email\nslack".to_string()),
        SettingsMessage::CronAddDeliveryChannelChanged("matrix".to_string()),
        SettingsMessage::CronAddDeliveryToChanged("!room".to_string()),
    ];
    for message in messages {
        let _ = update(&mut app, message);
    }
    let _ = update(&mut app, SettingsMessage::CronAddTaskPoolToggled(true));
    let _ = update(&mut app, SettingsMessage::CronAddDeliveryBestEffortToggled(false));
    let _ = update(&mut app, SettingsMessage::CronAddDeleteAfterRunToggled(true));

    let draft = &app.cron_settings.add_draft;
    assert_eq!(draft.session_target, "isolated");
    assert_eq!(draft.agent, "worker");
    assert_eq!(draft.acp_agent, "codex");
    assert_eq!(draft.project_path, "/project");
    assert_eq!(draft.model_provider, "openai");
    assert_eq!(draft.model, "openai/gpt-5");
    assert_eq!(draft.fallbacks, "email\nslack");
    assert_eq!(draft.delivery_channel, "matrix");
    assert_eq!(draft.delivery_to, "!room");
    assert!(draft.task_pool && draft.delete_after_run);
    assert!(!draft.delivery_best_effort);
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

#[test]
fn mutation_actions_clear_errors_when_they_can_start() {
    let mut app = app();

    app.cron_settings.save_error = Some("old".to_string());
    app.cron_settings.editing_job_id = Some("job-1".to_string());
    let _ = update(&mut app, SettingsMessage::CronJobEditSave);
    assert_eq!(app.cron_settings.save_error, None);

    app.cron_settings.save_error = Some("old".to_string());
    let _ = update(&mut app, SettingsMessage::CronJobEnabledChanged("job-1".to_string(), false));
    assert_eq!(app.cron_settings.save_error, None);

    app.cron_settings.save_error = Some("old".to_string());
    let _ = update(&mut app, SettingsMessage::CronJobDelete("job-1".to_string()));
    assert_eq!(app.cron_settings.save_error, None);

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
