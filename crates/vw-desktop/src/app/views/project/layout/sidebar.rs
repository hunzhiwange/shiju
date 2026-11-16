use crate::app::{App, Message, message};
use super::sessions_panel::project_sessions_panel_container;
use desktop_workspace::views::project::layout::sidebar as layout;
fn workspace_rail(app: &App, collapsed: bool) -> iced::Element<'_, Message> {
    layout::workspace_rail(super::super::components::projects_list(app, collapsed), Message::View(message::ViewMessage::OpenSystemSettingsTab(crate::app::components::system_settings::SystemTab::Apps)), crate::app::components::external_sessions::message(crate::app::components::external_sessions::Event::Open))
}
pub fn sidebar_group(app: &App, width: f32, rail: f32, scale: f32, radius: f32) -> (iced::Element<'_, Message>, Option<iced::Element<'_, Message>>) {
    layout::sidebar_group(app.navigation_panels.show_settings, workspace_rail(app, !app.navigation_panels.show_settings), project_sessions_panel_container(app,width,rail,scale,radius,app.navigation_tabs.project_path.clone()), Message::View(message::ViewMessage::SettingsDragStarted), width,rail,scale)
}
pub fn hover_overlay_layout<'a>(app: &'a App, left: iced::Element<'a, Message>, right: iced::Element<'a, Message>, width:f32, rail:f32, scale:f32, radius:f32, spacing:f32) -> iced::Element<'a, Message> {
    layout::hover_overlay_layout(left,right,app.hovered_recent_project.clone().map(|p| project_sessions_panel_container(app,width,rail,scale,radius,Some(p))), Message::Project(message::ProjectMessage::RecentOverlayClosed),width,rail,scale,spacing)
}
