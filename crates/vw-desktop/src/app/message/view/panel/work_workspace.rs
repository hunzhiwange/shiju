use super::ViewMessage;
use crate::app::{App, Message};
use iced::Task;
use desktop_navigation::panel::work_workspace as domain;

pub(super) fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    let message = match message {
        ViewMessage::WorkResourceModeSelected(mode) => domain::Message::WorkResourceModeSelected(mode),
        ViewMessage::WorkResourcesToggled => domain::Message::WorkResourcesToggled,
        ViewMessage::WorkResourceFullscreenToggled => domain::Message::WorkResourceFullscreenToggled,
        ViewMessage::WorkResourceHalfFullscreenToggled => domain::Message::WorkResourceHalfFullscreenToggled,
        ViewMessage::WorkSpaceToggled(project_path) => domain::Message::WorkSpaceToggled(project_path),
        ViewMessage::WorkSplitRatioChanged(ratio) => domain::Message::WorkSplitRatioChanged(ratio),
        _ => return Task::none(),
    };
    domain::update(&mut app.navigation_workspace, message);
    Task::none()
}

#[cfg(test)]
#[path = "work_workspace_tests.rs"]
mod work_workspace_tests;
