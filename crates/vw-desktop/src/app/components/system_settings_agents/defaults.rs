use crate::app::{App, Message};
use crate::app::state::DelegateAgentSettingsEntry;
use iced::Element;
pub(crate) use desktop_settings_agents::defaults::patch;
pub(crate) fn validation_error(app: &App, draft: &DelegateAgentSettingsEntry) -> Option<String> {
    desktop_settings_agents::defaults::validation_error(&super::domain::defaults_input(app, draft), draft)
}
pub(super) use desktop_settings_agents::defaults::selection_mode;
pub(super) fn view<'a>(app: &'a App, draft: &'a DelegateAgentSettingsEntry) -> Element<'a, Message> {
    desktop_settings_agents::defaults::view(super::domain::defaults_input(app, draft), draft).map(super::domain::event)
}
