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
    let result = domain::update(&mut app.navigation_mode, message);
    if app.navigation_mode.show_workspace_mode_popover { app.active_menu = None; }
    if let Some(mode) = result { return app.save_desktop_preference("workspace_mode", mode.as_str().into()); }
    Task::none()
}

#[cfg(test)]
#[path = "workspace_mode_tests.rs"]
mod workspace_mode_tests;
