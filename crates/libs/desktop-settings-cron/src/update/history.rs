use super::messages::{Message, SettingsMessage};
use crate::message::HistoryMessage as CronHistoryMessage;
use iced::Task;
pub fn refresh(app: &mut super::cron::Inputs<'_>) -> Task<Message> {
    let Some(generation) = app.cron_settings.history.begin_request() else {
        return Task::none();
    };
    Task::perform(desktop_config::config_cron_jobs::load_cron_history_async(app.client.clone()), move |result| {
        Message::Settings(SettingsMessage::CronHistory(CronHistoryMessage::Loaded {
            generation,
            result,
        }))
    })
}

pub fn update(app: &mut super::cron::Inputs<'_>, event: CronHistoryMessage) -> Task<Message> {

    let task = match event {
        CronHistoryMessage::Refresh => refresh(app),
        CronHistoryMessage::SearchToggled => {
            app.cron_settings.history.search_open = !app.cron_settings.history.search_open;
            Task::none()
        }
        CronHistoryMessage::DetailsOpened(key) => {
            app.cron_settings.history.selected_run = Some(key);
            Task::none()
        }
        CronHistoryMessage::DetailsClosed => {
            app.cron_settings.history.selected_run = None;
            Task::none()
        }
        CronHistoryMessage::CopyOutput(output) => iced::clipboard::write(output),
        CronHistoryMessage::Tick(generation) => {
            if generation == app.cron_settings.history.generation {
                refresh(app)
            } else {
                Task::none()
            }
        }
        CronHistoryMessage::Loaded { generation, result } => {
            if !app.cron_settings.history.accepts(generation) {
                return Task::none();
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
    task
}

/// Visibility is coordinated by the host; the history domain owns poll cadence and identity.
pub fn subscription(visible: bool, generation: u64) -> iced::Subscription<CronHistoryMessage> {
    if visible {
        iced::time::every(std::time::Duration::from_secs(3))
            .with(generation)
            .map(|(generation, _)| CronHistoryMessage::Tick(generation))
    } else {
        iced::Subscription::none()
    }
}
