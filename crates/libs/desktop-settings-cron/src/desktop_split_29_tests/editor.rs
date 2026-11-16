use super::*;

#[test]
fn run_history_ignores_stale_results_and_handles_error_and_close() {
    let mut app = app();
    app.runs_modal_job_id = Some("current".to_string());
    app.runs_modal_loading = true;
    app.runs_modal = vec![run(1, Some("existing"), Some(10))];

    let _ = update(
        &mut app,
        SettingsMessage::CronJobRunsLoaded("stale".to_string(), Ok(vec![run(2, None, None)])),
    );
    assert!(app.runs_modal_loading);
    assert_eq!(app.runs_modal[0].id, 1);

    let _ = update(
        &mut app,
        SettingsMessage::CronJobRunsLoaded(
            "current".to_string(),
            Err("history unavailable".to_string()),
        ),
    );
    assert!(!app.runs_modal_loading);
    assert!(app.runs_modal.is_empty());
    assert_eq!(app.runs_modal_editor.text(), "");
    assert_eq!(app.runs_modal_error.as_deref(), Some("history unavailable"));

    let _ = update(&mut app, SettingsMessage::CronJobRunsClose);
    assert_eq!(app.runs_modal_job_id, None);
    assert_eq!(app.runs_modal_error, None);
}

#[test]
fn run_history_success_populates_read_only_text_state() {
    let mut app = app();
    app.runs_modal_job_id = Some("job-1".to_string());
    app.runs_modal_loading = true;
    app.runs_modal_error = Some("old".to_string());

    let _ = update(
        &mut app,
        SettingsMessage::CronJobRunsLoaded(
            "job-1".to_string(),
            Ok(vec![run(1, Some("hello"), Some(12))]),
        ),
    );

    assert!(!app.runs_modal_loading);
    assert_eq!(app.runs_modal.len(), 1);
    assert!(app.runs_modal_editor.text().contains("输出:\nhello"));
    assert_eq!(app.runs_modal_error, None);
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
    app.jobs = vec![value];
    app.save_error = Some("old".to_string());
    app.action_status = Some("old status".to_string());

    let _ = update(&mut app, SettingsMessage::CronJobEditStarted("missing".to_string()));
    assert_eq!(app.editing_job_id, None);
    assert_eq!(app.save_error.as_deref(), Some("old"));

    let _ = update(&mut app, SettingsMessage::CronJobEditStarted("agent".to_string()));

    let draft = &app.edit_draft;
    assert_eq!(app.editing_job_id.as_deref(), Some("agent"));
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
    assert_eq!(app.save_error, None);
    assert_eq!(app.action_status, None);
}

#[test]
fn edit_schedule_kind_clears_incompatible_fields_and_cancel_resets_draft() {
    let mut app = app();
    app.editing_job_id = Some("job-1".to_string());
    app.edit_draft.schedule = "cron".to_string();
    app.edit_draft.at = "at".to_string();
    app.edit_draft.every_ms = "1000".to_string();

    let _ =
        update(&mut app, SettingsMessage::CronJobEditScheduleKindChanged(CronAddScheduleKind::At));
    assert_eq!(app.edit_draft.schedule, "");
    assert_eq!(app.edit_draft.at, "at");
    assert_eq!(app.edit_draft.every_ms, "");

    app.edit_draft.schedule = "cron".to_string();
    app.edit_draft.every_ms = "1000".to_string();
    let _ = update(
        &mut app,
        SettingsMessage::CronJobEditScheduleKindChanged(CronAddScheduleKind::Every),
    );
    assert_eq!(app.edit_draft.schedule, "");
    assert_eq!(app.edit_draft.at, "");
    assert_eq!(app.edit_draft.every_ms, "1000");

    app.edit_draft.at = "at".to_string();
    let _ = update(
        &mut app,
        SettingsMessage::CronJobEditScheduleKindChanged(CronAddScheduleKind::Cron),
    );
    assert_eq!(app.edit_draft.at, "");
    assert_eq!(app.edit_draft.every_ms, "");

    let _ = update(&mut app, SettingsMessage::CronJobEditCanceled);
    assert_eq!(app.editing_job_id, None);
    assert_eq!(app.edit_draft.agent, "main");
    assert_eq!(app.edit_draft.schedule_kind, CronAddScheduleKind::Cron);
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

    assert_eq!(app.add_draft.name, "nightly");
    assert_eq!(app.add_draft.job_type, CronAddJobType::Agent);
    assert_eq!(app.add_draft.command_editor.text(), "echo hi");
    assert_eq!(app.add_draft.prompt_editor.text(), "prompt");
    assert!(app.add_draft.wake);
    assert!(!app.add_draft.full_access);
    assert!(app.add_draft.full_access_confirmation);
    assert!(app.add_draft.delivery_enabled);

    let _ = update(&mut app, SettingsMessage::CronAddScheduleKindChanged(CronAddScheduleKind::At));
    assert_eq!(app.add_draft.schedule, "");
    assert_eq!(app.add_draft.at, "at");
    assert_eq!(app.add_draft.every_ms, "");

    app.add_draft.schedule = "cron".to_string();
    app.add_draft.every_ms = "1000".to_string();
    let _ =
        update(&mut app, SettingsMessage::CronAddScheduleKindChanged(CronAddScheduleKind::Every));
    assert_eq!(app.add_draft.schedule, "");
    assert_eq!(app.add_draft.at, "");
    assert_eq!(app.add_draft.every_ms, "1000");

    app.add_draft.at = "at".to_string();
    let _ =
        update(&mut app, SettingsMessage::CronAddScheduleKindChanged(CronAddScheduleKind::Cron));
    assert_eq!(app.add_draft.at, "");
    assert_eq!(app.add_draft.every_ms, "");
}

#[test]
fn guarded_actions_report_errors_without_selected_or_edited_jobs() {
    let mut app = app();

    let _ = update(&mut app, SettingsMessage::CronJobEditSave);
    assert_eq!(app.save_error.as_deref(), Some("没有正在编辑的定时任务"));

    let _ = update(&mut app, SettingsMessage::CronSelectedJobsEnable);
    assert_eq!(app.save_error.as_deref(), Some("请先选择要启用的定时任务"));
    let _ = update(&mut app, SettingsMessage::CronSelectedJobsDisable);
    assert_eq!(app.save_error.as_deref(), Some("请先选择要禁用的定时任务"));
    let _ = update(&mut app, SettingsMessage::CronSelectedJobsDelete);
    assert_eq!(app.save_error.as_deref(), Some("请先选择要删除的定时任务"));
}

#[test]
fn mutation_completion_resets_drafts_on_success_and_preserves_state_on_error() {
    let mut app = app();
    app.editing_job_id = Some("job-1".to_string());
    app.edit_draft.name = "edited".to_string();
    app.add_draft.name = "new".to_string();
    app.save_error = Some("old".to_string());

    app.edit_draft.submitting = true;
    let _ = update(
        &mut app,
        SettingsMessage::CronEditorSaveCompleted { editing: true, result: Ok("saved".to_string()) },
    );

    assert_eq!(app.action_status.as_deref(), Some("saved"));
    assert_eq!(app.save_error, None);
    assert_eq!(app.editing_job_id, None);
    assert_eq!(app.edit_draft.name, "");
    assert_eq!(app.add_draft.name, "");
    assert!(app.jobs_loading);

    app.action_status = Some("keep".to_string());
    app.jobs_loading = false;
    app.edit_draft.submitting = true;
    let _ = update(
        &mut app,
        SettingsMessage::CronEditorSaveCompleted {
            editing: true,
            result: Err("failed".to_string()),
        },
    );
    assert_eq!(app.save_error.as_deref(), Some("failed"));
    assert_eq!(app.action_status.as_deref(), Some("keep"));
    assert!(!app.jobs_loading);
}

#[test]
fn tabs_refresh_and_help_messages_reset_transient_state() {
    let mut app = app();
    app.save_error = Some("old".to_string());
    app.action_status = Some("old status".to_string());

    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Config));
    assert_eq!(app.active_tab, CronSettingsTab::Config);
    assert!(!app.jobs_loading);
    assert_eq!(app.save_error, None);
    assert_eq!(app.action_status, None);

    let _ = update(&mut app, SettingsMessage::CronJobsRefresh);
    assert!(app.jobs_loading);

    let _ = update(&mut app, SettingsMessage::CronHelpOpen);
    assert!(app.show_help_modal);
    let _ = update(&mut app, SettingsMessage::CronHelpClose);
    assert!(!app.show_help_modal);
}

#[test]
fn config_messages_apply_values_and_clamp_history_bounds() {
    let mut app = app();
    app.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::CronEnabledToggled(false));
    assert!(!app.enabled);
    assert_eq!(app.save_error, None);

    let _ = update(&mut app, SettingsMessage::CronMaxRunHistoryChanged(0));
    assert_eq!(app.max_run_history, 1);

    let _ = update(&mut app, SettingsMessage::CronMaxRunHistoryChanged(20_000));
    assert_eq!(app.max_run_history, 10_000);
}

#[test]
fn run_history_open_resets_previous_modal_state() {
    let mut app = app();
    app.runs_modal_job_id = Some("old".to_string());
    app.runs_modal_error = Some("old error".to_string());
    app.runs_modal = vec![run(1, Some("old output"), Some(4))];
    app.runs_modal_editor = text_editor::Content::with_text("old text");

    let _ = update(&mut app, SettingsMessage::CronJobRunsOpen("job-2".to_string()));

    assert_eq!(app.runs_modal_job_id.as_deref(), Some("job-2"));
    assert!(app.runs_modal_loading);
    assert_eq!(app.runs_modal_error, None);
    assert!(app.runs_modal.is_empty());
    assert_eq!(app.runs_modal_editor.text(), "");
}

#[test]
fn run_history_editor_allows_navigation_but_ignores_text_edits() {
    let mut app = app();
    app.runs_modal_editor = text_editor::Content::with_text("history");

    let _ =
        update(&mut app, SettingsMessage::CronJobRunsEditorAction(text_editor::Action::SelectAll));
    let _ = update(
        &mut app,
        SettingsMessage::CronJobRunsEditorAction(text_editor::Action::Edit(
            text_editor::Edit::Paste(Arc::new("replacement".to_string())),
        )),
    );

    assert_eq!(app.runs_modal_editor.text(), "history");
}

#[test]
fn edit_draft_editor_actions_keep_text_fields_in_sync() {
    let mut app = app();
    app.edit_draft.command_editor = text_editor::Content::with_text("old command");
    app.edit_draft.prompt_editor = text_editor::Content::with_text("old prompt");

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

    assert_eq!(app.edit_draft.command, "new command");
    assert_eq!(app.edit_draft.prompt, "new prompt");
}

#[test]
fn add_draft_editor_actions_keep_text_fields_in_sync() {
    let mut app = app();
    app.add_draft.command_editor = text_editor::Content::with_text("old command");
    app.add_draft.prompt_editor = text_editor::Content::with_text("old prompt");

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

    assert_eq!(app.add_draft.command, "new command");
    assert_eq!(app.add_draft.prompt, "new prompt");
}

#[test]
fn jobs_tab_selection_starts_loading_and_clears_transient_messages() {
    let mut app = app();
    app.save_error = Some("old error".to_string());
    app.action_status = Some("old status".to_string());

    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Jobs));

    assert_eq!(app.active_tab, CronSettingsTab::Jobs);
    assert!(app.jobs_loading);
    assert_eq!(app.save_error, None);
    assert_eq!(app.action_status, None);
}

#[test]
fn mutation_actions_clear_errors_when_they_can_start() {
    let mut app = app();
    app.edit_draft.prompt = "report".into();
    app.add_draft.prompt = "report".into();

    app.save_error = Some("old".to_string());
    app.editing_job_id = Some("job-1".to_string());
    let _ = update(&mut app, SettingsMessage::CronJobEditSave);
    assert_eq!(app.save_error, None);
    assert!(app.edit_draft.submitting);
    let _ = update(
        &mut app,
        SettingsMessage::CronEditorSaveCompleted { editing: true, result: Err("offline".into()) },
    );
    assert!(!app.edit_draft.submitting);

    app.save_error = Some("old".to_string());
    let _ = update(&mut app, SettingsMessage::CronJobEnabledChanged("job-1".to_string(), false));
    assert_eq!(app.save_error, None);

    app.save_error = Some("old".to_string());
    let _ = update(&mut app, SettingsMessage::CronJobDelete("job-1".to_string()));
    assert_eq!(app.save_error, None);

    app.jobs = vec![job("job-1")];
    app.selected_job_ids = vec!["job-1".to_string()];
    app.save_error = Some("old".to_string());
    let _ = update(&mut app, SettingsMessage::CronSelectedJobsEnable);
    assert_eq!(app.save_error, None);

    app.save_error = Some("old".to_string());
    let _ = update(&mut app, SettingsMessage::CronSelectedJobsDisable);
    assert_eq!(app.save_error, None);

    app.save_error = Some("old".to_string());
    let _ = update(&mut app, SettingsMessage::CronSelectedJobsDelete);
    assert_eq!(app.save_error, None);

    app.save_error = Some("old".to_string());
    let _ = update(&mut app, SettingsMessage::CronAddSubmit);
    assert_eq!(app.save_error, None);
}

#[test]
fn common_frequency_selection_updates_both_drafts_and_advanced_preserves_expression() {
    let mut app = app();
    for frequency in [CronFrequency::Minutes, CronFrequency::Hourly, CronFrequency::Daily,
        CronFrequency::Weekly, CronFrequency::Monthly, CronFrequency::Advanced] {
        let _ = update(&mut app, SettingsMessage::CronAddFrequencyChanged(frequency));
        let _ = update(&mut app, SettingsMessage::CronJobEditFrequencyChanged(frequency));
        assert_eq!(app.add_draft.frequency, frequency);
        assert_eq!(app.edit_draft.frequency, frequency);
        if frequency != CronFrequency::Advanced {
            assert_eq!(app.add_draft.schedule, frequency.default_expression());
            assert_eq!(app.edit_draft.schedule, frequency.default_expression());
        } else {
            assert_eq!(app.add_draft.schedule, "0 9 1 * *");
            assert_eq!(app.edit_draft.schedule, "0 9 1 * *");
        }
    }
    let expression = "0 9 * * MON-FRI";
    let _ = update(&mut app, SettingsMessage::CronAddScheduleChanged(expression.to_string()));
    let _ = update(&mut app, SettingsMessage::CronAddFrequencyChanged(CronFrequency::Advanced));
    assert_eq!(app.add_draft.schedule, expression);
    assert_eq!(app.add_draft.frequency, CronFrequency::Advanced);
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
        let draft = if edit { &app.edit_draft } else { &app.add_draft };
        assert_eq!(draft.model_provider, "provider-a");
        assert_eq!(draft.model, "provider-a/model-a");

        let _ = update(&mut app, provider_message("provider-b"));
        let draft = if edit { &app.edit_draft } else { &app.add_draft };
        assert_eq!(draft.model_provider, "provider-b");
        assert!(draft.model.is_empty());

        let _ = update(&mut app, model_message("provider-b/model-b"));
        let draft = if edit { &app.edit_draft } else { &app.add_draft };
        assert_eq!(draft.model_provider, "provider-b");
        assert_eq!(draft.model, "provider-b/model-b");
    }
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
    app.jobs = vec![first, second, third];
    let history = &mut app.history;
    assert!(history.runs.is_empty());
    let candidates = history.task_candidates(&app.jobs);
    assert_eq!(candidates.len(), 3);
    assert_eq!(candidates[0], ("a", "备份 (a)".into()));
    assert_eq!(candidates[1], ("b", "备份 (b)".into()));
    history.task_search = " DaI ".into();
    assert_eq!(history.task_candidates(&app.jobs), vec![("c", "DAILY".into())]);
}

#[test]
fn cron_wecom_form_switch_clears_target_and_uses_default_group() {
    let mut app = app();
    for editing in [false, true] {
        let target = if editing {
            &mut app.edit_draft
        } else {
            &mut app.add_draft
        };
        target.delivery_channel = "wecom".into();
        target.delivery_to = "user-private".into();
        let msg = if editing {
            SettingsMessage::CronJobEditDeliveryChannelChanged("wecom_webhook".into())
        } else {
            SettingsMessage::CronAddDeliveryChannelChanged("wecom_webhook".into())
        };
        drop(update(&mut app, msg));
        let target =
            if editing { &app.edit_draft } else { &app.add_draft };
        assert_eq!(target.delivery_to, "default");
        let msg = if editing {
            SettingsMessage::CronJobEditDeliveryChannelChanged("wecom".into())
        } else {
            SettingsMessage::CronAddDeliveryChannelChanged("wecom".into())
        };
        drop(update(&mut app, msg));
        let target =
            if editing { &app.edit_draft } else { &app.add_draft };
        assert!(target.delivery_to.is_empty());
    }
}

#[test]
fn cron_modal_unrelated_mutation_preserves_draft() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    app.add_draft.name = "保留草稿".into();
    app.add_draft.submitting = true;
    let _ = update(&mut app, SettingsMessage::CronJobMutationCompleted(Ok("deleted".into())));
    assert!(app.show_add_modal);
    assert!(app.add_draft.submitting);
    assert_eq!(app.add_draft.name, "保留草稿");
}

#[test]
fn cron_modal_busy_edit_switch_is_ignored() {
    let mut app = app();
    app.jobs = vec![job("other")];
    app.show_add_modal = true;
    app.add_draft.submitting = true;
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted("other".into()));
    assert!(app.show_add_modal);
    assert!(app.editing_job_id.is_none());
}

#[test]
fn cron_modal_open_add_uses_clean_draft() {
    let mut app = app();
    app.add_draft.name = "旧草稿".into();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    assert!(app.show_add_modal);
    assert_eq!(app.active_tab, CronSettingsTab::Jobs);
    assert!(app.add_draft.name.is_empty());
}

#[test]
fn cron_modal_cancel_discards_without_mutation() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    app.add_draft.name = "草稿".into();
    let task = update(&mut app, SettingsMessage::CronJobEditCanceled);
    assert_eq!(task.units(), 0);
    assert!(!app.show_add_modal);
    assert!(app.add_draft.name.is_empty());
}

#[test]
fn cron_modal_failed_save_keeps_draft() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    app.add_draft.name = "草稿".into();
    app.add_draft.submitting = true;
    let _ = update(
        &mut app,
        SettingsMessage::CronEditorSaveCompleted { editing: false, result: Err("offline".into()) },
    );
    assert!(app.show_add_modal);
    assert!(!app.add_draft.submitting);
    assert_eq!(app.add_draft.name, "草稿");
}

#[test]
fn cron_modal_success_closes_and_refreshes() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    app.add_draft.submitting = true;
    let _ = update(
        &mut app,
        SettingsMessage::CronEditorSaveCompleted { editing: false, result: Ok("saved".into()) },
    );
    assert!(!app.show_add_modal);
    assert!(app.jobs_loading);
}

#[test]
fn cron_modal_double_submit_is_ignored() {
    let mut app = app();
    app.add_draft.submitting = true;
    let task = update(&mut app, SettingsMessage::CronAddSubmit);
    assert_eq!(task.units(), 0);
    assert!(app.add_draft.submitting);
}
