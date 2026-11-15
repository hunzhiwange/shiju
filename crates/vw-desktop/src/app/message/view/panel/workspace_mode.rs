use super::ViewMessage;
use crate::app::{App, Message};
use iced::Task;
use desktop_navigation::panel::workspace_mode as domain;

#[cfg(test)]
fn select_workspace_mode(app: &mut App, mode: crate::app::state::WorkspaceMode) {
    let _ = update(app, ViewMessage::WorkspaceModeSelected(mode));
}
pub(super) fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    let message = match message {
        ViewMessage::ToggleWorkspaceModePopover => domain::Message::ToggleWorkspaceModePopover,
        ViewMessage::CloseWorkspaceModePopover => domain::Message::CloseWorkspaceModePopover,
        ViewMessage::WorkspaceModeSelected(mode) => domain::Message::WorkspaceModeSelected(mode),
        _ => return Task::none(),
    };
    let mut state = domain::State {
        workspace_mode: std::mem::take(&mut app.workspace_mode),
        work_current_project_menu: std::mem::take(&mut app.work_current_project_menu),
        show_workspace_mode_popover: std::mem::take(&mut app.show_workspace_mode_popover),
        code_projects_menu_open: std::mem::take(&mut app.code_projects_menu_open),
    };
    let result = domain::update(&mut state, message);
    app.workspace_mode = state.workspace_mode;
    app.work_current_project_menu = state.work_current_project_menu;
    app.show_workspace_mode_popover = state.show_workspace_mode_popover;
    app.code_projects_menu_open = state.code_projects_menu_open;
    if app.show_workspace_mode_popover { app.active_menu = None; }
    if let Some(mode) = result { return app.save_desktop_preference("workspace_mode", mode.as_str().into()); }
    Task::none()
}

#[cfg(test)]
#[path = "workspace_mode_tests.rs"]
mod workspace_mode_tests;
