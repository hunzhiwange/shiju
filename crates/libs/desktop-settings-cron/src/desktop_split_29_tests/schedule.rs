use super::*;

#[test]
fn wb_schedule_draft_late_preview_ignored() {
    let mut app = app();
    let old = app.add_draft.schedule_revision;
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
    assert!(app.add_draft.preview.is_none());
    let revision = app.add_draft.schedule_revision;
    let _ = update(
        &mut app,
        SettingsMessage::CronSchedulePreviewLoaded {
            editing: false,
            revision,
            result: Err("current".into()),
        },
    );
    assert!(matches!(&app.add_draft.preview, Some(Err(e)) if e == "current"));
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
    app.jobs = vec![original.clone()];
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted(original.id.clone()));
    let before = schedule_request(&app.edit_draft).unwrap();
    let _ = update(
        &mut app,
        SettingsMessage::CronScheduleDraftChanged { editing: true, draft: Default::default() },
    );
    let _ = update(&mut app, SettingsMessage::CronJobEditCanceled);
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted(original.id.clone()));
    assert_eq!(
        serde_json::to_value(schedule_request(&app.edit_draft).unwrap()).unwrap(),
        serde_json::to_value(before).unwrap()
    );
    assert_eq!(app.jobs[0].starts_at, original.starts_at);
}

#[test]
fn wb_schedule_draft_duplicate_submit_ignored() {
    let mut app = app();
    app.add_draft.submitting = true;
    app.save_error = Some("sentinel".into());
    let _ = update(&mut app, SettingsMessage::CronAddSubmit);
    assert_eq!(app.save_error.as_deref(), Some("sentinel"));
    assert!(app.add_draft.submitting);
}

#[test]
fn wb_schedule_draft_unsupported_job_does_not_select_stale_edit_draft() {
    let mut app = app();
    let mut invalid = job("unsupported");
    invalid.schedule_kind = "unsupported".into();
    app.jobs = vec![invalid.clone()];
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted(invalid.id));
    assert!(app.editing_job_id.is_none());
    assert!(app.save_error.is_some());
}

#[test]
fn wb_schedule_picker_cancel_preserves_draft() {
    let mut app = app();
    app.add_draft.at = "2030-01-02T10:11:00Z".into();
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: false });
    let _ = update(
        &mut app,
        SettingsMessage::CronPickerDate(chrono::NaiveDate::from_ymd_opt(2031, 3, 4).unwrap()),
    );
    let _ = update(&mut app, SettingsMessage::CronPickerCancel);
    assert!(app.date_time_picker.is_none());
    assert_eq!(app.add_draft.at, "2030-01-02T10:11:00Z");
    assert!(app.add_draft.schedule_draft.is_none());
    assert_eq!(app.add_draft.schedule_revision, 0);
}

#[test]
fn wb_schedule_picker_confirm_commits_and_invalidates_preview() {
    let mut app = app();
    app.add_draft.preview = Some(Err("stale".into()));
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: false });
    let date = chrono::NaiveDate::from_ymd_opt(2031, 3, 4).unwrap();
    let _ = update(&mut app, SettingsMessage::CronPickerDate(date));
    let _ = update(&mut app, SettingsMessage::CronPickerTime { hour: 23, minute: 59 });
    let _ = update(&mut app, SettingsMessage::CronPickerConfirm);
    assert!(app.date_time_picker.is_none());
    let request = schedule_request(&app.add_draft).unwrap();
    assert_eq!(request.local_at.as_deref(), Some("2031-03-04T23:59"));
    assert_eq!(request.timezone.as_deref(), Some("UTC"));
    assert_eq!(app.add_draft.schedule_revision, 1);
    assert!(app.add_draft.preview.is_none());
}

#[test]
fn wb_schedule_picker_only_one_overlay_and_edit_isolation() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: false });
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: true });
    assert!(!app.date_time_picker.as_ref().unwrap().editing);
    let _ = update(&mut app, SettingsMessage::CronPickerCancel);
    app.editing_job_id = Some("job".into());
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: true });
    let _ = update(&mut app, SettingsMessage::CronPickerConfirm);
    assert!(app.edit_draft.schedule_draft.is_some());
    assert!(app.add_draft.schedule_draft.is_none());
}

#[test]
fn wb_schedule_picker_invalid_input_and_submitting_are_safe() {
    let mut app = app();
    app.add_draft.at = "invalid".into();
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: false });
    assert!(app.date_time_picker.is_some());
    let _ = update(&mut app, SettingsMessage::CronPickerCancel);
    app.add_draft.submitting = true;
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: false });
    assert!(app.date_time_picker.is_none());
}

#[test]
fn wb_schedule_picker_stale_confirmation_preserves_newer_draft() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: false });
    let _ = update(&mut app, SettingsMessage::CronAddAtChanged("2035-01-01T00:00:00Z".into()));
    let _ = update(&mut app, SettingsMessage::CronPickerConfirm);
    assert_eq!(app.add_draft.at, "2035-01-01T00:00:00Z");
    assert!(app.add_draft.schedule_draft.is_none());
}

#[test]
fn wb_schedule_modal_new_opens_above_jobs() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    assert_eq!(app.active_tab, CronSettingsTab::Jobs);
    assert!(app.add_draft.schedule_draft.is_some());
}

#[test]
fn wb_schedule_modal_cancel_does_not_submit() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    let task = update(&mut app, SettingsMessage::CronJobEditCanceled);
    assert!(iced_runtime::task::into_stream(task).is_none());
    assert!(!app.show_add_modal);
    assert!(!app.add_draft.submitting);
    assert!(app.jobs.is_empty());
    assert_eq!(app.active_tab, CronSettingsTab::Jobs);
}

#[test]
fn wb_schedule_modal_edit_reads_timezone_and_window() {
    let mut app = app();
    let mut j = job("modal");
    j.expression = "30 9 * * MON,WED".into();
    j.timezone = Some("Asia/Shanghai".into());
    j.ends_at = Some("2099-01-01T00:00:00Z".into());
    app.jobs.push(j);
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted("modal".into()));
    let request = schedule_request(&app.edit_draft).unwrap();
    assert_eq!(request.timezone.as_deref(), Some("Asia/Shanghai"));
    assert_eq!(request.ends_at, Some(Some("2099-01-01T00:00:00+00:00".into())));
}

#[test]
fn wb_schedule_modal_changed_rule_invalidates_preview() {
    let mut app = app();
    app.add_draft.preview = Some(Err("old error".into()));
    let before = app.add_draft.schedule_revision;
    let _ = update(
        &mut app,
        SettingsMessage::CronScheduleDraftChanged { editing: false, draft: Default::default() },
    );
    assert!(app.add_draft.preview.is_none());
    assert!(app.add_draft.schedule_revision > before);
}

#[test]
fn wb_schedule_modal_stale_preview_ignored() {
    let mut app = app();
    app.add_draft.schedule_revision = 8;
    let _ = update(
        &mut app,
        SettingsMessage::CronSchedulePreviewLoaded {
            editing: false,
            revision: 7,
            result: Err("stale".into()),
        },
    );
    assert!(app.add_draft.preview.is_none());
}

#[test]
fn wb_schedule_modal_failure_preserves_draft() {
    let mut app = app();
    app.add_draft.name = "保留草稿".into();
    app.add_draft.submitting = true;
    let _ = update(
        &mut app,
        SettingsMessage::CronEditorSaveCompleted { editing: false, result: Err("network".into()) },
    );
    assert_eq!(app.add_draft.name, "保留草稿");
    assert_eq!(app.save_error.as_deref(), Some("network"));
}

#[test]
fn wb_schedule_modal_success_refreshes_jobs() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    app.add_draft.submitting = true;
    let _ = update(
        &mut app,
        SettingsMessage::CronEditorSaveCompleted {
            editing: false,
            result: Ok("定时任务已新增".into()),
        },
    );
    assert!(app.jobs_loading);
    assert_eq!(app.active_tab, CronSettingsTab::Jobs);
    assert!(app.editing_job_id.is_none());
}

#[test]
fn wb_schedule_modal_missing_prompt_does_not_submit() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    let _ = update(&mut app, SettingsMessage::CronAddSubmit);
    assert!(!app.add_draft.submitting);
    assert!(app.save_error.is_some());
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
    app.jobs = vec![original.clone()];
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted(original.id.clone()));
    let before = schedule_request(&app.edit_draft).unwrap();
    let _ = update(&mut app, SettingsMessage::CronJobEditCanceled);
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted(original.id));
    let after = schedule_request(&app.edit_draft).unwrap();
    assert_eq!(serde_json::to_value(before).unwrap(), serde_json::to_value(after).unwrap());
}

#[test]
fn wb_schedule_modal_preview_request_never_submits_job() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    let task = update(&mut app, SettingsMessage::CronSchedulePreview { editing: false });
    assert!(iced_runtime::task::into_stream(task).is_some());
    assert!(app.add_draft.preview_loading);
    assert!(!app.add_draft.submitting);
    assert!(app.jobs.is_empty());
}

#[test]
fn wb_schedule_modal_no_future_trigger_rejects_save() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    app.add_draft.prompt = "检查项目".into();
    app.add_draft.preview = Some(Ok(gateway_cron::CronPreviewResponse {
        normalized: Default::default(),
        timezone: "UTC".into(),
        candidates: Vec::new(),
    }));
    let _ = update(&mut app, SettingsMessage::CronAddSubmit);
    assert!(!app.add_draft.submitting);
    assert_eq!(app.add_draft.prompt, "检查项目");
    assert!(app.save_error.as_deref().unwrap().contains("预览"));
}

#[test]
fn wb_schedule_feedback_cancel_keeps_restricted_access() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    let _ = update(&mut app, SettingsMessage::CronAddFullAccessToggled(true));
    assert!(!app.add_draft.full_access, "授权确认前必须保持受限");
    let _ = update(&mut app, SettingsMessage::CronFullAccessCancel { editing: false });
    assert!(!app.add_draft.full_access);
    assert!(!app.add_draft.full_access_confirmation);
    let _ = update(&mut app, SettingsMessage::CronFullAccessConfirm { editing: false });
    assert!(!app.add_draft.full_access);
}

#[test]
fn wb_schedule_feedback_frequency_keeps_restricted_access() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronAddFrequencyChanged(CronFrequency::Weekly));
    assert!(!app.add_draft.full_access);
}

#[test]
fn wb_schedule_feedback_missing_target_rejected() {
    let mut app = app();
    app.add_draft.prompt = "report".into();
    app.add_draft.delivery_enabled = true;
    app.add_draft.delivery_channel = "slack".into();
    let _ = update(&mut app, SettingsMessage::CronAddSubmit);
    assert!(!app.add_draft.submitting);
    assert!(app.save_error.as_deref().unwrap().contains("投递目标"));
}

#[test]
fn wb_schedule_feedback_unsupported_channel_preserved_and_rejected() {
    let mut app = app();
    let mut existing = job("unsupported");
    existing.delivery_mode = "announce".into();
    existing.delivery_channel = Some("legacy-channel".into());
    app.jobs.push(existing);
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted("unsupported".into()));
    assert_eq!(app.edit_draft.delivery_channel, "legacy-channel");
    app.edit_draft.schedule_draft = None;
    let _ = update(&mut app, SettingsMessage::CronJobEditSave);
    assert!(!app.edit_draft.submitting);
    assert!(app.save_error.as_deref().unwrap().contains("不支持"));
}

#[test]
fn wb_schedule_feedback_execution_error_readable_in_history() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronJobRunsOpen("job-1".into()));
    let mut denied = run(1, Some("blocked by security policy: read-only"), Some(1));
    denied.status = "error".into();
    let _ = update(&mut app, SettingsMessage::CronJobRunsLoaded("job-1".into(), Ok(vec![denied])));
    assert!(app.runs_modal_editor.text().contains("blocked by security policy"));
}

#[test]
fn wb_schedule_feedback_confirm_then_cancel_form_does_not_save_authority() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTabSelected(CronSettingsTab::Add));
    let _ = update(&mut app, SettingsMessage::CronAddFullAccessToggled(true));
    let _ = update(&mut app, SettingsMessage::CronFullAccessConfirm { editing: false });
    assert!(app.add_draft.full_access);
    assert!(!app.add_draft.full_access_confirmation);
    let _ = update(&mut app, SettingsMessage::CronAddFrequencyChanged(CronFrequency::Weekly));
    assert!(app.add_draft.full_access);
    let _ = update(&mut app, SettingsMessage::CronJobEditCanceled);
    assert!(!app.add_draft.full_access);
    assert!(app.jobs.is_empty());
    let _ = update(&mut app, SettingsMessage::CronFullAccessConfirm { editing: false });
    assert!(!app.add_draft.full_access);
}

#[test]
fn wb_schedule_feedback_existing_authority_does_not_prompt_again() {
    let mut app = app();
    let mut existing = job("authorized");
    existing.full_access = true;
    app.jobs.push(existing);
    let _ = update(&mut app, SettingsMessage::CronJobEditStarted("authorized".into()));
    assert!(app.edit_draft.full_access);
    assert!(!app.edit_draft.full_access_confirmation);
    let _ = update(&mut app, SettingsMessage::CronJobEditFrequencyChanged(CronFrequency::Weekly));
    assert!(app.edit_draft.full_access);
    let _ = update(&mut app, SettingsMessage::CronJobEditFullAccessToggled(false));
    let _ = update(&mut app, SettingsMessage::CronJobEditFullAccessToggled(true));
    assert!(!app.edit_draft.full_access);
    assert!(app.edit_draft.full_access_confirmation);
    let _ = update(&mut app, SettingsMessage::CronFullAccessCancel { editing: true });
    assert!(!app.edit_draft.full_access);
    let _ = update(&mut app, SettingsMessage::CronJobEditFullAccessToggled(true));
    let _ = update(&mut app, SettingsMessage::CronFullAccessConfirm { editing: true });
    assert!(app.edit_draft.full_access);
    let _ = update(&mut app, SettingsMessage::CronJobEditCanceled);
    assert!(app.jobs[0].full_access);
}

#[test]
fn cron_template_acceptance_edit_failure_retry_preserves_fields() {
    let mut app = app();
    app.jobs = vec![job("acceptance")];
    drop(update(&mut app, SettingsMessage::CronJobEditStarted("acceptance".into())));
    app.edit_draft.name = "中文修改任务".into();
    app.edit_draft.submitting = true;
    drop(update(
        &mut app,
        SettingsMessage::CronEditorSaveCompleted {
            editing: true,
            result: Err("isolated failure".into()),
        },
    ));
    assert_eq!(app.editing_job_id.as_deref(), Some("acceptance"));
    assert_eq!(app.edit_draft.name, "中文修改任务");
    assert!(!app.edit_draft.submitting);
    app.edit_draft.submitting = true;
    drop(update(
        &mut app,
        SettingsMessage::CronEditorSaveCompleted { editing: true, result: Ok("acceptance".into()) },
    ));
    assert!(app.editing_job_id.is_none());
    assert!(app.jobs_loading);
}

#[test]
fn wb_schedule_draft_failure_keeps_rule_and_releases_submission() {
    let mut app = app();
    let rule = crate::state::CronScheduleDraft::default();
    app.add_draft.schedule_draft = Some(rule.clone());
    app.add_draft.submitting = true;
    app.add_draft.submitting = true;
    let _ = update(
        &mut app,
        SettingsMessage::CronEditorSaveCompleted { editing: false, result: Err("offline".into()) },
    );
    assert!(!app.add_draft.submitting);
    assert_eq!(app.add_draft.schedule_draft, Some(rule));
}

#[test]
fn wb_schedule_draft_legacy_edit_preserves_timezone_and_window_patch() {
    let mut app = app();
    let rule = crate::state::CronScheduleDraft {
        timezone: "Asia/Tokyo".into(),
        start_date: Some(None),
        ..Default::default()
    };
    app.edit_draft.schedule_draft = Some(rule);
    let _ = update(&mut app, SettingsMessage::CronJobEditScheduleChanged("30 10 * * *".into()));
    let r = schedule_request(&app.edit_draft).unwrap();
    assert_eq!(r.timezone.as_deref(), Some("Asia/Tokyo"));
    assert_eq!(r.start_date, Some(None));
    assert_eq!(r.schedule.as_deref(), Some("30 10 * * *"));
}

#[test]
fn wb_schedule_draft_invalid_legacy_input_keeps_timezone_when_corrected() {
    let mut app = app();
    let mut rule = crate::state::CronScheduleDraft {
        timezone: "Asia/Tokyo".into(),
        ..Default::default()
    };
    rule.rule = crate::state::CronScheduleRule::Every {
        amount: 2,
        unit: crate::state::IntervalUnit::Hours,
    };
    app.add_draft.schedule_kind = CronAddScheduleKind::Every;
    app.add_draft.schedule_draft = Some(rule);
    let _ = update(&mut app, SettingsMessage::CronAddEveryMsChanged("".into()));
    assert!(schedule_request(&app.add_draft).is_err());
    let _ = update(&mut app, SettingsMessage::CronAddEveryMsChanged("60000".into()));
    let r = schedule_request(&app.add_draft).unwrap();
    assert_eq!(r.timezone.as_deref(), Some("Asia/Tokyo"));
    assert_eq!(r.every_ms, Some(60000));
}

#[test]
fn wb_schedule_picker_existing_absolute_uses_selected_timezone() {
    let mut app = app();
    let mut schedule = crate::state::CronScheduleDraft::default();
    schedule.timezone = "Asia/Shanghai".into();
    schedule.rule = crate::state::CronScheduleRule::AbsoluteAt("2030-12-31T23:35:42Z".into());
    app.add_draft.schedule_draft = Some(schedule);
    let _ = update(&mut app, SettingsMessage::CronPickerOpen { editing: false });
    let picker = app.date_time_picker.as_ref().unwrap();
    assert_eq!(picker.date.to_string(), "2031-01-01");
    assert_eq!(picker.time.to_string(), "07:35:00");
    let _ = update(&mut app, SettingsMessage::CronPickerConfirm);
    let request = schedule_request(&app.add_draft).unwrap();
    assert_eq!(request.timezone.as_deref(), Some("Asia/Shanghai"));
    assert_eq!(request.local_at.as_deref(), Some("2031-01-01T07:35"));
}

#[test]
fn wb_schedule_modal_empty_week_rejected() {
    let mut draft = crate::state::CronJobDraft::default();
    draft.schedule_draft = Some(crate::state::CronScheduleDraft {
        rule: crate::state::CronScheduleRule::Weekly {
            weekdays: vec![],
            time: "09:00:00".parse().unwrap(),
        },
        ..Default::default()
    });
    assert!(schedule_request(&draft).is_err());
}

#[test]
fn wb_schedule_modal_empty_month_rejected() {
    let mut draft = crate::state::CronJobDraft::default();
    draft.schedule_draft = Some(crate::state::CronScheduleDraft {
        rule: crate::state::CronScheduleRule::Monthly {
            days: vec![],
            time: "09:00:00".parse().unwrap(),
        },
        ..Default::default()
    });
    assert!(schedule_request(&draft).is_err());
}

#[test]
fn wb_schedule_modal_unknown_timezone_does_not_submit() {
    let mut app = app();
    app.add_draft.schedule_draft = Some(crate::state::CronScheduleDraft {
        timezone: "Unknown/Nowhere".into(),
        ..Default::default()
    });
    assert!(schedule_request(&app.add_draft).is_err());
}

#[test]
fn cron_template_selection_opens_prefilled_draft_without_saving() {
    let mut app = app();
    let _ = update(&mut app, SettingsMessage::CronTemplatesToggled(true));
    assert!(app.show_cron_templates);
    let _ = update(
        &mut app,
        SettingsMessage::CronTemplateSelected(crate::state::CronTemplateId::AiNews),
    );
    assert!(!app.show_cron_templates);
    assert!(app.show_add_modal);
    assert_eq!(app.add_draft.name, "每日 AI 新闻推送");
    assert!(!app.add_draft.prompt.is_empty());
    assert!(!app.add_draft.submitting);
    assert!(!app.add_draft.full_access);
}

#[test]
fn cron_template_flow_click_prefills_without_creation() {
    for template in crate::state::CRON_TEMPLATES {
        let mut app = app();
        drop(update(&mut app, SettingsMessage::CronTemplateSelected(template.id)));
        assert_eq!(app.add_draft.prompt, template.prompt);
        assert_eq!(app.add_draft.prompt_editor.text(), template.prompt);
        assert!(app.show_add_modal);
        assert!(!app.add_draft.submitting);
        assert!(app.jobs.is_empty());
    }
}

#[test]
fn cron_template_flow_cancel_creates_nothing() {
    let mut app = app();
    drop(update(
        &mut app,
        SettingsMessage::CronTemplateSelected(crate::state::CronTemplateId::AiNews),
    ));
    drop(update(&mut app, SettingsMessage::CronJobEditCanceled));
    assert!(!app.show_add_modal);
    assert!(app.jobs.is_empty());
    assert!(!app.add_draft.submitting);
}

#[test]
fn cron_template_flow_reopen_has_no_leaked_permissions() {
    let mut app = app();
    app.add_draft.full_access = true;
    app.add_draft.delivery_enabled = true;
    app.add_draft.project_path = "/private/old".into();
    drop(update(
        &mut app,
        SettingsMessage::CronTemplateSelected(crate::state::CronTemplateId::AiNews),
    ));
    assert!(!app.add_draft.full_access);
    assert!(!app.add_draft.delivery_enabled);
    assert!(app.add_draft.project_path.is_empty());
}

#[test]
fn cron_template_flow_edited_values_are_submitted() {
    let mut app = app();
    drop(update(
        &mut app,
        SettingsMessage::CronTemplateSelected(crate::state::CronTemplateId::AiNews),
    ));
    for message in [
        SettingsMessage::CronAddNameChanged("我的任务".into()),
        SettingsMessage::CronAddPromptChanged("修改后的正文".into()),
        SettingsMessage::CronAddScheduleChanged("0 11 * * *".into()),
        SettingsMessage::CronAddProjectPathChanged("/tmp/cron-template-project".into()),
    ] {
        drop(update(&mut app, message));
    }
    let revision = app.add_draft.schedule_revision;
    drop(update(
        &mut app,
        SettingsMessage::CronSchedulePreviewLoaded {
            editing: false,
            revision,
            result: Ok(gateway_cron::CronPreviewResponse {
                normalized: Default::default(),
                timezone: "UTC".into(),
                candidates: vec!["2099-01-01T11:00:00Z".into()],
            }),
        },
    ));
    drop(update(&mut app, SettingsMessage::CronAddSubmit));
    let draft = &app.add_draft;
    assert!(draft.submitting);
    assert_eq!(draft.name, "我的任务");
    assert_eq!(draft.prompt, "修改后的正文");
    assert_eq!(draft.project_path, "/tmp/cron-template-project");
    assert_eq!(schedule_request(draft).unwrap().schedule.as_deref(), Some("0 11 * * *"));
}
