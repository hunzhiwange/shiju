use crate::app::{App, Message};
#[cfg(test)]
use config_aggregate::AcpAgentConfig;
use iced::Element;
use super::system_settings_agents::domain;
#[cfg(test)]
pub(super) use desktop_settings_agents::acp::{acp_description, setup_hint};
#[cfg(test)]
fn acp_card<'a>(app: &'a App, agent: String, config: &'a AcpAgentConfig) -> Element<'a, Message> {
    desktop_settings_agents::acp_view::acp_card(domain::acp_input(app), agent, config).map(domain::event)
}
#[cfg(test)]
fn new_agent_form(app: &App) -> Element<'_, Message> {
    desktop_settings_agents::acp_view::new_agent_form(domain::acp_input(app)).map(domain::event)
}
pub fn view(app: &App) -> Element<'_, Message> {
    desktop_settings_agents::acp_view::view(domain::acp_input(app)).map(domain::event)
}
#[cfg(test)]
#[path = "system_settings_acp_tests.rs"]
mod system_settings_acp_tests;
#[cfg(test)]
#[path = "system_settings_acp_ui_refresh_18_tests.rs"]
mod ui_refresh_18_tests;
