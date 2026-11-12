//! 运行记录页的有界刷新与响应校验。
use super::SettingsMessage;
use crate::app::state::{CronHistoryLocation, CronHistoryStatus, CronSettingsTab};
use crate::app::{App, Message};
use iced::Task;
use vw_gateway_client::{CronHistoryResponse, CronJobDto};

#[derive(Debug, Clone)]
pub enum CronHistoryMessage {
    Refresh,
    Tick(u64),
    Loaded { generation: u64, result: Result<(CronHistoryResponse, Vec<CronJobDto>), String> },
    SearchChanged(String),
    StatusChanged(CronHistoryStatus),
    LocationChanged(CronHistoryLocation),
    TaskSelected(Option<String>),
    TaskSearchChanged(String),
    TaskPickerToggled(bool),
}

pub(crate) fn visible(app: &App) -> bool {
    app.show_system_settings
        && app.system_settings_tab == crate::app::components::system_settings::SystemTab::Cron
        && app.cron_settings.active_tab == CronSettingsTab::History
}

fn refresh(app: &mut App) -> Task<Message> {
    let Some(generation) = app.cron_settings.history.begin_request() else {
        return Task::none();
    };
    Task::perform(crate::app::config::load_cron_history_async(), move |result| {
        Message::Settings(SettingsMessage::CronHistory(CronHistoryMessage::Loaded {
            generation,
            result,
        }))
    })
}

/// 在主 reducer 边界同步上下文，使所有关闭/切页/项目切换入口行为一致。
pub(crate) fn reconcile(app: &mut App) -> Task<Message> {
    let visible = visible(app);
    if app.cron_settings.history.sync_context(visible, app.project_path.clone()) {
        refresh(app)
    } else {
        Task::none()
    }
}

pub(crate) fn update(app: &mut App, event: CronHistoryMessage) -> Task<Message> {
    let context_task = reconcile(app);
    let task = match event {
        CronHistoryMessage::Refresh => refresh(app),
        CronHistoryMessage::Tick(generation) => {
            if generation == app.cron_settings.history.generation {
                refresh(app)
            } else {
                Task::none()
            }
        }
        CronHistoryMessage::Loaded { generation, result } => {
            if !app.cron_settings.history.accepts(generation) {
                return context_task;
            }
            match result {
                Ok((response, jobs)) => {
                    app.cron_settings.history.apply_response(response, &jobs);
                    app.cron_settings.jobs = jobs;
                    app.cron_settings.prune_selected_jobs();
                    Task::none()
                }
                Err(error) => {
                    let history = &mut app.cron_settings.history;
                    let notify = history.error.as_ref() != Some(&error);
                    history.loading = false;
                    history.error = Some(error.clone());
                    if notify { app.show_error_toast(error) } else { Task::none() }
                }
            }
        }
        CronHistoryMessage::SearchChanged(value) => {
            app.cron_settings.history.search = value;
            Task::none()
        }
        CronHistoryMessage::StatusChanged(value) => {
            app.cron_settings.history.status_filter = value;
            Task::none()
        }
        CronHistoryMessage::LocationChanged(value) => {
            app.cron_settings.history.location_filter = value;
            Task::none()
        }
        CronHistoryMessage::TaskSelected(value) => {
            app.cron_settings.history.selected_job_id =
                value.filter(|id| app.cron_settings.jobs.iter().any(|job| job.id == *id));
            app.cron_settings.history.task_picker_open = false;
            Task::none()
        }
        CronHistoryMessage::TaskSearchChanged(value) => {
            app.cron_settings.history.task_search = value;
            Task::none()
        }
        CronHistoryMessage::TaskPickerToggled(value) => {
            app.cron_settings.history.task_picker_open = value;
            Task::none()
        }
    };
    Task::batch([context_task, task])
}
