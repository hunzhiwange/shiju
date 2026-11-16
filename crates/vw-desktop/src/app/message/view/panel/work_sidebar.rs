use super::ViewMessage;
use crate::app::{App, Message};
use iced::Task;
use desktop_navigation::panel::work_sidebar as domain;

pub(super) fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    if matches!(message, ViewMessage::WorkProjectHome) {
        if domain::project_home(app.navigation_tabs.screen, app.navigation_tabs.project_path.is_some(), app.active_session_id.is_some(), &mut app.navigation_workspace) {
            app.stash_active_session_view();
            app.active_session_id = None;
            // The stash may retain the view in place when it exceeds cache capacity.
            app.chat.clear();
            app.chat_message_ids.clear();
            app.usage = Default::default();
            app.active_session_view_state = Default::default();
            app.chat_panel_mode = Default::default();
            app.invalidate_chat_ui_state();
        }
        return Task::none();
    }
    let message = match message {
        ViewMessage::WorkSidebarToggled => domain::Message::WorkSidebarToggled,
        ViewMessage::WorkSidebarFilterToggled => domain::Message::WorkSidebarFilterToggled,
        ViewMessage::WorkSidebarFilterClosed => domain::Message::WorkSidebarFilterClosed,
        ViewMessage::WorkSidebarMoreOpened => domain::Message::WorkSidebarMoreOpened,
        ViewMessage::WorkSidebarMoreClosed => domain::Message::WorkSidebarMoreClosed,
        ViewMessage::WorkSidebarProjectNavHovered(hovered) => domain::Message::WorkSidebarProjectNavHovered(hovered),
        ViewMessage::WorkSidebarFilterScopeSelected(scope) => domain::Message::WorkSidebarFilterScopeSelected(scope),
        ViewMessage::WorkSidebarFilterStatusSelected(status) => domain::Message::WorkSidebarFilterStatusSelected(status),
        ViewMessage::WorkSidebarFilterTimeSelected(time) => domain::Message::WorkSidebarFilterTimeSelected(time),
        ViewMessage::WorkSidebarFilterReset => domain::Message::WorkSidebarFilterReset,
        ViewMessage::WorkSidebarTabSelected(tab) => domain::Message::WorkSidebarTabSelected(tab),
        ViewMessage::WorkSidebarLoadMorePressed => domain::Message::WorkSidebarLoadMorePressed,
        ViewMessage::WorkSidebarSpaceTasksLoadMorePressed(project_path) => domain::Message::WorkSidebarSpaceTasksLoadMorePressed(project_path),
        ViewMessage::WorkSpacePickerToggled => domain::Message::WorkSpacePickerToggled,
        ViewMessage::WorkSpacePickerClosed => domain::Message::WorkSpacePickerClosed,
        _ => return Task::none(),
    };
    let result = domain::update(&mut app.navigation_sidebar, message);
    if let Some(collapsed) = result { return app.save_desktop_preference("work_sidebar_collapsed", collapsed.into()); }
    Task::none()
}

#[cfg(test)]
#[path = "work_sidebar_tests.rs"]
mod work_sidebar_tests;
