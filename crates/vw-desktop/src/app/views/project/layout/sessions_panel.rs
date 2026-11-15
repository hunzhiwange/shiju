use crate::app::{App, Message};
pub fn project_sessions_panel_container(app: &App, width: f32, rail: f32, scale: f32, radius: f32, target: Option<String>) -> iced::Element<'_, Message> {
    desktop_workspace::views::project::layout::sessions_panel::project_sessions_panel_container(super::super::components::project_sessions_panel(app, target), width, rail, scale, radius)
}
