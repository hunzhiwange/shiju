use crate::app::{App, Message};
use crate::app::state::DelegateAgentSettingsEntry;
use iced::Element;
pub(super) fn view<'a>(app: &'a App, entry: &'a DelegateAgentSettingsEntry, provider_options: Vec<String>) -> Element<'a, Message> {
    desktop_settings_agents::basic::view(desktop_settings_agents::basic::Input { help: super::domain::help,
        locale: app.desktop_locale, theme: &app.app_theme,
        profile: super::domain::profile_input(app), defaults: super::domain::defaults_input(app, entry),
    }, entry, provider_options).map(super::domain::event)
}
#[cfg(test)]
#[path = "basic_ui_refresh_16_tests.rs"]
mod ui_refresh_16_tests;
