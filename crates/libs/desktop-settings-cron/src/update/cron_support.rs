use super::cron::Inputs;
use super::messages::{Message, SettingsMessage};
use crate::state::{CronAddJobType, CronAddScheduleKind};
use desktop_config::config_cron_jobs::load_cron_jobs_async;
use iced::Task;
use std::fmt::Write;
pub fn schedule_request(
    draft: &crate::state::CronJobDraft,
) -> Result<gateway_cron::CronPreviewRequest, String> {
    if let Some(err) = &draft.schedule_error {
        return Err(err.clone());
    }
    if let Some(rule) = &draft.schedule_draft {
        rule.timezone.parse::<chrono_tz::Tz>().map_err(|_| "请选择有效的 IANA 时区")?;
        return rule.request();
    }
    Ok(gateway_cron::CronPreviewRequest {
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

pub(super) fn restore_schedule_metadata(
    draft: &mut crate::state::CronJobDraft,
    previous: Option<crate::state::CronScheduleDraft>,
) {
    if let Some(previous) = previous {
        match schedule_request(draft)
            .and_then(|request| crate::state::CronScheduleDraft::from_request(&request))
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

pub(super) fn invalidate_schedule(draft: &mut crate::state::CronJobDraft) {
    draft.schedule_error = None;
    draft.schedule_revision += 1;
    draft.preview = None;
    draft.preview_loading = false;
}

pub(super) fn refresh_cron_jobs_task(app: &Inputs<'_>) -> Task<Message> {
    Task::perform(load_cron_jobs_async(app.client.clone()), |result| {
        Message::Settings(SettingsMessage::CronJobsLoaded(result))
    })
}

#[cfg(target_arch = "wasm32")]
pub(super) fn cron_mutation_task(
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
pub(super) fn cron_mutation_task(
    success_message: &'static str,
    future: impl std::future::Future<Output = Result<(), String>> + Send + 'static,
) -> Task<Message> {
    Task::perform(future, move |result| {
        Message::Settings(SettingsMessage::CronJobMutationCompleted(
            result.map(|()| success_message.to_string()),
        ))
    })
}

#[cfg(target_arch = "wasm32")]
pub(super) fn cron_editor_save_task(
    editing: bool,
    success_message: &'static str,
    future: impl std::future::Future<Output = Result<(), String>> + 'static,
) -> Task<Message> {
    Task::perform(future, move |result| {
        Message::Settings(SettingsMessage::CronEditorSaveCompleted {
            editing,
            result: result.map(|()| success_message.to_string()),
        })
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn cron_editor_save_task(
    editing: bool,
    success_message: &'static str,
    future: impl std::future::Future<Output = Result<(), String>> + Send + 'static,
) -> Task<Message> {
    Task::perform(future, move |result| {
        Message::Settings(SettingsMessage::CronEditorSaveCompleted {
            editing,
            result: result.map(|()| success_message.to_string()),
        })
    })
}

pub(super) fn format_run_history_text(
    locale: config_ui::DesktopLocale,
    runs: &[gateway_cron::CronRunDto],
) -> String {
    let mut text = String::new();

    for (index, run) in runs.iter().enumerate() {
        if index > 0 {
            text.push_str("\n\n");
        }

        let duration = run
            .duration_ms
            .map(|value| format!("{value} ms"))
            .unwrap_or_else(|| desktop_i18n::I18n::new(locale).__("无").to_string());
        let output =
            run.output.as_deref().unwrap_or(desktop_i18n::I18n::new(locale).__("无输出"));

        let _ = writeln!(text, "#{}", index + 1);
        let _ = writeln!(
            text,
            "{}",
            desktop_i18n::__!(desktop_i18n::I18n::new(locale), "状态: %s", run.status.as_str())
                .expect("valid history template")
        );
        let _ = writeln!(
            text,
            "{}",
            desktop_i18n::__!(desktop_i18n::I18n::new(locale), "开始: %s", run.started_at.as_str())
                .expect("valid history template")
        );
        let _ = writeln!(
            text,
            "{}",
            desktop_i18n::__!(desktop_i18n::I18n::new(locale), "结束: %s", run.finished_at.as_str())
                .expect("valid history template")
        );
        let _ = writeln!(
            text,
            "{}",
            desktop_i18n::__!(desktop_i18n::I18n::new(locale), "耗时: %s", duration.as_str())
                .expect("valid history duration template")
        );
        let _ = writeln!(text, "{}", desktop_i18n::I18n::new(locale).__("输出:"));
        text.push_str(output);
    }

    text
}

pub(super) fn persist_cron_settings(app: &mut Inputs<'_>) -> Task<Message> {
    let enabled = app.cron_settings.enabled;
    let max_run_history = app.cron_settings.max_run_history.clamp(1, 10_000);
    app.config.update_cron_config_async(move |cron| {
        cron.enabled = enabled;
        cron.max_run_history = max_run_history;
    }).map(Message::ConfigSaved)
}

pub(super) fn schedule_kind_from_api(
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

pub(super) fn job_type_from_api(job: &gateway_cron::CronJobDto) -> CronAddJobType {
    if crate::system_settings_cron::job_is_agent(job) {
        CronAddJobType::Agent
    } else {
        CronAddJobType::Shell
    }
}

pub(super) fn model_provider_from_model(model: &str) -> String {
    model.split_once('/').map(|(provider, _)| provider.to_string()).unwrap_or_default()
}




#[cfg(test)] #[path = "cron_tests.rs"] mod tests;
