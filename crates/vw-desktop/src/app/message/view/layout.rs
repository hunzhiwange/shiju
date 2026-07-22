//! 处理应用视图级别的主题或布局消息。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::ViewMessage;
use crate::app::{App, FocusArea, Message, set_config_field};
use iced::Task;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowClosedOutcome {
    Continue,
    Exit,
}

pub(crate) fn mark_window_closed(
    main_window_id: &mut Option<iced::window::Id>,
    task_pet_window_id: &mut Option<iced::window::Id>,
    closed_window_id: iced::window::Id,
) -> WindowClosedOutcome {
    if *main_window_id == Some(closed_window_id) {
        *main_window_id = None;
        *task_pet_window_id = None;
        return WindowClosedOutcome::Exit;
    }

    if *task_pet_window_id == Some(closed_window_id) {
        *task_pet_window_id = None;
    }

    if main_window_id.is_none() && task_pet_window_id.is_none() {
        WindowClosedOutcome::Exit
    } else {
        WindowClosedOutcome::Continue
    }
}

/// 执行 update 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    match message {
        ViewMessage::WindowMoved(window_id, x, y) => {
            if app.main_window_id == Some(window_id) {
                app.window_position = (x, y);
            } else if app.task_pet_window_id == Some(window_id) {
                app.move_task_pet_window_to(x, y);
            }
            Task::none()
        }
        ViewMessage::WindowClosed(window_id) => {
            if mark_window_closed(&mut app.main_window_id, &mut app.task_pet_window_id, window_id)
                == WindowClosedOutcome::Exit
            {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    for child in app.independent_webview_children.iter_mut() {
                        let child: &mut std::process::Child = child;
                        let _ = child.kill();
                    }
                    app.independent_webview_children.clear();
                }
                return iced::exit();
            }
            Task::none()
        }
        ViewMessage::SplitDragStarted => {
            app.dragging_split = true;
            app.split_drag_anchor_x = None;
            app.split_drag_start_ratio = app.split_ratio;
            Task::none()
        }
        ViewMessage::SettingsDragStarted => {
            app.dragging_settings = true;
            app.settings_drag_anchor_x = None;
            app.settings_drag_start_width = app.settings_panel_width;
            Task::none()
        }
        ViewMessage::FileManagerDragStarted => {
            app.dragging_file_manager = true;
            app.file_manager_drag_anchor_x = None;
            app.file_manager_start_width = app.file_manager_width;
            Task::none()
        }
        ViewMessage::PointerMoved(x, y) => {
            app.cursor_position = iced::Point::new(x, y);
            if app.dragging_settings {
                if app.settings_drag_anchor_x.is_none() {
                    app.settings_drag_anchor_x = Some(x);
                    return Task::none();
                }
                let anchor = app.settings_drag_anchor_x.unwrap_or(x);
                let delta = x - anchor;
                let new_width = (app.settings_drag_start_width + delta).clamp(150.0, 800.0);
                app.settings_panel_width = new_width;
            }
            if app.dragging_file_manager {
                if app.file_manager_drag_anchor_x.is_none() {
                    app.file_manager_drag_anchor_x = Some(x);
                    return Task::none();
                }
                let anchor = app.file_manager_drag_anchor_x.unwrap_or(x);
                let delta = anchor - x;
                let new_width = (app.file_manager_start_width + delta).clamp(180.0, 600.0);
                app.file_manager_width = new_width;
            }
            if app.dragging_split {
                if app.split_drag_anchor_x.is_none() {
                    app.split_drag_anchor_x = Some(x);
                    return Task::none();
                }
                let anchor = app.split_drag_anchor_x.unwrap_or(x);
                let w = app.window_size.0.max(1.0);
                let delta = x - anchor;
                let ratio = (app.split_drag_start_ratio + delta / w).clamp(0.2, 0.8);
                app.split_ratio = ratio;
            }
            if app.task_pet_dragging {
                app.drag_task_pet_to(x, y);
            }
            if app.terminal.is_dragging {
                if app.terminal.drag_anchor_y.is_none() {
                    app.terminal.drag_anchor_y = Some(y);
                    return Task::none();
                }
                let anchor = app.terminal.drag_anchor_y.unwrap_or(y);
                let delta = anchor - y;
                let mut th = (app.terminal.drag_start_height + delta).max(0.0);
                let h = app.window_size.1.max(1.0);
                th = th.clamp(120.0, h * 0.8);
                app.terminal.height = th;
            }
            if let Some((task_id, from_status, press_pos)) = &app.task_board_drag_pending {
                let dx = app.cursor_position.x - press_pos.x;
                let dy = app.cursor_position.y - press_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();
                if distance > 5.0 {
                    let task_id = task_id.as_str().to_owned();
                    let from_status = *from_status;
                    app.task_board_drag_pending = None;
                    app.task_board_dragging = Some((task_id, from_status));
                }
            }
            Task::none()
        }
        ViewMessage::WindowResized(window_id, w, h) => {
            if app.main_window_id != Some(window_id) {
                return Task::none();
            }
            let window_w = w.max(1.0);
            let window_h = h.max(1.0);
            app.window_size = (window_w, window_h);
            if app.fullscreen_layout_settling
                || app.chat_panel_fullscreen
                || app.chat_panel_half_fullscreen
                || app.git_diff_fullscreen
                || app.git_diff_half_fullscreen
            {
                return Task::none();
            }
            if app.startup_resize_checked {
                return Task::none();
            }
            app.startup_resize_checked = true;
            #[cfg(not(target_arch = "wasm32"))]
            if window_w > window_h * 2.0 {
                return iced::window::resize::<Message>(
                    window_id,
                    iced::Size::new(window_w * 0.5, window_h),
                );
            }
            Task::none()
        }
        ViewMessage::HoveredFilePath(path) => {
            if app.dragging_file_paths.is_empty()
                && !app.pending_drop_file_paths.iter().any(|existing| existing == &path)
            {
                app.pending_drop_file_paths.push(path);
                app.pending_drop_file_position = None;
            }
            Task::none()
        }
        ViewMessage::HoveredFilesLeft => {
            if app.dragging_file_paths.is_empty() {
                app.pending_drop_file_paths.clear();
                app.pending_drop_file_position = None;
                app.input_drop_hovered = false;
            }
            Task::none()
        }
        ViewMessage::FullscreenLayoutSettled => {
            app.fullscreen_layout_settling = false;
            Task::none()
        }
        ViewMessage::WindowDragPressed => {
            if let Some(window_id) = app.main_window_id {
                iced::window::drag(window_id)
            } else {
                Task::none()
            }
        }
        ViewMessage::GlobalMouseReleased => {
            app.task_board_drag_pending = None;
            app.task_board_dragging = None;
            if app.dragging_settings {
                app.dragging_settings = false;
                app.settings_drag_anchor_x = None;
            }
            if app.dragging_file_manager {
                app.dragging_file_manager = false;
                app.file_manager_drag_anchor_x = None;
                set_config_field(
                    "file_manager_width",
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(app.file_manager_width as f64).unwrap(),
                    ),
                );
            }
            if app.dragging_split {
                app.dragging_split = false;
            }
            if app.task_pet_dragging {
                app.finish_task_pet_drag();
            }
            if app.terminal.is_dragging {
                app.terminal.is_dragging = false;
                app.terminal.drag_anchor_y = None;
            }
            if app.git_diff_dragging {
                return Task::done(Message::Git(
                    crate::app::message::git::GitMessage::DiffDragSelectEnd,
                ));
            }
            if !app.dragging_file_paths.is_empty() {
                app.pending_drop_file_paths = app.dragging_file_paths.clone();
                app.pending_drop_file_position = app.dragging_file_position;
                app.dragging_file_paths.clear();
                app.dragging_file_position = None;
            }
            let input_drop_hovered = app.input_drop_hovered;
            app.input_drop_hovered = false;

            if input_drop_hovered && !app.pending_drop_file_paths.is_empty() {
                app.focus_area = FocusArea::None;
                return Task::done(Message::Chat(
                    crate::app::message::chat::ChatMessage::InputAreaDragDrop,
                ));
            }

            app.pending_drop_file_paths.clear();
            app.pending_drop_file_position = None;

            Task::none()
        }
        ViewMessage::GlobalCursorLeft => {
            #[cfg(not(target_arch = "wasm32"))]
            if app.lsp_overlay.hover_visible {
                app.lsp_overlay.hover_interactive = false;
                app.lsp_hover_hide_deadline =
                    Some(std::time::Instant::now() + std::time::Duration::from_millis(400));
            }
            Task::none()
        }
        _ => Task::none(),
    }
}
#[cfg(test)]
#[path = "layout_tests.rs"]
mod layout_tests;
