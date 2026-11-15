//! Desktop identity and effect adapter.
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;
pub(crate) use desktop_settings_memory::data_management::backup_list_ui::{Event, State};
use desktop_settings_memory::data_management::backup_list_ui as domain;
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::DataBackups(event)) }
pub(crate) fn visible(app: &App) -> bool { domain::visible(&app.apps_catalog.data_backups, super::export_ui::identity(app).as_ref()) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Feedback(success, value) = event { return if success { app.show_success_toast(value) } else { app.show_error_toast(value) }; }
    let current = super::export_ui::identity(app);
    domain::update(&mut app.apps_catalog.data_backups, current, crate::app::config::gateway_client().ok(), app.apps_catalog.token.clone(), event).then(|output| match output {
        domain::Output::Event(event) => Task::done(message(event)),
        domain::Output::Error(value) => Task::done(Message::Settings(SettingsMessage::DataBackups(Event::Feedback(false, value)))),
        domain::Output::Success(value) => Task::done(Message::Settings(SettingsMessage::DataBackups(Event::Feedback(true, value)))),
    })
}
