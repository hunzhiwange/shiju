//! Computer control UI assembly.
use crate::app::{App, Message};
use crate::app::state::ControlIdentity;
use crate::app::message::settings::computer_control::message;
use iced::Element;
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    desktop_settings_security::system_settings_computer_control::view(&app.computer_control_settings, &ControlIdentity::from_endpoint(&crate::app::gateway_client_endpoint())).map(message)
}
pub(crate) fn view_overlays<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    // The overlay accepts a host element via a typed wrapper to preserve its messages.
    desktop_settings_security::system_settings_computer_control::host_overlays(&app.computer_control_settings, &ControlIdentity::from_endpoint(&crate::app::gateway_client_endpoint()), base, message)
}
#[cfg(test)]
#[path = "system_settings_computer_control_tests.rs"]
mod tests;
