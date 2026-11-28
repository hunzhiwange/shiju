//! Computer control UI assembly.
use crate::app::{App, Message};
use crate::app::state::ControlIdentity;
use crate::app::message::settings::computer_control::message;
use iced::Element;
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    desktop_settings_security::system_settings_computer_control::view(desktop_i18n::I18n::new(app.general_settings.desktop_locale), &app.computer_control_settings, &ControlIdentity::from_endpoint(&crate::app::gateway_client_endpoint())).map(message)
}
#[cfg(test)]
#[path = "system_settings_computer_control_tests.rs"]
mod tests;
