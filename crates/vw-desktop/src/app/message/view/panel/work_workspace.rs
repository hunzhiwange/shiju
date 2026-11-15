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
    let mut state = domain::State {
        work_resource_mode: std::mem::take(&mut app.work_resource_mode),
        work_resources_visible: std::mem::take(&mut app.work_resources_visible),
        dragging_work_split: std::mem::take(&mut app.dragging_work_split),
        work_split_drag_anchor_x: std::mem::take(&mut app.work_split_drag_anchor_x),
        work_resource_fullscreen: std::mem::take(&mut app.work_resource_fullscreen),
        work_resource_half_fullscreen: std::mem::take(&mut app.work_resource_half_fullscreen),
        work_split_ratio: std::mem::take(&mut app.work_split_ratio),
        work_chat_fullscreen: std::mem::take(&mut app.work_chat_fullscreen),
        work_chat_half_fullscreen: std::mem::take(&mut app.work_chat_half_fullscreen),
        work_expanded_spaces: std::mem::take(&mut app.work_expanded_spaces),
    };
    domain::update(&mut state, message);
    app.work_resource_mode = state.work_resource_mode;
    app.work_resources_visible = state.work_resources_visible;
    app.dragging_work_split = state.dragging_work_split;
    app.work_split_drag_anchor_x = state.work_split_drag_anchor_x;
    app.work_resource_fullscreen = state.work_resource_fullscreen;
    app.work_resource_half_fullscreen = state.work_resource_half_fullscreen;
    app.work_split_ratio = state.work_split_ratio;
    app.work_chat_fullscreen = state.work_chat_fullscreen;
    app.work_chat_half_fullscreen = state.work_chat_half_fullscreen;
    app.work_expanded_spaces = state.work_expanded_spaces;
    Task::none()
}

#[cfg(test)]
#[path = "work_workspace_tests.rs"]
mod work_workspace_tests;
