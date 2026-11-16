use crate::view_layout::drag::{self, Message};
#[derive(Default)]
struct Fixture {
    layout_drag: drag::State,
    navigation_workspace: crate::panel::work_workspace::State,
    window_size: (f32, f32),
}
fn test_app() -> Fixture {
    Fixture::default()
}
fn update(app: &mut Fixture, message: Message) -> bool {
    drag::update(
        &mut app.layout_drag,
        &mut app.navigation_workspace,
        false,
        app.window_size.0,
        message,
    )
}

#[test]
fn drag_start_messages_capture_current_dimensions_and_reset_anchors() {
    let mut app = test_app();
    app.layout_drag.split_ratio = 0.65;
    app.layout_drag.split_drag_anchor_x = Some(1.0);
    app.layout_drag.settings_panel_width = 420.0;
    app.layout_drag.settings_drag_anchor_x = Some(2.0);
    app.layout_drag.file_manager_width = 310.0;
    app.layout_drag.file_manager_drag_anchor_x = Some(3.0);

    let _task = update(&mut app, Message::SplitDragStarted);
    let _task = update(&mut app, Message::SettingsDragStarted);
    let _task = update(&mut app, Message::FileManagerDragStarted);

    assert!(app.layout_drag.dragging_split);
    assert_eq!(app.layout_drag.split_drag_anchor_x, None);
    assert_eq!(app.layout_drag.split_drag_start_ratio, 0.65);
    assert!(app.layout_drag.dragging_settings);
    assert_eq!(app.layout_drag.settings_drag_anchor_x, None);
    assert_eq!(app.layout_drag.settings_drag_start_width, 420.0);
    assert!(app.layout_drag.dragging_file_manager);
    assert_eq!(app.layout_drag.file_manager_drag_anchor_x, None);
    assert_eq!(app.layout_drag.file_manager_start_width, 310.0);
}

#[test]
fn work_drag_start_messages_capture_sidebar_width_and_split_ratio() {
    let mut app = test_app();
    app.layout_drag.work_sidebar_width = 340.0;
    app.layout_drag.work_sidebar_drag_anchor_x = Some(1.0);
    app.navigation_workspace.work_split_ratio = 0.55;
    app.navigation_workspace.work_split_drag_anchor_x = Some(2.0);

    let _task = update(&mut app, Message::WorkSidebarDragStarted);
    let _task = update(&mut app, Message::WorkSplitDragStarted);

    assert!(app.layout_drag.dragging_work_sidebar);
    assert_eq!(app.layout_drag.work_sidebar_drag_anchor_x, None);
    assert_eq!(app.layout_drag.work_sidebar_drag_start_width, 340.0);
    assert!(app.navigation_workspace.dragging_work_split);
    assert_eq!(app.navigation_workspace.work_split_drag_anchor_x, None);
    assert_eq!(app.layout_drag.work_split_drag_start_ratio, 0.55);
}

#[test]
fn work_drag_pointer_updates_sidebar_width_and_chat_split() {
    let mut app = test_app();
    app.window_size = (1_200.0, 800.0);
    app.layout_drag.work_sidebar_width = 300.0;
    app.navigation_workspace.work_split_ratio = 0.6;

    let _task = update(&mut app, Message::WorkSidebarDragStarted);
    let _task = update(&mut app, Message::PointerMoved(100.0, 10.0));
    let _task = update(&mut app, Message::PointerMoved(-1_000.0, 10.0));
    assert_eq!(app.layout_drag.work_sidebar_width, 220.0);
    let _task = update(&mut app, Message::PointerMoved(1_000.0, 10.0));
    assert_eq!(app.layout_drag.work_sidebar_width, 520.0);

    app.layout_drag.dragging_work_sidebar = false;
    let _task = update(&mut app, Message::WorkSplitDragStarted);
    let _task = update(&mut app, Message::PointerMoved(100.0, 10.0));
    let _task = update(&mut app, Message::PointerMoved(1_000.0, 10.0));
    assert_eq!(app.navigation_workspace.work_split_ratio, 0.75);
    let _task = update(&mut app, Message::PointerMoved(-1_000.0, 10.0));
    assert_eq!(app.navigation_workspace.work_split_ratio, 0.25);
}

#[test]
fn file_manager_drag_anchors_then_clamps_inverse_delta() {
    let mut app = test_app();
    app.layout_drag.file_manager_width = 300.0;
    let _task = update(&mut app, Message::FileManagerDragStarted);

    let _task = update(&mut app, Message::PointerMoved(500.0, 10.0));
    assert_eq!(app.layout_drag.file_manager_drag_anchor_x, Some(500.0));
    assert_eq!(app.layout_drag.file_manager_width, 300.0);

    let _task = update(&mut app, Message::PointerMoved(100.0, 20.0));
    assert_eq!(app.layout_drag.file_manager_width, 600.0);
}

#[test]
fn split_drag_uses_window_width_and_clamps_ratio() {
    let mut app = test_app();
    app.layout_drag.split_ratio = 0.6;
    app.window_size = (1_000.0, 700.0);
    let _task = update(&mut app, Message::SplitDragStarted);

    let _task = update(&mut app, Message::PointerMoved(100.0, 10.0));
    let _task = update(&mut app, Message::PointerMoved(1_000.0, 10.0));
    assert_eq!(app.layout_drag.split_ratio, 0.8);

    let _task = update(&mut app, Message::PointerMoved(-1_000.0, 10.0));
    assert_eq!(app.layout_drag.split_ratio, 0.2);
}
