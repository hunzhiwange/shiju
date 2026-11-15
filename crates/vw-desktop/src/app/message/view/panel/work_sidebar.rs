use super::ViewMessage;
use crate::app::{App, Message, Screen};
use iced::Task;
use desktop_navigation::panel::work_sidebar as domain;

pub(super) fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    if matches!(message, ViewMessage::WorkProjectHome) {
        if !matches!(app.screen, Screen::Project) || app.project_path.is_none() {
            return Task::none();
        }
        app.work_resource_half_fullscreen = false;
        if app.active_session_id.is_some() {
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
    let mut state = domain::State {
        work_sidebar_visible_tasks: std::mem::take(&mut app.work_sidebar_visible_tasks),
        work_sidebar_visible_spaces: std::mem::take(&mut app.work_sidebar_visible_spaces),
        work_sidebar_space_task_visible: std::mem::take(&mut app.work_sidebar_space_task_visible),
        work_sidebar_project_nav_hovered: std::mem::take(&mut app.work_sidebar_project_nav_hovered),
        work_sidebar_filter_open: std::mem::take(&mut app.work_sidebar_filter_open),
        work_sidebar_more_open: std::mem::take(&mut app.work_sidebar_more_open),
        work_sidebar_collapsed: std::mem::take(&mut app.work_sidebar_collapsed),
        work_sidebar_filter_scope: std::mem::take(&mut app.work_sidebar_filter_scope),
        work_sidebar_filter_status: std::mem::take(&mut app.work_sidebar_filter_status),
        work_sidebar_filter_time: std::mem::take(&mut app.work_sidebar_filter_time),
        work_sidebar_tab: std::mem::take(&mut app.work_sidebar_tab),
        work_space_picker_open: std::mem::take(&mut app.work_space_picker_open),
    };
    let result = domain::update(&mut state, message);
    app.work_sidebar_visible_tasks = state.work_sidebar_visible_tasks;
    app.work_sidebar_visible_spaces = state.work_sidebar_visible_spaces;
    app.work_sidebar_space_task_visible = state.work_sidebar_space_task_visible;
    app.work_sidebar_project_nav_hovered = state.work_sidebar_project_nav_hovered;
    app.work_sidebar_filter_open = state.work_sidebar_filter_open;
    app.work_sidebar_more_open = state.work_sidebar_more_open;
    app.work_sidebar_collapsed = state.work_sidebar_collapsed;
    app.work_sidebar_filter_scope = state.work_sidebar_filter_scope;
    app.work_sidebar_filter_status = state.work_sidebar_filter_status;
    app.work_sidebar_filter_time = state.work_sidebar_filter_time;
    app.work_sidebar_tab = state.work_sidebar_tab;
    app.work_space_picker_open = state.work_space_picker_open;
    if let Some(collapsed) = result { return app.save_desktop_preference("work_sidebar_collapsed", collapsed.into()); }
    Task::none()
}

#[cfg(test)]
#[path = "work_sidebar_tests.rs"]
mod work_sidebar_tests;
