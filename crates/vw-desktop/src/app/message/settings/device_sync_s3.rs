//! Desktop identity and effect adapter.
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;
pub(crate) use desktop_settings_memory::device_sync_s3::{Event, State};
use desktop_settings_memory::device_sync_s3 as domain;
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::DeviceSyncS3(event)) }
pub(crate) fn visible(app: &App) -> bool { domain::visible(&app.apps_catalog.device_sync_s3, super::data_management::export_ui::identity(app).as_ref()) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Feedback(success, value) = event { return if success { app.show_success_toast(value) } else { app.show_error_toast(value) }; }
    let current = super::data_management::export_ui::identity(app);
    domain::update(&mut app.apps_catalog.device_sync_s3, current, crate::app::config::gateway_client().ok(), app.apps_catalog.token.clone(), event).then(|output| match output {
        domain::Output::Event(event) => Task::done(message(event)),
        domain::Output::Error(value) => Task::done(Message::Settings(SettingsMessage::DeviceSyncS3(Event::Feedback(false, value)))),
        domain::Output::Success(value) => Task::done(Message::Settings(SettingsMessage::DeviceSyncS3(Event::Feedback(true, value)))),
    })
}

#[cfg(test)]
pub(crate) use desktop_settings_memory::system_settings_device_sync::s3_modal::S3Preset;
