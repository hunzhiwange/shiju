//! Desktop effects for computer control.
use crate::app::{App, Message};
use crate::app::message::SettingsMessage;
use crate::app::state::ToastKind;
use iced::Task;
pub(crate) use desktop_settings_security::computer_control_update::Event;
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::ComputerControl(event)) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let (task, notice) = desktop_settings_security::computer_control_update::update(
        &mut app.computer_control_settings, &crate::app::config::gateway_client_endpoint(),
        app.active_session_id.as_deref(), app.project_path.as_deref(), event);
    let toast = match notice {
        Some(desktop_settings_security::computer_control_update::Notice::Error(value)) => app.show_toast(ToastKind::Error, value),
        Some(desktop_settings_security::computer_control_update::Notice::Success(value)) => app.show_toast(ToastKind::Success, value),
        None => Task::none(),
    };
    Task::batch([task.map(message), toast])
}
