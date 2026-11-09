//! 处理工作模式左栏的收起与过滤交互。

use super::ViewMessage;
use crate::app::state::{
    WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS, WORK_SIDEBAR_LOAD_MORE_STEP,
    WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE, WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP,
    WorkSidebarFilterScope, WorkSidebarFilterStatus, WorkSidebarFilterTime, WorkSidebarTab,
};
use crate::app::{App, Message, Screen};
use iced::Task;

fn reset_visible_items(app: &mut App) {
    app.work_sidebar_visible_tasks = WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS;
    app.work_sidebar_visible_spaces = WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS;
    app.work_sidebar_space_task_visible.clear();
}

pub(super) fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::WorkSidebarToggled => {
            app.work_sidebar_project_nav_hovered = false;
            app.work_sidebar_filter_open = false;
            app.work_sidebar_more_open = false;
            app.work_sidebar_collapsed = !app.work_sidebar_collapsed;
            return app.save_desktop_preference(
                "work_sidebar_collapsed",
                app.work_sidebar_collapsed.into(),
            );
        }
        ViewMessage::WorkProjectHome => {
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
        }
        ViewMessage::WorkSidebarFilterToggled => {
            app.work_sidebar_filter_open = !app.work_sidebar_filter_open;
            app.work_sidebar_more_open = false;
        }
        ViewMessage::WorkSidebarFilterClosed => {
            app.work_sidebar_filter_open = false;
        }
        ViewMessage::WorkSidebarMoreOpened => {
            app.work_sidebar_more_open = true;
            app.work_sidebar_filter_open = false;
        }
        ViewMessage::WorkSidebarMoreClosed => {
            app.work_sidebar_more_open = false;
        }
        ViewMessage::WorkSidebarProjectNavHovered(hovered) => {
            app.work_sidebar_project_nav_hovered = hovered;
        }
        ViewMessage::WorkSidebarFilterScopeSelected(scope) => {
            app.work_sidebar_filter_scope = scope;
            reset_visible_items(app);
        }
        ViewMessage::WorkSidebarFilterStatusSelected(status) => {
            app.work_sidebar_filter_status = status;
            reset_visible_items(app);
        }
        ViewMessage::WorkSidebarFilterTimeSelected(time) => {
            app.work_sidebar_filter_time = time;
            reset_visible_items(app);
        }
        ViewMessage::WorkSidebarFilterReset => {
            app.work_sidebar_filter_scope = WorkSidebarFilterScope::All;
            app.work_sidebar_filter_status = WorkSidebarFilterStatus::All;
            app.work_sidebar_filter_time = WorkSidebarFilterTime::All;
            reset_visible_items(app);
        }
        ViewMessage::WorkSidebarTabSelected(tab) => {
            app.work_sidebar_tab = tab;
        }
        ViewMessage::WorkSidebarLoadMorePressed => match app.work_sidebar_tab {
            WorkSidebarTab::Tasks => {
                app.work_sidebar_visible_tasks =
                    app.work_sidebar_visible_tasks.saturating_add(WORK_SIDEBAR_LOAD_MORE_STEP);
            }
            WorkSidebarTab::Spaces => {
                app.work_sidebar_visible_spaces =
                    app.work_sidebar_visible_spaces.saturating_add(WORK_SIDEBAR_LOAD_MORE_STEP);
            }
        },
        ViewMessage::WorkSidebarSpaceTasksLoadMorePressed(project_path) => {
            let current = app
                .work_sidebar_space_task_visible
                .get(&project_path)
                .copied()
                .unwrap_or(WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE);
            app.work_sidebar_space_task_visible.insert(
                project_path,
                current.saturating_add(WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP),
            );
        }
        ViewMessage::WorkSpacePickerToggled => {
            app.work_space_picker_open = !app.work_space_picker_open;
        }
        ViewMessage::WorkSpacePickerClosed => {
            app.work_space_picker_open = false;
        }
        _ => {}
    }

    Task::none()
}

#[cfg(test)]
#[path = "work_sidebar_tests.rs"]
mod work_sidebar_tests;
