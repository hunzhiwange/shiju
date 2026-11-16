// 宿主保留窗口事件、焦点、面板布局与领域状态投影；窗口关闭分类已迁入 desktop-navigation。
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

    assert_eq!(app.pet_state.task_pet_position, iced::Point::new(120.0, 240.0));
    assert_eq!(app.window_position, (4.0, 8.0));
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。

#[test]
fn settings_drag_anchors_then_clamps_panel_width() {
    let mut app = test_app();
    app.layout_drag.settings_panel_width = 400.0;
    let _task = super::update(&mut app, super::ViewMessage::SettingsDragStarted);

    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(100.0, 10.0));
    assert_eq!(app.layout_drag.settings_drag_anchor_x, Some(100.0));
    assert_eq!(app.layout_drag.settings_panel_width, 400.0);

    let _task = super::update(&mut app, super::ViewMessage::PointerMoved(600.0, 20.0));
    assert_eq!(app.cursor_position, iced::Point::new(600.0, 20.0));
    assert_eq!(app.layout_drag.settings_panel_width, 800.0);
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。



#[test]
fn global_release_resets_active_drag_state_without_file_manager_persistence() {
    let mut app = test_app();
    app.layout_drag.dragging_settings = true;
    app.layout_drag.settings_drag_anchor_x = Some(10.0);
    app.layout_drag.dragging_split = true;
    app.layout_drag.dragging_work_sidebar = true;
    app.layout_drag.work_sidebar_drag_anchor_x = Some(30.0);
    app.navigation_workspace.dragging_work_split = true;
    app.navigation_workspace.work_split_drag_anchor_x = Some(40.0);
    app.terminal.is_dragging = true;
    app.terminal.drag_anchor_y = Some(20.0);
    app.task_board.task_board_drag_pending =
        Some(("pending".to_owned(), crate::app::task::TaskStatus::Pending, iced::Point::ORIGIN));
    app.task_board.task_board_dragging = Some(("dragging".to_owned(), crate::app::task::TaskStatus::Running));

    let _task = super::update(&mut app, super::ViewMessage::GlobalMouseReleased);

    assert!(app.task_board.task_board_drag_pending.is_none());
    assert!(app.task_board.task_board_dragging.is_none());
    assert!(!app.layout_drag.dragging_settings);
    assert_eq!(app.layout_drag.settings_drag_anchor_x, None);
    assert!(!app.layout_drag.dragging_split);
    assert!(!app.layout_drag.dragging_work_sidebar);
    assert_eq!(app.layout_drag.work_sidebar_drag_anchor_x, None);
    assert!(!app.navigation_workspace.dragging_work_split);
    assert_eq!(app.navigation_workspace.work_split_drag_anchor_x, None);
    assert!(!app.terminal.is_dragging);
    assert_eq!(app.terminal.drag_anchor_y, None);
}

#[test]
fn global_release_transfers_internal_files_to_hovered_input() {
    let mut app = test_app();
    app.file_tree_interaction.dragging_file_paths = vec!["src/main.rs".to_owned()];
    app.file_tree_interaction.dragging_file_position = Some((4, 5));
    app.file_tree_interaction.input_drop_hovered = true;
    app.focus_area = crate::app::FocusArea::Preview;

    let _task = super::update(&mut app, super::ViewMessage::GlobalMouseReleased);

    assert!(app.file_tree_interaction.dragging_file_paths.is_empty());
    assert_eq!(app.file_tree_interaction.dragging_file_position, None);
    assert_eq!(app.file_tree_interaction.pending_drop_file_paths, vec!["src/main.rs"]);
    assert_eq!(app.file_tree_interaction.pending_drop_file_position, Some((4, 5)));
    assert!(!app.file_tree_interaction.input_drop_hovered);
    assert_eq!(app.focus_area, crate::app::FocusArea::None);
}

#[cfg(not(target_arch = "wasm32"))]
// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn global_cursor_left_makes_visible_lsp_hover_noninteractive_and_schedules_hide() {
    let mut app = test_app();
    app.preview_state.lsp_overlay.hover_visible = true;
    app.preview_state.lsp_overlay.hover_interactive = true;
    app.preview_state.lsp_hover_hide_deadline = None;

    let _task = super::update(&mut app, super::ViewMessage::GlobalCursorLeft);

    assert!(!app.preview_state.lsp_overlay.hover_interactive);
    assert!(app.preview_state.lsp_hover_hide_deadline.is_some());
}

#[test]
fn file_tree_move_drop_global_release_submits_once_without_chat_delivery() {
    use crate::app::message::{ProjectMessage as P, ViewMessage};
    for global_first in [true, false] {
        let mut app = test_app();
        app.navigation_tabs.project_path = Some("/project".into());
        let _ = crate::app::message::project::update(
            &mut app,
            P::FileTreeInternalDragStart {
                project_path: "/project".into(),
                source_path: "/project/a.txt".into(),
            },
        );
        let _ = crate::app::message::project::update(
            &mut app,
            P::FileTreeDropTargetChanged(Some("/project/dst".into())),
        );
        app.file_tree_interaction.input_drop_hovered = true;
        if global_first {
            let _ = super::update(&mut app, ViewMessage::GlobalMouseReleased);
            let _ = crate::app::message::project::update(&mut app, P::FileTreeDragEnd);
        } else {
            let _ = crate::app::message::project::update(&mut app, P::FileTreeDragEnd);
            let _ = super::update(&mut app, ViewMessage::GlobalMouseReleased);
        }
        let _ = super::update(&mut app, ViewMessage::GlobalMouseReleased);
        assert!(app.file_tree_interaction.file_tree_operation_in_progress);
        assert!(app.file_tree_interaction.file_tree_drag_source.is_none());
        assert!(app.file_tree_interaction.pending_drop_file_paths.is_empty());
        assert!(!app.file_tree_interaction.input_drop_hovered);
    }
}

#[test]
fn file_tree_move_drop_cursor_leave_cancels_without_pending_chat_files() {
    use crate::app::message::{ProjectMessage as P, ViewMessage};
    let mut app = test_app();
    app.navigation_tabs.project_path = Some("/project".into());
    let _ = crate::app::message::project::update(
        &mut app,
        P::FileTreeInternalDragStart {
            project_path: "/project".into(),
            source_path: "/project/a.txt".into(),
        },
    );
    let _ = super::update(&mut app, ViewMessage::GlobalCursorLeft);
    let _ = super::update(&mut app, ViewMessage::GlobalMouseReleased);
    assert!(!app.file_tree_interaction.file_tree_operation_in_progress);
    assert!(app.file_tree_interaction.file_tree_drag_source.is_none());
    assert!(app.file_tree_interaction.pending_drop_file_paths.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn file_tree_move_drop_chat_release_orders_preserve_existing_delivery() {
    use crate::app::message::{ProjectMessage as P, ViewMessage};
    use iced::futures::StreamExt;
    for global_first in [true, false] {
        let mut app = test_app();
        app.navigation_tabs.project_path = Some("/project".into());
        let _ = crate::app::message::project::update(
            &mut app,
            P::FileTreeInternalDragStart {
                project_path: "/project".into(),
                source_path: "/project/a.txt".into(),
            },
        );
        app.file_tree_interaction.input_drop_hovered = true;
        if !global_first {
            let _ = crate::app::message::project::update(&mut app, P::FileTreeDragEnd);
        }
        let task = super::update(&mut app, ViewMessage::GlobalMouseReleased);
        if global_first {
            let _ = crate::app::message::project::update(&mut app, P::FileTreeDragEnd);
        }
        let mut stream = iced_runtime::task::into_stream(task).expect("chat delivery");
        let action = stream.next().await;
        assert!(matches!(
            action,
            Some(iced_runtime::Action::Output(crate::app::Message::Chat(
                crate::app::message::chat::ChatMessage::InputAreaDragDrop
            )))
        ));
        assert_eq!(app.file_tree_interaction.pending_drop_file_paths, ["/project/a.txt"]);
        assert!(!app.file_tree_interaction.file_tree_operation_in_progress);
    }
}

#[test]
fn file_tree_external_drop_hover_left_and_release_never_import_or_leave_chat_paths() {
    use crate::app::message::ViewMessage as V;
    let mut app = test_app();
    app.navigation_tabs.project_path = Some("/project".into());
    app.file_tree_interaction.dragging_file_paths = vec!["/stale-internal-drag".into()];
    for message in [
        V::HoveredFilePath("/tmp/hover-only".into()),
        V::HoveredFilesLeft,
        V::GlobalMouseReleased,
        V::DroppedFilePath("/tmp/actual".into()),
    ] {
        let _ = super::update(&mut app, message);
    }
    assert!(app.file_tree_interaction.pending_drop_file_paths.is_empty());
    assert!(app.file_tree_interaction.dragging_file_paths.is_empty());
    assert!(!app.file_tree_interaction.file_tree_operation_in_progress);
}

#[test]
fn terminal_resize_drag_continues_outside_handle_and_release_stops_updates() {
    use crate::app::message::{TerminalMessage, ViewMessage};
    let mut app = test_app();
    app.window_size = (800.0, 500.0);
    app.terminal.height = 240.0;
    app.cursor_position = iced::Point::new(100.0, 260.0);
    let _ = crate::app::message::terminal::update(&mut app, TerminalMessage::DragStarted);
    assert!(app.terminal.is_dragging);
    for (y, expected) in [(200.0, 300.0), (-100.0, 400.0), (1000.0, 120.0)] {
        let _ = super::update(&mut app, ViewMessage::PointerMoved(100.0, y));
        assert_eq!(app.terminal.height, expected);
    }
    let _ = super::update(&mut app, ViewMessage::GlobalMouseReleased);
    assert!(!app.terminal.is_dragging);
    assert_eq!(app.terminal.drag_anchor_y, None);
    let _ = super::update(&mut app, ViewMessage::PointerMoved(100.0, -200.0));
    assert_eq!(app.terminal.height, 120.0);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn shutdown_waits_for_pending_layout_preferences() {
    use super::ViewMessage;
    use crate::app::{Message, config, message::view::panel};
    use iced_runtime::Action;
    use iced_runtime::futures::futures::{StreamExt, future::ready, poll};
    use serde_json::json;
    use std::time::Duration;

    const TIMEOUT: Duration = Duration::from_secs(4);

    fn outputs(
        task: iced::Task<Message>,
    ) -> impl iced_runtime::futures::futures::Stream<Item = Message> {
        iced_runtime::task::into_stream(task).expect("expected a task").filter_map(|action| {
            ready(match action {
                Action::Output(message) => Some(message),
                _ => None,
            })
        })
    }

    config::with_editor_settings_gateway(|mut app, requests, responses| {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let run_save = |task| {
            let mut stream = outputs(task);
            runtime.spawn(async move { stream.next().await.unwrap() })
        };
        let finish_save = |handle: tokio::task::JoinHandle<Message>| {
            runtime
                .block_on(async { tokio::time::timeout(TIMEOUT, handle).await.unwrap().unwrap() })
        };
        let capture_patch = |expected| {
            let request = requests.recv_timeout(TIMEOUT).expect("preference PATCH");
            assert!(request.starts_with("PATCH /v1/desktop/preferences "));
            let fields: serde_json::Value =
                serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
            assert_eq!(fields, expected);
        };

        let _ = super::update(&mut app, ViewMessage::FileManagerDragStarted);
        app.layout_drag.file_manager_width = 320.0;
        let first = run_save(super::update(&mut app, ViewMessage::GlobalMouseReleased));
        capture_patch(json!({"file_manager_width": 320.0}));

        let queued = panel::update(&mut app, ViewMessage::FileManagerPanelVisible(false));
        assert!(iced_runtime::task::into_stream(queued).is_none());
        let queued = panel::update(&mut app, ViewMessage::FileManagerPanelVisible(true));
        assert!(iced_runtime::task::into_stream(queued).is_none());
        let _ = super::update(&mut app, ViewMessage::FileManagerDragStarted);
        app.layout_drag.file_manager_width = 340.0;
        let queued = super::update(&mut app, ViewMessage::GlobalMouseReleased);
        assert!(iced_runtime::task::into_stream(queued).is_none());

        let window = iced::window::Id::unique();
        app.main_window_id = Some(window);
        app.lifecycle.close_to_tray = false;
        let mut shutdown = outputs(panel::update(&mut app, ViewMessage::CloseRequested(window)));
        assert!(runtime.block_on(async { poll!(shutdown.next()).is_pending() }));
        responses.send(Some("{}".into())).unwrap();

        let next = app.update(finish_save(first));
        let second = run_save(next);
        capture_patch(json!({"file_manager_width": 340.0, "show_file_manager": true}));
        assert!(runtime.block_on(async { poll!(shutdown.next()).is_pending() }));
        responses.send(Some("{}".into())).unwrap();

        let done = app.update(finish_save(second));
        assert!(iced_runtime::task::into_stream(done).is_none());
        let message =
            runtime.block_on(async { tokio::time::timeout(TIMEOUT, shutdown.next()).await });
        assert!(matches!(message, Ok(Some(Message::View(ViewMessage::ShutdownFinished)))));
        assert!(requests.try_recv().is_err());
    });
}
