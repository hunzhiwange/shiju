//! 处理工作模式左栏的收起与过滤交互。

use super::ViewMessage;
use crate::app::state::{
    WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS, WORK_SIDEBAR_LOAD_MORE_STEP,
    WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE, WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP,
    WorkSidebarFilterScope, WorkSidebarFilterStatus, WorkSidebarFilterTime, WorkSidebarTab,
};
use crate::app::{App, Message};
use iced::Task;

pub(crate) const WORK_SIDEBAR_COLLAPSED_WIDTH: f32 = 54.0;
const MIN_WORK_SIDEBAR_WIDTH: f32 = 220.0;
const MAX_WORK_SIDEBAR_WIDTH: f32 = 520.0;

fn reset_visible_items(app: &mut App) {
    app.work_sidebar_visible_tasks = WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS;
    app.work_sidebar_visible_spaces = WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS;
    app.work_sidebar_space_task_visible.clear();
}

pub(super) fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::WorkSidebarToggled => {
            if app.work_sidebar_collapsed {
                app.work_sidebar_width = app
                    .work_sidebar_expanded_width
                    .clamp(MIN_WORK_SIDEBAR_WIDTH, MAX_WORK_SIDEBAR_WIDTH);
            } else {
                app.work_sidebar_expanded_width =
                    app.work_sidebar_width.clamp(MIN_WORK_SIDEBAR_WIDTH, MAX_WORK_SIDEBAR_WIDTH);
                app.work_sidebar_width = WORK_SIDEBAR_COLLAPSED_WIDTH;
                app.work_sidebar_filter_open = false;
                app.work_sidebar_more_open = false;
                app.work_sidebar_project_nav_hovered = false;
            }
            app.work_sidebar_collapsed = !app.work_sidebar_collapsed;
        }
        ViewMessage::WorkSidebarFilterToggled => {
            app.work_sidebar_filter_open = !app.work_sidebar_filter_open;
            app.work_sidebar_more_open = false;
        }
        ViewMessage::WorkSidebarFilterClosed => {
            app.work_sidebar_filter_open = false;
        }
        ViewMessage::WorkSidebarMoreToggled => {
            app.work_sidebar_more_open = !app.work_sidebar_more_open;
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
