#[cfg(test)]
use crate::app::{App, Message};
#[cfg(test)]
use crate::app::state::{AgentsSettingsState, DelegateAgentSettingsEntry};
#[cfg(test)]
use iced::Element;
#[cfg(test)]
pub(super) fn entries<'a>(settings: &'a AgentsSettingsState, official: bool, query: &str) -> Vec<&'a DelegateAgentSettingsEntry> {
    desktop_settings_agents::catalog::entries(&settings.entries, official, query)
}
#[cfg(test)]
pub(super) fn grid(app: &App) -> Element<'_, Message> {
    desktop_settings_agents::catalog::grid(super::domain::catalog_input(app)).map(super::domain::event)
}
