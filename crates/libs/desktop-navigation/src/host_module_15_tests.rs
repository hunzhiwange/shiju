//! 由任务15迁移的完整局部侧栏消息行为。
use crate::panel::work_sidebar::{Message, State, update};
use crate::presentation::*;

#[test]
fn work_sidebar_filter_messages_select_and_reset_real_filters() {
    let mut app = State::default();

    let _task = update(&mut app, Message::WorkSidebarFilterToggled);
    assert!(app.work_sidebar_filter_open);

    let _task = update(
        &mut app,
        Message::WorkSidebarFilterScopeSelected(WorkSidebarFilterScope::CurrentProject),
    );
    let _task = update(
        &mut app,
        Message::WorkSidebarFilterStatusSelected(WorkSidebarFilterStatus::InProgress),
    );
    let _task = update(
        &mut app,
        Message::WorkSidebarFilterTimeSelected(WorkSidebarFilterTime::LastSevenDays),
    );
    assert_eq!(app.work_sidebar_filter_scope, WorkSidebarFilterScope::CurrentProject);
    assert_eq!(app.work_sidebar_filter_status, WorkSidebarFilterStatus::InProgress);
    assert_eq!(app.work_sidebar_filter_time, WorkSidebarFilterTime::LastSevenDays);

    let _task = update(&mut app, Message::WorkSidebarFilterReset);
    assert_eq!(app.work_sidebar_filter_scope, WorkSidebarFilterScope::All);
    assert_eq!(app.work_sidebar_filter_status, WorkSidebarFilterStatus::All);
    assert_eq!(app.work_sidebar_filter_time, WorkSidebarFilterTime::All);

    let _task = update(&mut app, Message::WorkSidebarFilterClosed);
    assert!(!app.work_sidebar_filter_open);
}

#[test]
fn work_sidebar_tabs_and_load_more_keep_independent_limits() {
    let mut app = State::default();
    assert_eq!(app.work_sidebar_tab, WorkSidebarTab::Tasks);
    assert_eq!(app.work_sidebar_visible_tasks, WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS);
    assert_eq!(app.work_sidebar_visible_spaces, WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS);

    let _task = update(&mut app, Message::WorkSidebarLoadMorePressed);
    assert_eq!(
        app.work_sidebar_visible_tasks,
        WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS + WORK_SIDEBAR_LOAD_MORE_STEP
    );
    assert_eq!(app.work_sidebar_visible_spaces, WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS);

    let _task = update(&mut app, Message::WorkSidebarTabSelected(WorkSidebarTab::Spaces));
    let _task = update(&mut app, Message::WorkSidebarLoadMorePressed);
    assert_eq!(
        app.work_sidebar_visible_spaces,
        WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS + WORK_SIDEBAR_LOAD_MORE_STEP
    );

    let _task = update(
        &mut app,
        Message::WorkSidebarFilterStatusSelected(WorkSidebarFilterStatus::Completed),
    );
    assert_eq!(app.work_sidebar_visible_tasks, WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS);
    assert_eq!(app.work_sidebar_visible_spaces, WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS);
}

#[test]
fn work_sidebar_space_tasks_load_more_increments_per_space_and_resets_on_filter() {
    let mut app = State::default();
    let path = "/workspace/alpha".to_string();

    let _task = update(&mut app, Message::WorkSidebarSpaceTasksLoadMorePressed(path.clone()));
    assert_eq!(
        app.work_sidebar_space_task_visible.get(&path),
        Some(&(WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE + WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP))
    );

    let _task = update(&mut app, Message::WorkSidebarSpaceTasksLoadMorePressed(path.clone()));
    assert_eq!(
        app.work_sidebar_space_task_visible.get(&path),
        Some(
            &(WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE
                + WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP * 2)
        )
    );

    let other = "/workspace/beta".to_string();
    let _task = update(&mut app, Message::WorkSidebarSpaceTasksLoadMorePressed(other.clone()));
    assert_eq!(
        app.work_sidebar_space_task_visible.get(&other),
        Some(&(WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE + WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP))
    );
    assert_eq!(
        app.work_sidebar_space_task_visible.get(&path),
        Some(
            &(WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE
                + WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP * 2)
        )
    );

    let _task = update(&mut app, Message::WorkSidebarFilterReset);
    assert!(app.work_sidebar_space_task_visible.is_empty());
}

#[test]
fn work_space_picker_messages_open_and_close_popover() {
    let mut app = State::default();

    let _task = update(&mut app, Message::WorkSpacePickerToggled);
    assert!(app.work_space_picker_open);

    let _task = update(&mut app, Message::WorkSpacePickerClosed);
    assert!(!app.work_space_picker_open);
}

#[test]
fn work_sidebar_more_and_filter_popovers_are_mutually_exclusive() {
    let mut app = State::default();

    let _task = update(&mut app, Message::WorkSidebarMoreOpened);
    assert!(app.work_sidebar_more_open);
    assert!(!app.work_sidebar_filter_open);

    let _task = update(&mut app, Message::WorkSidebarFilterToggled);
    assert!(!app.work_sidebar_more_open);
    assert!(app.work_sidebar_filter_open);

    let _task = update(&mut app, Message::WorkSidebarMoreOpened);
    let _task = update(&mut app, Message::WorkSidebarMoreClosed);
    assert!(!app.work_sidebar_more_open);
}

#[test]
fn work_sidebar_project_nav_hover_shows_create_entry_and_clears_on_collapse() {
    let mut app = State::default();
    assert!(!app.work_sidebar_project_nav_hovered);

    let _task = update(&mut app, Message::WorkSidebarProjectNavHovered(true));
    assert!(app.work_sidebar_project_nav_hovered);

    let _task = update(&mut app, Message::WorkSidebarProjectNavHovered(false));
    assert!(!app.work_sidebar_project_nav_hovered);

    let _task = update(&mut app, Message::WorkSidebarProjectNavHovered(true));
    let _task = update(&mut app, Message::WorkSidebarToggled);
    assert!(app.work_sidebar_collapsed);
    assert!(!app.work_sidebar_project_nav_hovered);
}

mod layout;

mod popovers;

mod tabs;
mod settings;

mod update;

mod geometry;
mod file_hover;

mod notifications;

mod external;

mod mobile;
