//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use desktop_config::config_cron_jobs::{
    add_cron_job_async, delete_cron_job_async, delete_cron_jobs_async, load_cron_job_runs_async,
    set_cron_job_enabled_async, set_cron_jobs_enabled_async,
    update_cron_job_async,
};
use crate::state::{CronAddJobType, CronAddScheduleKind, CronFrequency, CronSettingsTab};
use super::messages::Message;
use desktop_config::ConfigAccess;
use iced::Task;
use iced::widget::text_editor;
use super::cron_support::*;
pub use super::cron_support::schedule_request;

use super::messages::SettingsMessage;

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut Inputs<'_>, message: SettingsMessage) -> Task<Message> {
    if app.cron_settings.add_draft.submitting || app.cron_settings.edit_draft.submitting {
        match &message {
            SettingsMessage::CronEditorSaveCompleted { .. }
            | SettingsMessage::CronJobMutationCompleted(_)
            | SettingsMessage::CronJobsLoaded(_)
            | SettingsMessage::CronSchedulePreviewLoaded { .. } => {}
            _ => return Task::none(),
        }
    }
    match message {
        SettingsMessage::CronFullAccessConfirm { editing } => {
            let state = &mut app.cron_settings;
            let active = if editing {
                state.editing_job_id.is_some()
            } else {
                state.show_add_modal && state.editing_job_id.is_none()
            };
            let draft = if editing { &mut state.edit_draft } else { &mut state.add_draft };
            if active && !draft.submitting && draft.full_access_confirmation {
                draft.full_access = true;
                draft.full_access_confirmation = false;
            }
            Task::none()
        }
        SettingsMessage::CronFullAccessCancel { editing } => {
            let draft = if editing {
                &mut app.cron_settings.edit_draft
            } else {
                &mut app.cron_settings.add_draft
            };
            draft.full_access_confirmation = false;
            Task::none()
        }
        SettingsMessage::CronPickerOpen { editing } => {
            let state = &mut app.cron_settings;
            if state.date_time_picker.is_some()
                || state.show_help_modal
                || state.runs_modal_job_id.is_some()
                || (editing && state.editing_job_id.is_none())
            {
                return Task::none();
            }
            let draft = if editing { &state.edit_draft } else { &state.add_draft };
            if draft.submitting {
                return Task::none();
            }
            let timezone =
                draft.schedule_draft.as_ref().map(|d| d.timezone.as_str()).unwrap_or("UTC");
            let tz = match timezone.parse::<chrono_tz::Tz>() {
                Ok(tz) => tz,
                Err(_) => {
                    state.save_error_translation = None;
                    state.save_error = Some("请选择有效的 IANA 时区后重试".into());
                    return Task::none();
                }
            };
            let now = chrono::Utc::now().with_timezone(&tz);
            let absolute = match draft.schedule_draft.as_ref().map(|d| &d.rule) {
                Some(crate::state::CronScheduleRule::AbsoluteAt(value)) => {
                    Some(value.as_str())
                }
                _ => Some(draft.at.as_str()),
            }
            .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
            .map(|v| v.with_timezone(&tz));
            let (date, time) = match draft.schedule_draft.as_ref().map(|d| &d.rule) {
                Some(crate::state::CronScheduleRule::At { date, time }) => (*date, *time),
                _ => absolute.map(|v| (v.date_naive(), v.time())).unwrap_or((
                    now.date_naive(),
                    chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                )),
            };
            let mut picker =
                crate::system_settings_cron::date_time::DateTimePicker::new(
                    editing, date, time,
                );
            picker.revision = draft.schedule_revision;
            // 时分精度显式归一化，避免绝对时间中的秒在确认后被隐藏保留。
            use chrono::Timelike;
            picker.set_time(time.hour(), time.minute());
            state.date_time_picker = Some(picker);
            Task::none()
        }
        SettingsMessage::CronPickerNavigate(months) => {
            if let Some(picker) = &mut app.cron_settings.date_time_picker {
                picker.navigate(months);
            }
            Task::none()
        }
        SettingsMessage::CronPickerDate(date) => {
            if let Some(picker) = &mut app.cron_settings.date_time_picker {
                picker.select(date);
            }
            Task::none()
        }
        SettingsMessage::CronPickerTime { hour, minute } => {
            if let Some(picker) = &mut app.cron_settings.date_time_picker {
                picker.set_time(hour, minute);
            }
            Task::none()
        }
        SettingsMessage::CronPickerCancel => {
            app.cron_settings.date_time_picker = None;
            Task::none()
        }
        SettingsMessage::CronPickerConfirm => {
            if let Some(picker) = app.cron_settings.date_time_picker.take() {
                let target = if picker.editing {
                    &mut app.cron_settings.edit_draft
                } else {
                    &mut app.cron_settings.add_draft
                };
                if target.submitting || target.schedule_revision != picker.revision {
                    return Task::none();
                }
                let mut schedule = target.schedule_draft.clone().unwrap_or_default();
                schedule.rule = crate::state::CronScheduleRule::At {
                    date: picker.date,
                    time: picker.time,
                };
                target.schedule_kind = CronAddScheduleKind::At;
                // 同步旧表单的展示值，实际请求始终由强类型草稿生成。
                target.at = format!("{}T{}", picker.date, picker.time.format("%H:%M"));
                target.schedule_draft = Some(schedule);
                invalidate_schedule(target);
            }
            Task::none()
        }
        SettingsMessage::CronScheduleDraftChanged { editing, draft } => {
            let target = if editing {
                &mut app.cron_settings.edit_draft
            } else {
                &mut app.cron_settings.add_draft
            };
            if !target.submitting {
                target.schedule_draft = Some(draft);
                invalidate_schedule(target);
            }
            Task::none()
        }
        SettingsMessage::CronSchedulePreview { editing } => {
            let draft = if editing {
                &mut app.cron_settings.edit_draft
            } else {
                &mut app.cron_settings.add_draft
            };
            let revision = draft.schedule_revision;
            match schedule_request(draft) {
                Ok(request) => {
                    draft.preview = None;
                    draft.preview_loading = true;
                    Task::perform(
                        desktop_config::config_cron_jobs::preview_cron_schedule_async(app.client.clone(), request),
                        move |result| {
                            Message::Settings(SettingsMessage::CronSchedulePreviewLoaded {
                                editing,
                                revision,
                                result,
                            })
                        },
                    )
                }
                Err(err) => {
                    draft.preview_loading = false;
                    draft.preview = Some(Err(err));
                    Task::none()
                }
            }
        }
        SettingsMessage::CronSchedulePreviewLoaded { editing, revision, result } => {
            let draft = if editing {
                &mut app.cron_settings.edit_draft
            } else {
                &mut app.cron_settings.add_draft
            };
            if revision == draft.schedule_revision {
                draft.preview_loading = false;
                draft.preview = Some(result);
            }
            Task::none()
        }

        SettingsMessage::CronEnabledToggled(v) => {
            app.cron_settings.enabled = v;
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            persist_cron_settings(app)
        }
        SettingsMessage::CronMaxRunHistoryChanged(v) => {
            app.cron_settings.max_run_history = v.clamp(1, 10_000);
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            persist_cron_settings(app)
        }
        SettingsMessage::CronTemplatesToggled(open) => {
            if !app.cron_settings.add_draft.submitting && !app.cron_settings.edit_draft.submitting {
                app.cron_settings.show_cron_templates = open;
            }
            Task::none()
        }
        SettingsMessage::CronTemplateSelected(id) => {
            if app.cron_settings.add_draft.submitting || app.cron_settings.edit_draft.submitting {
                return Task::none();
            }
            let Some(template) =
                crate::state::CRON_TEMPLATES.iter().find(|template| template.id == id)
            else {
                return app.show_error_toast("未找到常用任务模板");
            };
            let mut draft = match template.draft(chrono::Local::now()) {
                Ok(draft) => draft,
                Err(error) => return app.show_error_toast(error),
            };
            draft.schedule_draft = match schedule_request(&draft)
                .and_then(|request| crate::state::CronScheduleDraft::from_request(&request))
            {
                Ok(schedule) => Some(schedule),
                Err(error) => return app.show_error_toast(error),
            };
            draft.schedule_revision = app.cron_settings.add_draft.schedule_revision + 1;
            app.cron_settings.add_draft = draft;
            app.cron_settings.show_add_modal = true;
            app.cron_settings.editing_job_id = None;
            app.cron_settings.date_time_picker = None;
            app.cron_settings.active_tab = CronSettingsTab::Jobs;
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            app.cron_settings.action_status = None;
            app.cron_settings.show_cron_templates = false;
            Task::none()
        }
        SettingsMessage::CronTabSelected(tab) => {
            app.cron_settings.date_time_picker = None;
            if app.cron_settings.add_draft.submitting || app.cron_settings.edit_draft.submitting {
                return Task::none();
            }
            if tab == CronSettingsTab::Add {
                app.cron_settings.show_add_modal = true;
                app.cron_settings.editing_job_id = None;
                app.cron_settings.active_tab = CronSettingsTab::Jobs;
                let revision = app.cron_settings.add_draft.schedule_revision + 1;
                app.cron_settings.add_draft = Default::default();
                app.cron_settings.add_draft.schedule_revision = revision;
                app.cron_settings.add_draft.schedule_draft = Some(Default::default());
            } else {
                app.cron_settings.active_tab = tab;
                app.cron_settings.show_add_modal = false;
            }
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            app.cron_settings.action_status = None;
            if tab == CronSettingsTab::Jobs {
                app.cron_settings.jobs_loading = true;
                refresh_cron_jobs_task(app)
            } else {
                Task::none()
            }
        }
        SettingsMessage::CronListFilterChanged(filter) => {
            if app.cron_settings.list_filter != filter {
                app.cron_settings.list_filter = filter;
                app.cron_settings.selected_job_ids.clear();
            }
            Task::none()
        }
        SettingsMessage::CronBatchModeToggled(enabled) => {
            app.cron_settings.batch_mode = enabled;
            if !enabled {
                app.cron_settings.selected_job_ids.clear();
            }
            Task::none()
        }
        SettingsMessage::CronJobsRefresh => {
            app.cron_settings.jobs_loading = true;
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            app.cron_settings.action_status = None;
            refresh_cron_jobs_task(app)
        }
        SettingsMessage::CronJobsLoaded(result) => {
            app.cron_settings.jobs_loading = false;
            match result {
                Ok(jobs) => {
                    app.cron_settings.jobs = jobs;
                    app.cron_settings.save_error = None;
                    app.cron_settings.save_error_translation = None;
                    app.cron_settings.prune_selected_jobs();
                    app.cron_settings
                        .expanded_job_ids
                        .retain(|id| app.cron_settings.jobs.iter().any(|job| job.id == *id));
                }
                Err(err) => {
                    app.cron_settings.save_error_translation = None;
                    app.cron_settings.save_error = Some(err);
                }
            }
            Task::none()
        }
        SettingsMessage::CronJobSelectionToggled(job_id, selected) => {
            if selected {
                if app.cron_settings.visible_jobs().any(|job| job.id == job_id)
                    && !app.cron_settings.selected_job_ids.iter().any(|id| id == &job_id)
                {
                    app.cron_settings.selected_job_ids.push(job_id);
                }
            } else {
                app.cron_settings.selected_job_ids.retain(|id| id != &job_id);
            }
            Task::none()
        }
        SettingsMessage::CronJobDetailsToggled(job_id, expanded) => {
            if expanded {
                if !app.cron_settings.expanded_job_ids.iter().any(|id| id == &job_id) {
                    app.cron_settings.expanded_job_ids.push(job_id);
                }
            } else {
                app.cron_settings.expanded_job_ids.retain(|id| id != &job_id);
            }
            Task::none()
        }
        SettingsMessage::CronJobsSelectAllToggled(selected) => {
            app.cron_settings.selected_job_ids = if selected {
                let mut ids =
                    Vec::with_capacity(app.cron_settings.job_count(app.cron_settings.list_filter));
                ids.extend(app.cron_settings.visible_jobs().map(|job| job.id.clone()));
                ids
            } else {
                Vec::new()
            };
            Task::none()
        }
        SettingsMessage::CronJobRunsOpen(job_id) => {
            app.cron_settings.runs_modal_job_id = Some(job_id.clone());
            app.cron_settings.runs_modal_loading = true;
            app.cron_settings.runs_modal_error = None;
            app.cron_settings.runs_modal.clear();
            app.cron_settings.runs_modal_editor = text_editor::Content::new();
            Task::perform(load_cron_job_runs_async(app.client.clone(), job_id.clone()), move |result| {
                Message::Settings(SettingsMessage::CronJobRunsLoaded(job_id, result))
            })
        }
        SettingsMessage::CronJobRunsLoaded(job_id, result) => {
            if app.cron_settings.runs_modal_job_id.as_deref() != Some(job_id.as_str()) {
                return Task::none();
            }
            app.cron_settings.runs_modal_loading = false;
            match result {
                Ok(runs) => {
                    let history_text = format_run_history_text(app.desktop_locale, &runs);
                    app.cron_settings.runs_modal = runs;
                    app.cron_settings.runs_modal_editor =
                        text_editor::Content::with_text(&history_text);
                    app.cron_settings.runs_modal_error = None;
                }
                Err(err) => {
                    app.cron_settings.runs_modal.clear();
                    app.cron_settings.runs_modal_editor = text_editor::Content::new();
                    app.cron_settings.runs_modal_error = Some(err);
                }
            }
            Task::none()
        }
        SettingsMessage::CronJobRunsEditorAction(action) => {
            match action {
                text_editor::Action::Edit(_) => {}
                action => app.cron_settings.runs_modal_editor.perform(action),
            }
            Task::none()
        }
        SettingsMessage::CronJobRunsClose => {
            app.cron_settings.runs_modal_job_id = None;
            app.cron_settings.runs_modal_loading = false;
            app.cron_settings.runs_modal_error = None;
            app.cron_settings.runs_modal.clear();
            app.cron_settings.runs_modal_editor = text_editor::Content::new();
            Task::none()
        }
        SettingsMessage::CronJobEditStarted(job_id) => {
            if let Some(job) = app.cron_settings.jobs.iter().find(|job| job.id == job_id) {
                match crate::state::CronScheduleDraft::from_job(job) {
                    Ok(rule) => app.cron_settings.edit_draft.schedule_draft = Some(rule),
                    Err(err) => {
                        app.cron_settings.save_error = Some(err);
                        return Task::none();
                    }
                }
                app.cron_settings.show_add_modal = false;
                app.cron_settings.editing_job_id = Some(job.id.clone());
                invalidate_schedule(&mut app.cron_settings.edit_draft);
                app.cron_settings.edit_draft.name = job.name.clone().unwrap_or_default();
                app.cron_settings.edit_draft.job_type = job_type_from_api(job);
                app.cron_settings.edit_draft.schedule_kind = schedule_kind_from_api(
                    &job.schedule_kind,
                    &job.expression,
                    job.at.as_deref(),
                    job.every_ms,
                );
                app.cron_settings.edit_draft.schedule = job.expression.clone();
                app.cron_settings.edit_draft.frequency = if matches!(
                    app.cron_settings.edit_draft.schedule_draft.as_ref().map(|d| &d.rule),
                    Some(crate::state::CronScheduleRule::Advanced(_))
                ) {
                    CronFrequency::Advanced
                } else {
                    CronFrequency::from_expression(&job.expression)
                };
                app.cron_settings.edit_draft.at = job.at.clone().unwrap_or_default();
                app.cron_settings.edit_draft.every_ms =
                    job.every_ms.map(|value| value.to_string()).unwrap_or_default();
                app.cron_settings.edit_draft.command = job.command.clone();
                app.cron_settings.edit_draft.command_editor =
                    text_editor::Content::with_text(&app.cron_settings.edit_draft.command);
                app.cron_settings.edit_draft.prompt = job.prompt.clone().unwrap_or_default();
                app.cron_settings.edit_draft.prompt_editor =
                    text_editor::Content::with_text(&app.cron_settings.edit_draft.prompt);
                app.cron_settings.edit_draft.session_target = "isolated".to_string();
                app.cron_settings.edit_draft.agent =
                    job.agent.clone().unwrap_or_else(|| "main".to_string());
                app.cron_settings.edit_draft.acp_agent = job.acp_agent.clone().unwrap_or_default();
                app.cron_settings.edit_draft.project_path =
                    job.project_path.clone().unwrap_or_default();
                app.cron_settings.edit_draft.model = job.model.clone().unwrap_or_default();
                app.cron_settings.edit_draft.model_provider =
                    model_provider_from_model(&app.cron_settings.edit_draft.model);
                app.cron_settings.edit_draft.wake = job.wake;
                app.cron_settings.edit_draft.fallbacks = job.fallbacks.join("\n");
                app.cron_settings.edit_draft.full_access = job.full_access;
                app.cron_settings.edit_draft.full_access_confirmation = false;
                app.cron_settings.edit_draft.task_pool = job.task_pool;
                app.cron_settings.edit_draft.delivery_enabled = job.delivery_mode == "announce";
                app.cron_settings.edit_draft.delivery_channel =
                    job.delivery_channel.clone().unwrap_or_default();
                app.cron_settings.edit_draft.delivery_to =
                    job.delivery_to.clone().unwrap_or_default();
                app.cron_settings.edit_draft.delivery_best_effort = job.delivery_best_effort;
                app.cron_settings.edit_draft.delete_after_run = job.delete_after_run;
                app.cron_settings.save_error = None;
                app.cron_settings.save_error_translation = None;
                app.cron_settings.action_status = None;
            }
            Task::none()
        }
        SettingsMessage::CronJobEditCanceled => {
            if app.cron_settings.add_draft.submitting || app.cron_settings.edit_draft.submitting {
                return Task::none();
            }
            app.cron_settings.show_add_modal = false;
            let add_revision = app.cron_settings.add_draft.schedule_revision + 1;
            app.cron_settings.add_draft = Default::default();
            app.cron_settings.add_draft.schedule_revision = add_revision;
            app.cron_settings.date_time_picker = None;
            app.cron_settings.editing_job_id = None;
            let revision = app.cron_settings.edit_draft.schedule_revision + 1;
            app.cron_settings.edit_draft = Default::default();
            app.cron_settings.edit_draft.schedule_revision = revision;
            Task::none()
        }
        SettingsMessage::CronJobEditNameChanged(value) => {
            app.cron_settings.edit_draft.name = value;
            Task::none()
        }
        SettingsMessage::CronJobEditJobTypeChanged(value) => {
            app.cron_settings.edit_draft.job_type = value;
            Task::none()
        }
        SettingsMessage::CronJobEditScheduleKindChanged(value) => {
            if app.cron_settings.edit_draft.submitting {
                return Task::none();
            }
            invalidate_schedule(&mut app.cron_settings.edit_draft);
            let previous_schedule = app.cron_settings.edit_draft.schedule_draft.take();
            app.cron_settings.edit_draft.schedule_kind = value;
            match value {
                CronAddScheduleKind::Cron => {
                    if app.cron_settings.edit_draft.schedule.is_empty() {
                        app.cron_settings.edit_draft.frequency = CronFrequency::Daily;
                        app.cron_settings.edit_draft.schedule =
                            CronFrequency::Daily.default_expression().to_string();
                    }
                    app.cron_settings.edit_draft.at.clear();
                    app.cron_settings.edit_draft.every_ms.clear();
                }
                CronAddScheduleKind::At => {
                    app.cron_settings.edit_draft.schedule.clear();
                    app.cron_settings.edit_draft.every_ms.clear();
                }
                CronAddScheduleKind::Every => {
                    app.cron_settings.edit_draft.schedule.clear();
                    app.cron_settings.edit_draft.at.clear();
                }
            }
            restore_schedule_metadata(&mut app.cron_settings.edit_draft, previous_schedule);
            Task::none()
        }
        SettingsMessage::CronJobEditFrequencyChanged(value) => {
            if app.cron_settings.edit_draft.submitting {
                return Task::none();
            }
            invalidate_schedule(&mut app.cron_settings.edit_draft);
            let previous_schedule = app.cron_settings.edit_draft.schedule_draft.take();
            let draft = &mut app.cron_settings.edit_draft;
            draft.frequency = value;
            if value != CronFrequency::Advanced {
                draft.schedule = value.default_expression().to_string();
            }
            restore_schedule_metadata(&mut app.cron_settings.edit_draft, previous_schedule);
            Task::none()
        }
        SettingsMessage::CronJobEditScheduleChanged(value) => {
            if app.cron_settings.edit_draft.submitting {
                return Task::none();
            }
            invalidate_schedule(&mut app.cron_settings.edit_draft);
            let previous_schedule = app.cron_settings.edit_draft.schedule_draft.take();
            app.cron_settings.edit_draft.schedule = value;
            restore_schedule_metadata(&mut app.cron_settings.edit_draft, previous_schedule);
            Task::none()
        }
        SettingsMessage::CronJobEditAtChanged(value) => {
            if app.cron_settings.edit_draft.submitting {
                return Task::none();
            }
            invalidate_schedule(&mut app.cron_settings.edit_draft);
            let previous_schedule = app.cron_settings.edit_draft.schedule_draft.take();
            app.cron_settings.edit_draft.at = value;
            restore_schedule_metadata(&mut app.cron_settings.edit_draft, previous_schedule);
            Task::none()
        }
        SettingsMessage::CronJobEditEveryMsChanged(value) => {
            if app.cron_settings.edit_draft.submitting {
                return Task::none();
            }
            invalidate_schedule(&mut app.cron_settings.edit_draft);
            let previous_schedule = app.cron_settings.edit_draft.schedule_draft.take();
            app.cron_settings.edit_draft.every_ms = value;
            restore_schedule_metadata(&mut app.cron_settings.edit_draft, previous_schedule);
            Task::none()
        }
        SettingsMessage::CronJobEditCommandChanged(value) => {
            app.cron_settings.edit_draft.command = value.clone();
            app.cron_settings.edit_draft.command_editor = text_editor::Content::with_text(&value);
            Task::none()
        }
        SettingsMessage::CronJobEditCommandEditorAction(action) => {
            app.cron_settings.edit_draft.command_editor.perform(action);
            app.cron_settings.edit_draft.command =
                app.cron_settings.edit_draft.command_editor.text();
            Task::none()
        }
        SettingsMessage::CronJobEditPromptChanged(value) => {
            app.cron_settings.edit_draft.prompt = value.clone();
            app.cron_settings.edit_draft.prompt_editor = text_editor::Content::with_text(&value);
            Task::none()
        }
        SettingsMessage::CronJobEditPromptEditorAction(action) => {
            app.cron_settings.edit_draft.prompt_editor.perform(action);
            app.cron_settings.edit_draft.prompt = app.cron_settings.edit_draft.prompt_editor.text();
            Task::none()
        }
        SettingsMessage::CronJobEditAgentChanged(value) => {
            app.cron_settings.edit_draft.agent = value;
            Task::none()
        }
        SettingsMessage::CronJobEditAcpAgentChanged(value) => {
            app.cron_settings.edit_draft.acp_agent = value;
            Task::none()
        }
        SettingsMessage::CronJobEditProjectPathChanged(value) => {
            app.cron_settings.edit_draft.project_path = value;
            Task::none()
        }
        SettingsMessage::CronJobEditModelProviderChanged(value) => {
            let draft = &mut app.cron_settings.edit_draft;
            if draft.model_provider != value {
                draft.model.clear();
                draft.model_provider = value;
            }
            Task::none()
        }
        SettingsMessage::CronJobEditModelChanged(value) => {
            let draft = &mut app.cron_settings.edit_draft;
            draft.model_provider = model_provider_from_model(&value);
            draft.model = value;
            Task::none()
        }
        SettingsMessage::CronJobEditWakeToggled(value) => {
            app.cron_settings.edit_draft.wake = value;
            Task::none()
        }
        SettingsMessage::CronJobEditFallbacksChanged(value) => {
            app.cron_settings.edit_draft.fallbacks = value;
            Task::none()
        }
        SettingsMessage::CronJobEditFullAccessToggled(value) => {
            let draft = &mut app.cron_settings.edit_draft;
            if !draft.submitting {
                draft.full_access_confirmation = value && !draft.full_access;
                if !value {
                    draft.full_access = false;
                }
            }
            Task::none()
        }
        SettingsMessage::CronJobEditTaskPoolToggled(value) => {
            app.cron_settings.edit_draft.task_pool = value;
            Task::none()
        }
        SettingsMessage::CronJobEditDeliveryEnabledToggled(value) => {
            app.cron_settings.edit_draft.delivery_enabled = value;
            Task::none()
        }
        SettingsMessage::CronJobEditDeliveryChannelChanged(value) => {
            app.cron_settings.edit_draft.select_delivery_channel(value);
            Task::none()
        }
        SettingsMessage::CronJobEditDeliveryToChanged(value) => {
            app.cron_settings.edit_draft.delivery_to = value;
            Task::none()
        }
        SettingsMessage::CronJobEditDeliveryBestEffortToggled(value) => {
            app.cron_settings.edit_draft.delivery_best_effort = value;
            Task::none()
        }
        SettingsMessage::CronJobEditDeleteAfterRunToggled(value) => {
            app.cron_settings.edit_draft.delete_after_run = value;
            Task::none()
        }
        SettingsMessage::CronJobEditSave => {
            if app.cron_settings.add_draft.submitting || app.cron_settings.edit_draft.submitting {
                return Task::none();
            }
            let request = match schedule_request(&app.cron_settings.edit_draft) {
                Ok(request) => request,
                Err(err) => {
                    app.cron_settings.save_error_translation = None;
                    app.cron_settings.save_error = Some(err.clone());
                    return app.show_error_toast(err);
                }
            };

            let Some(job_id) = app.cron_settings.editing_job_id.clone() else {
                app.cron_settings.save_error_translation =
                    Some(desktop_i18n::format::Message::new("没有正在编辑的定时任务", vec![]));
                app.cron_settings.save_error = Some("没有正在编辑的定时任务".to_string());
                return Task::none();
            };
            if let Err(error) = validate_delivery(app, &app.cron_settings.edit_draft)
                .and_then(|()| validate_modal_payload(&app.cron_settings.edit_draft))
            {
                app.cron_settings.save_error = Some(error.clone());
                app.cron_settings.save_error_translation = None;
                return app.show_error_toast(error);
            }
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            app.cron_settings.edit_draft.submitting = true;
            cron_editor_save_task(
                true,
                "定时任务已更新",
                update_cron_job_async(app.client.clone(),
                    job_id,
                    app.cron_settings.edit_draft.name.clone(),
                    app.cron_settings.edit_draft.job_type.as_api_value().to_string(),
                    app.cron_settings.edit_draft.schedule_kind.as_api_value().to_string(),
                    app.cron_settings.edit_draft.schedule.clone(),
                    app.cron_settings.edit_draft.at.clone(),
                    app.cron_settings.edit_draft.every_ms.clone(),
                    app.cron_settings.edit_draft.command.clone(),
                    app.cron_settings.edit_draft.prompt.clone(),
                    "isolated".to_string(),
                    app.cron_settings.edit_draft.agent.clone(),
                    app.cron_settings.edit_draft.acp_agent.clone(),
                    app.cron_settings.edit_draft.project_path.clone(),
                    app.cron_settings.edit_draft.wake,
                    app.cron_settings.edit_draft.model.clone(),
                    app.cron_settings.edit_draft.fallbacks.clone(),
                    app.cron_settings.edit_draft.full_access,
                    app.cron_settings.edit_draft.task_pool,
                    app.cron_settings.edit_draft.delivery_enabled,
                    app.cron_settings.edit_draft.delivery_channel.clone(),
                    app.cron_settings.edit_draft.delivery_to.clone(),
                    app.cron_settings.edit_draft.delivery_best_effort,
                    app.cron_settings.edit_draft.delete_after_run,
                    Some(request),
                ),
            )
        }
        SettingsMessage::CronJobEnabledChanged(job_id, enabled) => {
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            cron_mutation_task(
                if enabled { "定时任务已启用" } else { "定时任务已禁用" },
                set_cron_job_enabled_async(app.client.clone(), job_id, enabled),
            )
        }
        SettingsMessage::CronJobDelete(job_id) => {
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            cron_mutation_task("定时任务已删除", delete_cron_job_async(app.client.clone(), job_id))
        }
        SettingsMessage::CronSelectedJobsEnable => {
            app.cron_settings.prune_selected_jobs();
            let job_ids = app.cron_settings.selected_job_ids.clone();
            if job_ids.is_empty() {
                app.cron_settings.save_error_translation = Some(
                    desktop_i18n::format::Message::new("请先选择要启用的定时任务", vec![]),
                );
                app.cron_settings.save_error = Some("请先选择要启用的定时任务".to_string());
                return Task::none();
            }
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            cron_mutation_task("选中的定时任务已启用", set_cron_jobs_enabled_async(app.client.clone(), job_ids, true))
        }
        SettingsMessage::CronSelectedJobsDisable => {
            app.cron_settings.prune_selected_jobs();
            let job_ids = app.cron_settings.selected_job_ids.clone();
            if job_ids.is_empty() {
                app.cron_settings.save_error_translation = Some(
                    desktop_i18n::format::Message::new("请先选择要禁用的定时任务", vec![]),
                );
                app.cron_settings.save_error = Some("请先选择要禁用的定时任务".to_string());
                return Task::none();
            }
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            cron_mutation_task("选中的定时任务已禁用", set_cron_jobs_enabled_async(app.client.clone(), job_ids, false))
        }
        SettingsMessage::CronSelectedJobsDelete => {
            app.cron_settings.prune_selected_jobs();
            let job_ids = app.cron_settings.selected_job_ids.clone();
            if job_ids.is_empty() {
                app.cron_settings.save_error_translation = Some(
                    desktop_i18n::format::Message::new("请先选择要删除的定时任务", vec![]),
                );
                app.cron_settings.save_error = Some("请先选择要删除的定时任务".to_string());
                return Task::none();
            }
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            cron_mutation_task("选中的定时任务已删除", delete_cron_jobs_async(app.client.clone(), job_ids))
        }
        SettingsMessage::CronAddNameChanged(value) => {
            app.cron_settings.add_draft.name = value;
            Task::none()
        }
        SettingsMessage::CronAddJobTypeChanged(value) => {
            app.cron_settings.add_draft.job_type = value;
            Task::none()
        }
        SettingsMessage::CronAddScheduleKindChanged(value) => {
            if app.cron_settings.add_draft.submitting {
                return Task::none();
            }
            invalidate_schedule(&mut app.cron_settings.add_draft);
            let previous_schedule = app.cron_settings.add_draft.schedule_draft.take();
            app.cron_settings.add_draft.schedule_kind = value;
            match value {
                CronAddScheduleKind::Cron => {
                    if app.cron_settings.add_draft.schedule.is_empty() {
                        app.cron_settings.add_draft.frequency = CronFrequency::Daily;
                        app.cron_settings.add_draft.schedule =
                            CronFrequency::Daily.default_expression().to_string();
                    }
                    app.cron_settings.add_draft.at.clear();
                    app.cron_settings.add_draft.every_ms.clear();
                }
                CronAddScheduleKind::At => {
                    app.cron_settings.add_draft.schedule.clear();
                    app.cron_settings.add_draft.every_ms.clear();
                }
                CronAddScheduleKind::Every => {
                    app.cron_settings.add_draft.schedule.clear();
                    app.cron_settings.add_draft.at.clear();
                }
            }
            restore_schedule_metadata(&mut app.cron_settings.add_draft, previous_schedule);
            Task::none()
        }
        SettingsMessage::CronAddFrequencyChanged(value) => {
            if app.cron_settings.add_draft.submitting {
                return Task::none();
            }
            invalidate_schedule(&mut app.cron_settings.add_draft);
            let previous_schedule = app.cron_settings.add_draft.schedule_draft.take();
            let draft = &mut app.cron_settings.add_draft;
            draft.frequency = value;
            if value != CronFrequency::Advanced {
                draft.schedule = value.default_expression().to_string();
            }
            restore_schedule_metadata(&mut app.cron_settings.add_draft, previous_schedule);
            Task::none()
        }
        SettingsMessage::CronAddScheduleChanged(value) => {
            if app.cron_settings.add_draft.submitting {
                return Task::none();
            }
            invalidate_schedule(&mut app.cron_settings.add_draft);
            let previous_schedule = app.cron_settings.add_draft.schedule_draft.take();
            app.cron_settings.add_draft.schedule = value;
            restore_schedule_metadata(&mut app.cron_settings.add_draft, previous_schedule);
            Task::none()
        }
        SettingsMessage::CronAddAtChanged(value) => {
            if app.cron_settings.add_draft.submitting {
                return Task::none();
            }
            invalidate_schedule(&mut app.cron_settings.add_draft);
            let previous_schedule = app.cron_settings.add_draft.schedule_draft.take();
            app.cron_settings.add_draft.at = value;
            restore_schedule_metadata(&mut app.cron_settings.add_draft, previous_schedule);
            Task::none()
        }
        SettingsMessage::CronAddEveryMsChanged(value) => {
            if app.cron_settings.add_draft.submitting {
                return Task::none();
            }
            invalidate_schedule(&mut app.cron_settings.add_draft);
            let previous_schedule = app.cron_settings.add_draft.schedule_draft.take();
            app.cron_settings.add_draft.every_ms = value;
            restore_schedule_metadata(&mut app.cron_settings.add_draft, previous_schedule);
            Task::none()
        }
        SettingsMessage::CronAddCommandChanged(value) => {
            app.cron_settings.add_draft.command = value.clone();
            app.cron_settings.add_draft.command_editor = text_editor::Content::with_text(&value);
            Task::none()
        }
        SettingsMessage::CronAddCommandEditorAction(action) => {
            app.cron_settings.add_draft.command_editor.perform(action);
            app.cron_settings.add_draft.command = app.cron_settings.add_draft.command_editor.text();
            Task::none()
        }
        SettingsMessage::CronAddPromptChanged(value) => {
            app.cron_settings.add_draft.prompt = value.clone();
            app.cron_settings.add_draft.prompt_editor = text_editor::Content::with_text(&value);
            Task::none()
        }
        SettingsMessage::CronAddPromptEditorAction(action) => {
            app.cron_settings.add_draft.prompt_editor.perform(action);
            app.cron_settings.add_draft.prompt = app.cron_settings.add_draft.prompt_editor.text();
            Task::none()
        }
        SettingsMessage::CronAddSessionTargetChanged(value) => {
            app.cron_settings.add_draft.session_target = value;
            Task::none()
        }
        SettingsMessage::CronAddAgentChanged(value) => {
            app.cron_settings.add_draft.agent = value;
            Task::none()
        }
        SettingsMessage::CronAddAcpAgentChanged(value) => {
            app.cron_settings.add_draft.acp_agent = value;
            Task::none()
        }
        SettingsMessage::CronAddProjectPathChanged(value) => {
            app.cron_settings.add_draft.project_path = value;
            Task::none()
        }
        SettingsMessage::CronAddModelProviderChanged(value) => {
            let draft = &mut app.cron_settings.add_draft;
            if draft.model_provider != value {
                draft.model.clear();
                draft.model_provider = value;
            }
            Task::none()
        }
        SettingsMessage::CronAddModelChanged(value) => {
            let draft = &mut app.cron_settings.add_draft;
            draft.model_provider = model_provider_from_model(&value);
            draft.model = value;
            Task::none()
        }
        SettingsMessage::CronAddWakeToggled(value) => {
            app.cron_settings.add_draft.wake = value;
            Task::none()
        }
        SettingsMessage::CronAddFallbacksChanged(value) => {
            app.cron_settings.add_draft.fallbacks = value;
            Task::none()
        }
        SettingsMessage::CronAddFullAccessToggled(value) => {
            let draft = &mut app.cron_settings.add_draft;
            if !draft.submitting {
                draft.full_access_confirmation = value && !draft.full_access;
                if !value {
                    draft.full_access = false;
                }
            }
            Task::none()
        }
        SettingsMessage::CronAddTaskPoolToggled(value) => {
            app.cron_settings.add_draft.task_pool = value;
            Task::none()
        }
        SettingsMessage::CronAddDeliveryEnabledToggled(value) => {
            app.cron_settings.add_draft.delivery_enabled = value;
            Task::none()
        }
        SettingsMessage::CronAddDeliveryChannelChanged(value) => {
            app.cron_settings.add_draft.select_delivery_channel(value);
            Task::none()
        }
        SettingsMessage::CronAddDeliveryToChanged(value) => {
            app.cron_settings.add_draft.delivery_to = value;
            Task::none()
        }
        SettingsMessage::CronAddDeliveryBestEffortToggled(value) => {
            app.cron_settings.add_draft.delivery_best_effort = value;
            Task::none()
        }
        SettingsMessage::CronAddDeleteAfterRunToggled(value) => {
            app.cron_settings.add_draft.delete_after_run = value;
            Task::none()
        }
        SettingsMessage::CronAddSubmit => {
            if app.cron_settings.add_draft.submitting || app.cron_settings.edit_draft.submitting {
                return Task::none();
            }
            let request = match schedule_request(&app.cron_settings.add_draft) {
                Ok(request) => request,
                Err(err) => {
                    app.cron_settings.save_error_translation = None;
                    app.cron_settings.save_error = Some(err.clone());
                    return app.show_error_toast(err);
                }
            };

            if let Err(error) = validate_delivery(app, &app.cron_settings.add_draft)
                .and_then(|()| validate_modal_payload(&app.cron_settings.add_draft))
            {
                app.cron_settings.save_error = Some(error.clone());
                app.cron_settings.save_error_translation = None;
                return app.show_error_toast(error);
            }
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            app.cron_settings.add_draft.submitting = true;
            cron_editor_save_task(
                false,
                "定时任务已新增",
                add_cron_job_async(app.client.clone(),
                    app.cron_settings.add_draft.name.clone(),
                    app.cron_settings.add_draft.job_type.as_api_value().to_string(),
                    app.cron_settings.add_draft.schedule_kind.as_api_value().to_string(),
                    app.cron_settings.add_draft.schedule.clone(),
                    app.cron_settings.add_draft.at.clone(),
                    app.cron_settings.add_draft.every_ms.clone(),
                    app.cron_settings.add_draft.command.clone(),
                    app.cron_settings.add_draft.prompt.clone(),
                    "isolated".to_string(),
                    app.cron_settings.add_draft.agent.clone(),
                    app.cron_settings.add_draft.acp_agent.clone(),
                    app.cron_settings.add_draft.project_path.clone(),
                    app.cron_settings.add_draft.wake,
                    app.cron_settings.add_draft.model.clone(),
                    app.cron_settings.add_draft.fallbacks.clone(),
                    app.cron_settings.add_draft.full_access,
                    app.cron_settings.add_draft.task_pool,
                    app.cron_settings.add_draft.delivery_enabled,
                    app.cron_settings.add_draft.delivery_channel.clone(),
                    app.cron_settings.add_draft.delivery_to.clone(),
                    app.cron_settings.add_draft.delivery_best_effort,
                    app.cron_settings.add_draft.delete_after_run,
                    Some(request),
                ),
            )
        }
        SettingsMessage::CronEditorSaveCompleted { editing, result } => {
            let draft = if editing {
                &mut app.cron_settings.edit_draft
            } else {
                &mut app.cron_settings.add_draft
            };
            if !draft.submitting {
                return Task::none();
            }
            draft.submitting = false;
            match result {
                Ok(message) => {
                    let toast = app.show_success_toast(message.clone());
                    app.cron_settings.show_add_modal = false;
                    app.cron_settings.action_status = Some(message);
                    app.cron_settings.save_error = None;
                    app.cron_settings.save_error_translation = None;
                    app.cron_settings.editing_job_id = None;
                    let edit_revision = app.cron_settings.edit_draft.schedule_revision + 1;
                    let add_revision = app.cron_settings.add_draft.schedule_revision + 1;
                    app.cron_settings.edit_draft = Default::default();
                    app.cron_settings.add_draft = Default::default();
                    app.cron_settings.edit_draft.schedule_revision = edit_revision;
                    app.cron_settings.add_draft.schedule_revision = add_revision;
                    app.cron_settings.jobs_loading = true;
                    Task::batch([refresh_cron_jobs_task(app), toast])
                }
                Err(err) => {
                    app.cron_settings.save_error_translation = None;
                    app.cron_settings.save_error = Some(err.clone());
                    app.show_error_toast(err)
                }
            }
        }
        SettingsMessage::CronJobMutationCompleted(result) => match result {
            Ok(message) => {
                app.cron_settings.action_status = Some(message.clone());
                app.cron_settings.jobs_loading = true;
                Task::batch([refresh_cron_jobs_task(app), app.show_success_toast(message)])
            }
            Err(error) => app.show_error_toast(error),
        },
        SettingsMessage::CronSave => persist_cron_settings(app),
        SettingsMessage::CronHelpOpen => {
            app.cron_settings.show_help_modal = true;
            Task::none()
        }
        SettingsMessage::CronHelpClose => {
            app.cron_settings.show_help_modal = false;
            Task::none()
        }
        _ => Task::none(),
    }
}

pub fn refresh_run_history_locale(state: &mut crate::state::CronSettingsState, locale: config_ui::DesktopLocale) {
 let text = format_run_history_text(locale, &state.runs_modal);
 state.runs_modal_editor = text_editor::Content::with_text(&text);
}

fn validate_modal_payload(draft: &crate::state::CronJobDraft) -> Result<(), String> {
    if draft.full_access_confirmation {
        return Err("请先确认或取消完全访问授权".into());
    }
    match draft.job_type {
        CronAddJobType::Shell if draft.command.trim().is_empty() => {
            return Err("请输入执行命令".into());
        }
        CronAddJobType::Agent if draft.prompt.trim().is_empty() => {
            return Err("请输入 Agent 提示词".into());
        }
        _ => {}
    }
    if draft.schedule_draft.is_some()
        && !matches!(&draft.preview, Some(Ok(p)) if !p.candidates.is_empty())
    {
        return Err("请先预览当前规则并确认存在未来触发时间".into());
    }
    Ok(())
}

/// 仅校验已知配置；凭证有效性与最终发送结果仍由执行器判定。
pub fn validate_delivery(
    app: &Inputs<'_>,
    draft: &crate::state::CronJobDraft,
) -> Result<(), String> {
    crate::delivery::validate(
        draft,
        app.delivery,
    )
}

pub struct Inputs<'a> {
pub cron_settings: &'a mut crate::state::CronSettingsState,
 pub desktop_locale: config_ui::DesktopLocale,
 pub delivery: crate::system_settings_cron::DeliveryAvailability,
 pub client: Result<vw_gateway_client::GatewayClient, String>,
 pub notices: &'a mut Vec<Notice>,
 pub config: &'a ConfigAccess,
}
pub enum Notice { Error(String), Success(String) }
impl Inputs<'_> {
 pub(super) fn show_error_toast(&mut self, text: impl Into<String>) -> Task<Message> { self.notices.push(Notice::Error(text.into())); Task::none() }
 fn show_success_toast(&mut self, text: impl Into<String>) -> Task<Message> { self.notices.push(Notice::Success(text.into())); Task::none() }
}
