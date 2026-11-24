//! Desktop identity and effect adapter.
use crate::app::{App, Message, message::SettingsMessage};
use iced::Task;
pub(crate) use desktop_settings_memory::data_management::recovery_ui::{Event, State};
use desktop_settings_memory::data_management::recovery_ui as domain;
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::DataRecovery(event)) }
pub(crate) fn visible(app: &App) -> bool { domain::visible(&app.apps_catalog.recovery, super::identity::identity(app).as_ref()) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Feedback(success, value) = event { let value = desktop_i18n::settings::message(desktop_i18n::I18n::new(app.general_settings.desktop_locale), &value); return if success { app.show_success_toast(value) } else { app.show_error_toast(value) }; }
    let current = super::identity::identity(app);
    domain::update(&mut app.apps_catalog.recovery, current, crate::app::config::gateway_client().ok(), app.apps_catalog.token.clone(), event).then(|output| match output {
        domain::Output::Event(event) => Task::done(message(event)),
        domain::Output::Error(value) => Task::done(Message::Settings(SettingsMessage::DataRecovery(Event::Feedback(false, value)))),
        domain::Output::Success(value) => Task::done(Message::Settings(SettingsMessage::DataRecovery(Event::Feedback(true, value)))),
    })
}
