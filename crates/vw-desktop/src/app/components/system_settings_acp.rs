use super::system_settings_agents::domain;
use crate::app::{App, Message};
use iced::Element;
pub fn view(app: &App) -> Element<'_, Message> {
    desktop_settings_agents::acp_view::view(domain::acp_input(app)).map(domain::event)
}
