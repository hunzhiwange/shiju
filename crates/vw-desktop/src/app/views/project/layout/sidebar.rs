use crate::app::{App, Message, message};
use super::sessions_panel::project_sessions_panel_container;
use desktop_workspace::views::project::layout::sidebar as layout;

pub fn sidebar_group(
    app: &App,
    width: f32,
    rail: f32,
    scale: f32,
    radius: f32,
) -> (iced::Element<'_, Message>, Option<iced::Element<'_, Message>>) {
    let expanded = app.navigation_panels.show_settings;
    #[cfg(not(target_arch = "wasm32"))]
    let expanded = expanded || app.navigation_tabs.screen == crate::app::Screen::DigitalEmployees;
    layout::sidebar_group(
        expanded,
        super::super::components::projects_list(app, !expanded),
        project_sessions_panel_container(
            app,
            width,
            rail,
            scale,
            radius,
            app.navigation_tabs.project_path.clone(),
        ),
        Message::View(message::ViewMessage::SettingsDragStarted),
        width,
        rail,
        scale,
    )
}
#[cfg(test)]
pub fn hover_overlay_layout<'a>(app: &'a App, left: iced::Element<'a, Message>, right: iced::Element<'a, Message>, width:f32, rail:f32, scale:f32, radius:f32, spacing:f32) -> iced::Element<'a, Message> {
    layout::hover_overlay_layout(left,right,app.workspace_state.hovered_recent_project.clone().map(|p| project_sessions_panel_container(app,width,rail,scale,radius,Some(p))), Message::Project(message::ProjectMessage::RecentOverlayClosed),width,rail,scale,spacing)
}
