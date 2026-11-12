//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::config::{
    add_cron_job_async, delete_cron_job_async, delete_cron_jobs_async, load_cron_job_runs_async,
    load_cron_jobs_async, set_cron_job_enabled_async, set_cron_jobs_enabled_async,
    update_cron_job_async,
};
use crate::app::state::{CronAddJobType, CronAddScheduleKind, CronFrequency, CronSettingsTab};
use crate::app::{App, Message};
use iced::Task;
use iced::widget::text_editor;
use std::fmt::Write;

use super::messages::SettingsMessage;

fn schedule_request(
    draft: &crate::app::state::CronJobDraft,
) -> Result<vw_gateway_client::CronPreviewRequest, String> {
    if let Some(err) = &draft.schedule_error {
        return Err(err.clone());
    }
    if let Some(rule) = &draft.schedule_draft {
        return rule.request();
    }
    Ok(vw_gateway_client::CronPreviewRequest {
        schedule_kind: draft.schedule_kind.as_api_value().into(),
        schedule: (draft.schedule_kind == CronAddScheduleKind::Cron)
            .then(|| draft.schedule.clone()),
        at: (draft.schedule_kind == CronAddScheduleKind::At).then(|| draft.at.clone()),
        every_ms: if draft.schedule_kind == CronAddScheduleKind::Every {
            Some(draft.every_ms.parse().map_err(|_| "固定间隔毫秒数无效")?)
        } else {
            None
        },
        timezone: Some("UTC".into()),
        ..Default::default()
    })
}

fn restore_schedule_metadata(
    draft: &mut crate::app::state::CronJobDraft,
    previous: Option<crate::app::state::CronScheduleDraft>,
) {
    if let Some(previous) = previous {
        match schedule_request(draft)
            .and_then(|request| crate::app::state::CronScheduleDraft::from_request(&request))
        {
            Ok(mut rule) => {
                rule.timezone = previous.timezone;
                rule.start_date = previous.start_date;
                rule.end_date = previous.end_date;
                rule.starts_at = previous.starts_at;
                rule.ends_at = previous.ends_at;
                draft.schedule_draft = Some(rule);
            }
            Err(err) => {
                draft.schedule_error = Some(err);
                draft.schedule_draft = Some(previous);
            }
        }
    }
}

fn invalidate_schedule(draft: &mut crate::app::state::CronJobDraft) {
    draft.schedule_error = None;
    draft.schedule_revision += 1;
    draft.preview = None;
}

fn refresh_cron_jobs_task() -> Task<Message> {
    Task::perform(load_cron_jobs_async(), |result| {
        Message::Settings(SettingsMessage::CronJobsLoaded(result))
    })
}

#[cfg(target_arch = "wasm32")]
fn cron_mutation_task(
    success_message: &'static str,
    future: impl std::future::Future<Output = Result<(), String>> + 'static,
) -> Task<Message> {
    Task::perform(future, move |result| {
        Message::Settings(SettingsMessage::CronJobMutationCompleted(
            result.map(|()| success_message.to_string()),
        ))
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn cron_mutation_task(
    success_message: &'static str,
    future: impl std::future::Future<Output = Result<(), String>> + Send + 'static,
) -> Task<Message> {
    Task::perform(future, move |result| {
        Message::Settings(SettingsMessage::CronJobMutationCompleted(
            result.map(|()| success_message.to_string()),
        ))
    })
}

fn format_run_history_text(
    locale: vw_config_types::ui::DesktopLocale,
    runs: &[vw_gateway_client::CronRunDto],
) -> String {
    let mut text = String::new();

    for (index, run) in runs.iter().enumerate() {
        if index > 0 {
            text.push_str("\n\n");
        }

        let duration = run
            .duration_ms
            .map(|value| format!("{value} ms"))
            .unwrap_or_else(|| crate::app::i18n::I18n::new(locale).__("无").to_string());
        let output =
            run.output.as_deref().unwrap_or(crate::app::i18n::I18n::new(locale).__("无输出"));

        let _ = writeln!(text, "#{}", index + 1);
        let _ = writeln!(
            text,
            "{}",
            crate::__!(crate::app::i18n::I18n::new(locale), "状态: %s", run.status.as_str())
                .expect("valid history template")
        );
        let _ = writeln!(
            text,
            "{}",
            crate::__!(crate::app::i18n::I18n::new(locale), "开始: %s", run.started_at.as_str())
                .expect("valid history template")
        );
        let _ = writeln!(
            text,
            "{}",
            crate::__!(crate::app::i18n::I18n::new(locale), "结束: %s", run.finished_at.as_str())
                .expect("valid history template")
        );
        let _ = writeln!(
            text,
            "{}",
            crate::__!(crate::app::i18n::I18n::new(locale), "耗时: %s", duration.as_str())
                .expect("valid history duration template")
        );
        let _ = writeln!(text, "{}", crate::app::i18n::I18n::new(locale).__("输出:"));
        text.push_str(output);
    }

    text
}

fn persist_cron_settings(app: &mut App) -> Task<Message> {
    let enabled = app.cron_settings.enabled;
    let max_run_history = app.cron_settings.max_run_history.clamp(1, 10_000);
    crate::app::update_cron_config_async(move |cron| {
        cron.enabled = enabled;
        cron.max_run_history = max_run_history;
    })
}

fn schedule_kind_from_api(
    value: &str,
    expression: &str,
    at: Option<&str>,
    every_ms: Option<u64>,
) -> CronAddScheduleKind {
    match value.trim().to_ascii_lowercase().as_str() {
        "at" | "指定时间" => CronAddScheduleKind::At,
        "every" | "固定间隔" => CronAddScheduleKind::Every,
        "cron" => CronAddScheduleKind::Cron,
        _ if every_ms.is_some() => CronAddScheduleKind::Every,
        _ if at.is_some_and(|value| !value.trim().is_empty()) => CronAddScheduleKind::At,
        _ if !expression.trim().is_empty() => CronAddScheduleKind::Cron,
        _ => CronAddScheduleKind::Cron,
    }
}

fn job_type_from_api(job: &vw_gateway_client::CronJobDto) -> CronAddJobType {
    if job.job_type.trim().eq_ignore_ascii_case("agent") {
        return CronAddJobType::Agent;
    }

    let has_prompt = job.prompt.as_deref().is_some_and(|value| !value.trim().is_empty());
    let has_command = !job.command.trim().is_empty();
    if has_prompt && !has_command { CronAddJobType::Agent } else { CronAddJobType::Shell }
}

fn model_provider_from_model(model: &str) -> String {
    model.split_once('/').map(|(provider, _)| provider.to_string()).unwrap_or_default()
}

#[cfg(test)]
#[path = "cron_tests.rs"]
mod cron_tests;

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    match message {
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
                Ok(request) => Task::perform(
                    crate::app::config::preview_cron_schedule_async(request),
                    move |result| {
                        Message::Settings(SettingsMessage::CronSchedulePreviewLoaded {
                            editing,
                            revision,
                            result,
                        })
                    },
                ),
                Err(err) => {
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
        SettingsMessage::CronTabSelected(tab) => {
            app.cron_settings.active_tab = tab;
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            app.cron_settings.action_status = None;
            if tab == CronSettingsTab::Jobs {
                app.cron_settings.jobs_loading = true;
                refresh_cron_jobs_task()
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
            refresh_cron_jobs_task()
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
            Task::perform(load_cron_job_runs_async(job_id.clone()), move |result| {
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
                match crate::app::state::CronScheduleDraft::from_job(job) {
                    Ok(rule) => app.cron_settings.edit_draft.schedule_draft = Some(rule),
                    Err(err) => {
                        app.cron_settings.save_error = Some(err);
                        return Task::none();
                    }
                }
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
                    Some(crate::app::state::CronScheduleRule::Advanced(_))
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
            app.cron_settings.edit_draft.full_access = value;
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
            app.cron_settings.edit_draft.delivery_channel = value;
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
                    app.cron_settings.save_error = Some(err);
                    return Task::none();
                }
            };

            let Some(job_id) = app.cron_settings.editing_job_id.clone() else {
                app.cron_settings.save_error_translation =
                    Some(crate::app::i18n::format::Message::new("没有正在编辑的定时任务", vec![]));
                app.cron_settings.save_error = Some("没有正在编辑的定时任务".to_string());
                return Task::none();
            };
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            app.cron_settings.edit_draft.submitting = true;
            cron_mutation_task(
                "定时任务已更新",
                update_cron_job_async(
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
                set_cron_job_enabled_async(job_id, enabled),
            )
        }
        SettingsMessage::CronJobDelete(job_id) => {
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            cron_mutation_task("定时任务已删除", delete_cron_job_async(job_id))
        }
        SettingsMessage::CronSelectedJobsEnable => {
            app.cron_settings.prune_selected_jobs();
            let job_ids = app.cron_settings.selected_job_ids.clone();
            if job_ids.is_empty() {
                app.cron_settings.save_error_translation = Some(
                    crate::app::i18n::format::Message::new("请先选择要启用的定时任务", vec![]),
                );
                app.cron_settings.save_error = Some("请先选择要启用的定时任务".to_string());
                return Task::none();
            }
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            cron_mutation_task("选中的定时任务已启用", set_cron_jobs_enabled_async(job_ids, true))
        }
        SettingsMessage::CronSelectedJobsDisable => {
            app.cron_settings.prune_selected_jobs();
            let job_ids = app.cron_settings.selected_job_ids.clone();
            if job_ids.is_empty() {
                app.cron_settings.save_error_translation = Some(
                    crate::app::i18n::format::Message::new("请先选择要禁用的定时任务", vec![]),
                );
                app.cron_settings.save_error = Some("请先选择要禁用的定时任务".to_string());
                return Task::none();
            }
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            cron_mutation_task("选中的定时任务已禁用", set_cron_jobs_enabled_async(job_ids, false))
        }
        SettingsMessage::CronSelectedJobsDelete => {
            app.cron_settings.prune_selected_jobs();
            let job_ids = app.cron_settings.selected_job_ids.clone();
            if job_ids.is_empty() {
                app.cron_settings.save_error_translation = Some(
                    crate::app::i18n::format::Message::new("请先选择要删除的定时任务", vec![]),
                );
                app.cron_settings.save_error = Some("请先选择要删除的定时任务".to_string());
                return Task::none();
            }
            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            cron_mutation_task("选中的定时任务已删除", delete_cron_jobs_async(job_ids))
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
            app.cron_settings.add_draft.full_access = value;
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
            app.cron_settings.add_draft.delivery_channel = value;
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
                    app.cron_settings.save_error = Some(err);
                    return Task::none();
                }
            };

            app.cron_settings.save_error = None;
            app.cron_settings.save_error_translation = None;
            app.cron_settings.add_draft.submitting = true;
            cron_mutation_task(
                "定时任务已新增",
                add_cron_job_async(
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
        SettingsMessage::CronJobMutationCompleted(result) => {
            app.cron_settings.add_draft.submitting = false;
            app.cron_settings.edit_draft.submitting = false;
            match result {
                Ok(message) => {
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
                    refresh_cron_jobs_task()
                }
                Err(err) => {
                    app.cron_settings.save_error_translation = None;
                    app.cron_settings.save_error = Some(err);
                    Task::none()
                }
            }
        }
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

pub(crate) fn refresh_run_history_locale(app: &mut App) {
    let text = format_run_history_text(app.desktop_locale, &app.cron_settings.runs_modal);
    app.cron_settings.runs_modal_editor = text_editor::Content::with_text(&text);
}
