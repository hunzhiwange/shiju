use crate::app::{App, Message};
use crate::app::state::DelegateAgentSettingsEntry;
use iced::Element;
pub(super) fn view<'a>(app: &'a App, draft: &'a DelegateAgentSettingsEntry) -> Element<'a, Message> {
    desktop_settings_agents::basic_profile::view(super::domain::profile_input(app), draft).map(super::domain::event)
}
#[cfg(test)]
#[path = "basic_profile_ui_refresh_15_tests.rs"]
mod ui_refresh_15_tests;
