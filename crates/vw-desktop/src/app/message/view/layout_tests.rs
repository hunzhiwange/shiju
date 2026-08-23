// 验证主窗口与任务宠物窗口的关闭、移动及退出条件。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

// 编译接线：引用父模块符号，确保测试片段仍由真实模块包含。
// 该冒烟用例只承担接线职责，具体行为由同领域测试覆盖。
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("layout_tests"));
}

fn test_app() -> crate::app::App {
    crate::app::App::new().0
}

#[test]
fn main_window_closed_exits_even_when_task_pet_is_open() {
    let main_window_id = iced::window::Id::unique();
    let task_pet_window_id = iced::window::Id::unique();
    let mut main_window = Some(main_window_id);
    let mut task_pet_window = Some(task_pet_window_id);

    let outcome = super::mark_window_closed(&mut main_window, &mut task_pet_window, main_window_id);

    assert_eq!(outcome, super::WindowClosedOutcome::Exit);
    assert_eq!(main_window, None);
    assert_eq!(task_pet_window, None);
}

#[test]
fn task_pet_window_closed_keeps_main_window_running() {
    let main_window_id = iced::window::Id::unique();
    let task_pet_window_id = iced::window::Id::unique();
    let mut main_window = Some(main_window_id);
    let mut task_pet_window = Some(task_pet_window_id);

    let outcome =
        super::mark_window_closed(&mut main_window, &mut task_pet_window, task_pet_window_id);

    assert_eq!(outcome, super::WindowClosedOutcome::Continue);
    assert_eq!(main_window, Some(main_window_id));
    assert_eq!(task_pet_window, None);
}

#[test]
fn closing_last_or_already_absent_window_exits() {
    let task_pet_window_id = iced::window::Id::unique();
    let mut main_window = None;
    let mut task_pet_window = Some(task_pet_window_id);

    let task_pet_outcome =
        super::mark_window_closed(&mut main_window, &mut task_pet_window, task_pet_window_id);
    let absent_outcome = super::mark_window_closed(
        &mut main_window,
        &mut task_pet_window,
        iced::window::Id::unique(),
    );

    assert_eq!(task_pet_outcome, super::WindowClosedOutcome::Exit);
    assert_eq!(absent_outcome, super::WindowClosedOutcome::Exit);
    assert_eq!(main_window, None);
    assert_eq!(task_pet_window, None);
}

#[test]
fn closing_unknown_window_preserves_open_windows() {
    let main_window_id = iced::window::Id::unique();
    let task_pet_window_id = iced::window::Id::unique();
    let mut main_window = Some(main_window_id);
    let mut task_pet_window = Some(task_pet_window_id);

    let outcome = super::mark_window_closed(
        &mut main_window,
        &mut task_pet_window,
        iced::window::Id::unique(),
    );

    assert_eq!(outcome, super::WindowClosedOutcome::Continue);
    assert_eq!(main_window, Some(main_window_id));
    assert_eq!(task_pet_window, Some(task_pet_window_id));
}

#[test]
fn window_moved_only_updates_main_window_position() {
    let mut app = test_app();
    let main_window_id = iced::window::Id::unique();
    app.main_window_id = Some(main_window_id);
    app.window_position = (4.0, 8.0);

    let _task = super::update(
        &mut app,
        super::ViewMessage::WindowMoved(iced::window::Id::unique(), 20.0, 30.0),
    );
    assert_eq!(app.window_position, (4.0, 8.0));

    let _task =
        super::update(&mut app, super::ViewMessage::WindowMoved(main_window_id, 20.0, 30.0));
    assert_eq!(app.window_position, (20.0, 30.0));
}

#[test]
fn task_pet_window_moved_updates_pet_position_without_moving_main_window() {
    let mut app = test_app();
    let task_pet_window_id = iced::window::Id::unique();
    app.task_pet_window_id = Some(task_pet_window_id);
    app.window_position = (4.0, 8.0);

    let _task =
        super::update(&mut app, super::ViewMessage::WindowMoved(task_pet_window_id, 120.0, 240.0));

    assert_eq!(app.task_pet_position, iced::Point::new(120.0, 240.0));
    assert_eq!(app.window_position, (4.0, 8.0));
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn drag_start_messages_capture_current_dimensions_and_reset_anchors() {
    let mut app = test_app();
    app.split_ratio = 0.65;
    app.split_drag_anchor_x = Some(1.0);
    app.settings_panel_width = 420.0;
    app.settings_drag_anchor_x = Some(2.0);
    app.file_manager_width = 310.0;
    app.file_manager_drag_anchor_x = Some(3.0);

    let _task = super::update(&mut app, super::ViewMessage::SplitDragStarted);
    let _task = super::update(&mut app, super::ViewMessage::SettingsDragStarted);
    let _task = super::update(&mut app, super::ViewMessage::FileManagerDragStarted);

    assert!(app.dragging_split);
    assert_eq!(app.split_drag_anchor_x, None);
    assert_eq!(app.split_drag_start_ratio, 0.65);
    assert!(app.dragging_settings);
    assert_eq!(app.settings_drag_anchor_x, None);
    assert_eq!(app.settings_drag_start_width, 420.0);
    assert!(app.dragging_file_manager);
    assert_eq!(app.file_manager_drag_anchor_x, None);
    assert_eq!(app.file_manager_start_width, 310.0);
}

#[test]
fn work_drag_start_messages_capture_sidebar_width_and_split_ratio() {
    let mut app = test_app();
    app.work_sidebar_width = 340.0;
    app.work_sidebar_drag_anchor_x = Some(1.0);
    app.work_split_ratio = 0.55;
    app.work_split_drag_anchor_x = Some(2.0);

    let _task = super::update(&mut app, super::ViewMessage::WorkSidebarDragStarted);
    let _task = super::update(&mut app, super::ViewMessage::WorkSplitDragStarted);

    assert!(app.dragging_work_sidebar);
    assert_eq!(app.work_sidebar_drag_anchor_x, None);
    assert_eq!(app.work_sidebar_drag_start_width, 340.0);
    assert!(app.dragging_work_split);
    assert_eq!(app.work_split_drag_anchor_x, None);
    assert_eq!(app.work_split_drag_start_ratio, 0.55);
}

#[test]
fn work_drag_pointer_updates_sidebar_width_and_chat_split() {
    let mut app = test_app();
    app.window_size = (1_200.0, 800.0);
    app.work_sidebar_width = 300.0;
    app.work_split_ratio = 0.6;

    let _task = super::update(&mut app, super::ViewMessage::WorkSidebarDragStarted);
    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(100.0, 10.0));
    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(-1_000.0, 10.0));
    assert_eq!(app.work_sidebar_width, 220.0);
    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(1_000.0, 10.0));
    assert_eq!(app.work_sidebar_width, 520.0);

    app.dragging_work_sidebar = false;
    let _task = super::update(&mut app, super::ViewMessage::WorkSplitDragStarted);
    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(100.0, 10.0));
    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(1_000.0, 10.0));
    assert_eq!(app.work_split_ratio, 0.75);
    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(-1_000.0, 10.0));
    assert_eq!(app.work_split_ratio, 0.25);
}

#[test]
fn settings_drag_anchors_then_clamps_panel_width() {
    let mut app = test_app();
    app.settings_panel_width = 400.0;
    let _task = super::update(&mut app, super::ViewMessage::SettingsDragStarted);

    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(100.0, 10.0));
    assert_eq!(app.settings_drag_anchor_x, Some(100.0));
    assert_eq!(app.settings_panel_width, 400.0);

    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(600.0, 20.0));
    assert_eq!(app.cursor_position, iced::Point::new(600.0, 20.0));
    assert_eq!(app.settings_panel_width, 800.0);
}

#[test]
fn file_manager_drag_anchors_then_clamps_inverse_delta() {
    let mut app = test_app();
    app.file_manager_width = 300.0;
    let _task = super::update(&mut app, super::ViewMessage::FileManagerDragStarted);

    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(500.0, 10.0));
    assert_eq!(app.file_manager_drag_anchor_x, Some(500.0));
    assert_eq!(app.file_manager_width, 300.0);

    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(100.0, 20.0));
    assert_eq!(app.file_manager_width, 600.0);
}

#[test]
fn split_drag_uses_window_width_and_clamps_ratio() {
    let mut app = test_app();
    app.split_ratio = 0.6;
    app.window_size = (1_000.0, 700.0);
    let _task = super::update(&mut app, super::ViewMessage::SplitDragStarted);

    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(100.0, 10.0));
    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(1_000.0, 10.0));
    assert_eq!(app.split_ratio, 0.8);

    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(-1_000.0, 10.0));
    assert_eq!(app.split_ratio, 0.2);
}

#[test]
fn terminal_drag_anchors_then_clamps_height_to_window() {
    let mut app = test_app();
    app.window_size = (800.0, 500.0);
    app.terminal.is_dragging = true;
    app.terminal.drag_anchor_y = None;
    app.terminal.drag_start_height = 300.0;

    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(10.0, 400.0));
    assert_eq!(app.terminal.drag_anchor_y, Some(400.0));

    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(10.0, 100.0));
    assert_eq!(app.terminal.height, 400.0);

    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(10.0, 1_000.0));
    assert_eq!(app.terminal.height, 120.0);
}

#[test]
fn task_board_drag_starts_only_after_threshold_is_exceeded() {
    let mut app = test_app();
    app.task_board_drag_pending =
        Some(("task-1".to_owned(), crate::app::task::TaskStatus::Planning, iced::Point::ORIGIN));

    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(3.0, 4.0));
    assert!(app.task_board_drag_pending.is_some());
    assert!(app.task_board_dragging.is_none());

    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(6.0, 0.0));
    assert!(app.task_board_drag_pending.is_none());
    assert_eq!(
        app.task_board_dragging,
        Some(("task-1".to_owned(), crate::app::task::TaskStatus::Planning))
    );
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn window_resize_ignores_other_windows_and_clamps_main_dimensions() {
    let mut app = test_app();
    let main_window_id = iced::window::Id::unique();
    app.main_window_id = Some(main_window_id);
    app.window_size = (900.0, 700.0);

    let _task = super::update(
        &mut app,
        super::ViewMessage::WindowResized(iced::window::Id::unique(), 0.0, -5.0),
    );
    assert_eq!(app.window_size, (900.0, 700.0));
    assert!(!app.startup_resize_checked);

    let _task =
        super::update(&mut app, super::ViewMessage::WindowResized(main_window_id, 0.0, -5.0));
    assert_eq!(app.window_size, (1.0, 1.0));
    assert!(app.startup_resize_checked);
}

#[test]
fn window_resize_during_fullscreen_settling_does_not_consume_startup_check() {
    let mut app = test_app();
    let main_window_id = iced::window::Id::unique();
    app.main_window_id = Some(main_window_id);
    app.fullscreen_layout_settling = true;

    let _task =
        super::update(&mut app, super::ViewMessage::WindowResized(main_window_id, 640.0, 480.0));

    assert_eq!(app.window_size, (640.0, 480.0));
    assert!(!app.startup_resize_checked);
}

#[test]
fn hovered_file_paths_are_unique_and_blocked_during_internal_drag() {
    let mut app = test_app();

    let _task = super::update(&mut app, super::ViewMessage::HoveredFilePath("first.rs".to_owned()));
    app.pending_drop_file_position = Some((2, 3));
    let _task = super::update(&mut app, super::ViewMessage::HoveredFilePath("first.rs".to_owned()));
    app.dragging_file_paths.push("internal.rs".to_owned());
    let _task =
        super::update(&mut app, super::ViewMessage::HoveredFilePath("blocked.rs".to_owned()));

    assert_eq!(app.pending_drop_file_paths, vec!["first.rs"]);
    assert_eq!(app.pending_drop_file_position, Some((2, 3)));
}

#[test]
fn hovered_files_left_clears_external_hover_but_preserves_internal_drag() {
    let mut app = test_app();
    app.pending_drop_file_paths = vec!["external.rs".to_owned()];
    app.pending_drop_file_position = Some((1, 2));
    app.input_drop_hovered = true;

    let _task = super::update(&mut app, super::ViewMessage::HoveredFilesLeft);
    assert!(app.pending_drop_file_paths.is_empty());
    assert_eq!(app.pending_drop_file_position, None);
    assert!(!app.input_drop_hovered);

    app.dragging_file_paths = vec!["internal.rs".to_owned()];
    app.pending_drop_file_paths = vec!["kept.rs".to_owned()];
    app.input_drop_hovered = true;
    let _task = super::update(&mut app, super::ViewMessage::HoveredFilesLeft);
    assert_eq!(app.pending_drop_file_paths, vec!["kept.rs"]);
    assert!(app.input_drop_hovered);
}

#[test]
fn fullscreen_layout_settled_clears_settling_flag() {
    let mut app = test_app();
    app.fullscreen_layout_settling = true;

    let _task = super::update(&mut app, super::ViewMessage::FullscreenLayoutSettled);

    assert!(!app.fullscreen_layout_settling);
}

#[test]
fn global_release_resets_active_drag_state_without_file_manager_persistence() {
    let mut app = test_app();
    app.dragging_settings = true;
    app.settings_drag_anchor_x = Some(10.0);
    app.dragging_split = true;
    app.dragging_work_sidebar = true;
    app.work_sidebar_drag_anchor_x = Some(30.0);
    app.dragging_work_split = true;
    app.work_split_drag_anchor_x = Some(40.0);
    app.terminal.is_dragging = true;
    app.terminal.drag_anchor_y = Some(20.0);
    app.task_board_drag_pending =
        Some(("pending".to_owned(), crate::app::task::TaskStatus::Pending, iced::Point::ORIGIN));
    app.task_board_dragging = Some(("dragging".to_owned(), crate::app::task::TaskStatus::Running));

    let _task = super::update(&mut app, super::ViewMessage::GlobalMouseReleased);

    assert!(app.task_board_drag_pending.is_none());
    assert!(app.task_board_dragging.is_none());
    assert!(!app.dragging_settings);
    assert_eq!(app.settings_drag_anchor_x, None);
    assert!(!app.dragging_split);
    assert!(!app.dragging_work_sidebar);
    assert_eq!(app.work_sidebar_drag_anchor_x, None);
    assert!(!app.dragging_work_split);
    assert_eq!(app.work_split_drag_anchor_x, None);
    assert!(!app.terminal.is_dragging);
    assert_eq!(app.terminal.drag_anchor_y, None);
}

#[test]
fn global_release_transfers_internal_files_to_hovered_input() {
    let mut app = test_app();
    app.dragging_file_paths = vec!["src/main.rs".to_owned()];
    app.dragging_file_position = Some((4, 5));
    app.input_drop_hovered = true;
    app.focus_area = crate::app::FocusArea::Preview;

    let _task = super::update(&mut app, super::ViewMessage::GlobalMouseReleased);

    assert!(app.dragging_file_paths.is_empty());
    assert_eq!(app.dragging_file_position, None);
    assert_eq!(app.pending_drop_file_paths, vec!["src/main.rs"]);
    assert_eq!(app.pending_drop_file_position, Some((4, 5)));
    assert!(!app.input_drop_hovered);
    assert_eq!(app.focus_area, crate::app::FocusArea::None);
}

#[cfg(not(target_arch = "wasm32"))]
// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn global_cursor_left_makes_visible_lsp_hover_noninteractive_and_schedules_hide() {
    let mut app = test_app();
    app.lsp_overlay.hover_visible = true;
    app.lsp_overlay.hover_interactive = true;
    app.lsp_hover_hide_deadline = None;

    let _task = super::update(&mut app, super::ViewMessage::GlobalCursorLeft);

    assert!(!app.lsp_overlay.hover_interactive);
    assert!(app.lsp_hover_hide_deadline.is_some());
}
