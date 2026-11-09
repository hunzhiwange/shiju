//! 工作区侧栏消息测试，验证折叠宽度、筛选器、分页与空间任务状态。
//! 各标签和弹出层应保持独立，切换视图时不得串用可见数量或悬停状态。

use super::super::{ViewMessage, update as update_panel};
use crate::app::models::{ChatMessage, ChatRole};
use crate::app::state::{
    WORK_SIDEBAR_INITIAL_VISIBLE_ITEMS, WORK_SIDEBAR_LOAD_MORE_STEP,
    WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE, WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP,
    WorkSidebarFilterScope, WorkSidebarFilterStatus, WorkSidebarFilterTime, WorkSidebarTab,
};
use crate::app::{App, Screen};

#[test]
fn work_sidebar_toggle_preserves_session_list_width() {
    let mut app = App::new().0;
    app.work_sidebar_width = 360.0;

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarToggled);
    assert!(app.work_sidebar_collapsed);
    assert_eq!(app.work_sidebar_width, 360.0);

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarMoreOpened);
    assert!(app.work_sidebar_more_open);
    let _task = update_panel(&mut app, ViewMessage::WorkSidebarToggled);
    assert!(!app.work_sidebar_collapsed);
    assert!(!app.work_sidebar_more_open);
    assert_eq!(app.work_sidebar_width, 360.0);
}

#[test]
fn work_sidebar_project_home_leaves_the_session_without_leaving_the_project() {
    let mut app = App::new().0;
    app.screen = Screen::Project;
    app.project_path = Some("/workspace/project".into());
    app.active_tab_id = Some("project_/workspace/project".into());
    app.active_session_id = Some("session-1".into());
    app.work_sidebar_collapsed = true;
    app.work_sidebar_tab = WorkSidebarTab::Spaces;
    app.work_resource_half_fullscreen = true;
    app.chat.push(ChatMessage {
        role: ChatRole::User,
        content: "Existing conversation".into(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });

    let _task = crate::app::message::view::update(&mut app, ViewMessage::WorkProjectHome);

    assert_eq!(app.screen, Screen::Project);
    assert_eq!(app.project_path.as_deref(), Some("/workspace/project"));
    assert_eq!(app.active_tab_id.as_deref(), Some("project_/workspace/project"));
    assert!(app.active_session_id.is_none());
    assert!(app.chat.is_empty());
    assert!(app.session_view_cache.contains("session-1"));
    assert!(app.work_sidebar_collapsed);
    assert!(!app.work_resource_half_fullscreen);
    assert_eq!(app.work_sidebar_tab, WorkSidebarTab::Spaces);
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
fn work_sidebar_space_tasks_load_more_increments_per_space_and_resets_on_filter() {
    let mut app = App::new().0;
    let path = "/workspace/alpha".to_string();

    let _task =
        update_panel(&mut app, ViewMessage::WorkSidebarSpaceTasksLoadMorePressed(path.clone()));
    assert_eq!(
        app.work_sidebar_space_task_visible.get(&path),
        Some(&(WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE + WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP))
    );

    let _task =
        update_panel(&mut app, ViewMessage::WorkSidebarSpaceTasksLoadMorePressed(path.clone()));
    assert_eq!(
        app.work_sidebar_space_task_visible.get(&path),
        Some(
            &(WORK_SIDEBAR_SPACE_TASKS_INITIAL_VISIBLE
                + WORK_SIDEBAR_SPACE_TASKS_LOAD_MORE_STEP * 2)
        )
    );

    let other = "/workspace/beta".to_string();
    let _task =
        update_panel(&mut app, ViewMessage::WorkSidebarSpaceTasksLoadMorePressed(other.clone()));
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

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarFilterReset);
    assert!(app.work_sidebar_space_task_visible.is_empty());
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

#[test]
fn work_sidebar_project_nav_hover_shows_create_entry_and_clears_on_collapse() {
    let mut app = App::new().0;
    assert!(!app.work_sidebar_project_nav_hovered);

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarProjectNavHovered(true));
    assert!(app.work_sidebar_project_nav_hovered);

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarProjectNavHovered(false));
    assert!(!app.work_sidebar_project_nav_hovered);

    let _task = update_panel(&mut app, ViewMessage::WorkSidebarProjectNavHovered(true));
    let _task = update_panel(&mut app, ViewMessage::WorkSidebarToggled);
    assert!(app.work_sidebar_collapsed);
    assert!(!app.work_sidebar_project_nav_hovered);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn work_sidebar_updates_before_gateway_reply() {
    use crate::app::{Message, config};
    use iced_runtime::futures::futures::StreamExt;
    use serde_json::json;
    use std::sync::mpsc;
    use std::time::Duration;

    const TIMEOUT: Duration = Duration::from_secs(3);

    config::with_editor_settings_gateway(|mut app, requests, responses| {
        app.work_sidebar_collapsed = false;
        let (states, observed_states) = mpsc::channel();
        let (click_again, second_click) = mpsc::channel();
        let gateway = std::thread::spawn(move || {
            let first = requests.recv_timeout(TIMEOUT).expect("first PATCH");
            let collapsed = observed_states.recv_timeout(TIMEOUT);
            click_again.send(()).unwrap();
            let expanded = collapsed.is_ok().then(|| observed_states.recv_timeout(TIMEOUT));
            // Release the response even on failure so a synchronous handler can return.
            responses.send(Some("{}".into())).unwrap();
            assert_eq!(collapsed, Ok(true), "first update must return before the PATCH reply");
            assert_eq!(
                expanded,
                Some(Ok(false)),
                "second update must return before the PATCH reply"
            );

            let second = requests.recv_timeout(TIMEOUT).expect("second PATCH");
            responses.send(Some("{}".into())).unwrap();
            for (request, value) in [(first, true), (second, false)] {
                assert!(request.starts_with("PATCH /v1/desktop/preferences "));
                let body: serde_json::Value =
                    serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
                assert_eq!(body, json!({"work_sidebar_collapsed": value}));
            }
        });

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let first = update_panel(&mut app, ViewMessage::WorkSidebarToggled);
        states.send(app.work_sidebar_collapsed).ok();
        let save = |task| {
            iced_runtime::task::into_stream(task).map(|mut actions| {
                runtime.spawn(async move {
                    while let Some(action) = actions.next().await {
                        if let iced_runtime::Action::Output(message) = action {
                            return message;
                        }
                    }
                    panic!("expected a save completion");
                })
            })
        };
        let first = save(first);
        second_click.recv_timeout(TIMEOUT).unwrap();
        let mut next = update_panel(&mut app, ViewMessage::WorkSidebarToggled);
        states.send(app.work_sidebar_collapsed).ok();

        if let Some(first) = first {
            assert!(iced_runtime::task::into_stream(next).is_none());
            let mut pending = Some(first);
            while let Some(task) = pending {
                let message: Message = runtime
                    .block_on(async { tokio::time::timeout(TIMEOUT, task).await })
                    .unwrap()
                    .unwrap();
                next = app.update(message);
                pending = save(next);
            }
        }
        gateway.join().unwrap();
        assert!(!app.work_sidebar_collapsed);
    });
}
