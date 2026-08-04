use super::super::{ViewMessage, update as update_panel};
use super::WORK_SIDEBAR_COLLAPSED_WIDTH;
use crate::app::App;
use crate::app::state::{
    WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS, WORK_SIDEBAR_LOAD_MORE_STEP, WorkSidebarFilterScope,
    WorkSidebarFilterStatus, WorkSidebarFilterTime, WorkSidebarTab,
};

#[test]
fn work_sidebar_toggle_preserves_expanded_width() {
    let mut app = App::new().0;
    app.work_sidebar_width = 360.0;

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarToggled);
    assert!(app.work_sidebar_collapsed);
    assert_eq!(app.work_sidebar_width, WORK_SIDEBAR_COLLAPSED_WIDTH);
    assert_eq!(app.work_sidebar_expanded_width, 360.0);

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarToggled);
    assert!(!app.work_sidebar_collapsed);
    assert_eq!(app.work_sidebar_width, 360.0);
}

#[test]
fn work_sidebar_filter_messages_select_and_reset_real_filters() {
    let mut app = App::new().0;

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarFilterToggled);
    assert!(app.work_sidebar_filter_open);

    let _task = update_panel(
        &mut app,
        ViewMessage::WorkSidebarFilterScopeSelected(WorkSidebarFilterScope::CurrentProject),
    );
    let _task = update_panel(
        &mut app,
        ViewMessage::WorkSidebarFilterStatusSelected(WorkSidebarFilterStatus::InProgress),
    );
    let _task = update_panel(
        &mut app,
        ViewMessage::WorkSidebarFilterTimeSelected(WorkSidebarFilterTime::LastSevenDays),
    );
    assert_eq!(app.work_sidebar_filter_scope, WorkSidebarFilterScope::CurrentProject);
    assert_eq!(app.work_sidebar_filter_status, WorkSidebarFilterStatus::InProgress);
    assert_eq!(app.work_sidebar_filter_time, WorkSidebarFilterTime::LastSevenDays);

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarFilterReset);
    assert_eq!(app.work_sidebar_filter_scope, WorkSidebarFilterScope::All);
    assert_eq!(app.work_sidebar_filter_status, WorkSidebarFilterStatus::All);
    assert_eq!(app.work_sidebar_filter_time, WorkSidebarFilterTime::All);

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarFilterClosed);
    assert!(!app.work_sidebar_filter_open);
}

#[test]
fn work_sidebar_tabs_and_load_more_keep_independent_limits() {
    let mut app = App::new().0;
    assert_eq!(app.work_sidebar_tab, WorkSidebarTab::Tasks);
    assert_eq!(app.work_sidebar_visible_tasks, WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS);
    assert_eq!(app.work_sidebar_visible_spaces, WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS);

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarLoadMorePressed);
    assert_eq!(
        app.work_sidebar_visible_tasks,
        WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS + WORK_SIDEBAR_LOAD_MORE_STEP
    );
    assert_eq!(app.work_sidebar_visible_spaces, WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS);

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarTabSelected(WorkSidebarTab::Spaces));
    let _task = update_panel(&mut app, ViewMessage::WorkSidebarLoadMorePressed);
    assert_eq!(
        app.work_sidebar_visible_spaces,
        WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS + WORK_SIDEBAR_LOAD_MORE_STEP
    );

    let _task = update_panel(
        &mut app,
        ViewMessage::WorkSidebarFilterStatusSelected(WorkSidebarFilterStatus::Completed),
    );
    assert_eq!(app.work_sidebar_visible_tasks, WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS);
    assert_eq!(app.work_sidebar_visible_spaces, WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS);
}

#[test]
fn work_space_picker_messages_open_and_close_popover() {
    let mut app = App::new().0;

    let _task = update_panel(&mut app, ViewMessage::WorkSpacePickerToggled);
    assert!(app.work_space_picker_open);

    let _task = update_panel(&mut app, ViewMessage::WorkSpacePickerClosed);
    assert!(!app.work_space_picker_open);
}

#[test]
fn work_sidebar_more_and_filter_popovers_are_mutually_exclusive() {
    let mut app = App::new().0;

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarMoreOpened);
    assert!(app.work_sidebar_more_open);
    assert!(!app.work_sidebar_filter_open);

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarFilterToggled);
    assert!(!app.work_sidebar_more_open);
    assert!(app.work_sidebar_filter_open);

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarMoreOpened);
    let _task = update_panel(&mut app, ViewMessage::WorkSidebarMoreClosed);
    assert!(!app.work_sidebar_more_open);
}
