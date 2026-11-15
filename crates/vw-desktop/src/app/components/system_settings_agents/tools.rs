#[cfg(test)]
use crate::app::{App, Message};
use crate::app::state::DelegateAgentSettingsEntry;
use iced::Element;
#[cfg(test)]
pub(super) use desktop_settings_agents::tools::*;
#[cfg(test)]
pub(super) fn view<'a>(app: &'a App, entry: &'a DelegateAgentSettingsEntry) -> Element<'a, Message> {
desktop_settings_agents::tools::view(desktop_settings_agents::tools::Input { locale: app.desktop_locale, theme: &app.app_theme, help: super::domain::help, available_tools: &app.agents_settings.available_tools, active_tools_list_tab: &app.agents_settings.active_tools_list_tab }, entry).map(super::domain::event)
}
#[cfg(test)]
pub(super) fn tool_card_press_message(key: String, tool: String, checked: bool, locked: bool) -> Option<Message> { desktop_settings_agents::tools::tool_card_press_message(key, tool, checked, locked).map(super::domain::event) }
