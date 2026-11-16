//! 处理应用视图级别的主题或布局消息。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::ViewMessage;
use crate::app::{App, FocusArea, Message};
use iced::Task;

use desktop_navigation::view_layout::{
    MIN_WORK_SIDEBAR_WIDTH, MAX_WORK_SIDEBAR_WIDTH, MIN_WORK_SPLIT_RATIO, MAX_WORK_SPLIT_RATIO,
};

pub(crate) use desktop_navigation::window_lifecycle::{WindowClosedOutcome, mark_window_closed};

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
            if let Some(index) = app.released_main_windows.iter().position(|id| *id == window_id) {
                app.released_main_windows.swap_remove(index);
                return Task::none();
            }
            if app.main_window_id == Some(window_id) {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    app.work_browser_controller.shutdown();
                }
            }
            if mark_window_closed(&mut app.main_window_id, &mut app.task_pet_window_id, window_id)
                == WindowClosedOutcome::Exit
                && !(app.close_to_tray && app.main_window_id.is_none() && !app.shutdown_requested)
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
        ViewMessage::WindowFocused(window_id) => {
            if app.main_window_id == Some(window_id) {
                app.work_browser.window_focused = true;
                return crate::app::message::settings::voice::update(
                    app,
                    crate::app::message::settings::voice::Event::RefreshLocal,
                );
            }
            Task::none()
        }
        ViewMessage::WindowUnfocused(window_id) => {
            if app.main_window_id == Some(window_id) {
                app.work_browser.window_focused = false;
            }
            Task::none()
        }
        ViewMessage::SplitDragStarted => {
            app.dragging_split = true;
            app.split_drag_anchor_x = None;
            app.split_drag_start_ratio = app.split_ratio;
            Task::none()
        }
        ViewMessage::WorkSidebarDragStarted => {
            app.dragging_work_sidebar = true;
            app.work_sidebar_drag_anchor_x = None;
            app.work_sidebar_drag_start_width = app.work_sidebar_width;
            Task::none()
        }
        ViewMessage::WorkSplitDragStarted => {
            app.dragging_work_split = true;
            app.work_split_drag_anchor_x = None;
            app.work_split_drag_start_ratio = app.work_split_ratio;
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
                let Some(value) = desktop_navigation::view_layout::drag_value(
                    &mut app.settings_drag_anchor_x, x, app.settings_drag_start_width, 1.0, 150.0, 800.0,
                ) else { return Task::none(); };
                app.settings_panel_width = value;
            }
            if app.dragging_file_manager {
                let Some(value) = desktop_navigation::view_layout::drag_value(
                    &mut app.file_manager_drag_anchor_x, x, app.file_manager_start_width, -1.0, 180.0, 600.0,
                ) else { return Task::none(); };
                app.file_manager_width = value;
            }
            if app.dragging_split {
                let Some(value) = desktop_navigation::view_layout::drag_value(
                    &mut app.split_drag_anchor_x, x, app.split_drag_start_ratio, app.window_size.0.max(1.0), 0.2, 0.8,
                ) else { return Task::none(); };
                app.split_ratio = value;
            }
            if app.dragging_work_sidebar {
                let Some(value) = desktop_navigation::view_layout::drag_value(
                    &mut app.work_sidebar_drag_anchor_x, x, app.work_sidebar_drag_start_width, 1.0, MIN_WORK_SIDEBAR_WIDTH, MAX_WORK_SIDEBAR_WIDTH,
                ) else { return Task::none(); };
                app.work_sidebar_width = value;
            }
            if app.dragging_work_split {
                let rail_width = if app.work_sidebar_collapsed { crate::app::state::WORK_SIDEBAR_RAIL_WIDTH } else { 0.0 };
                let available_width = (app.window_size.0 - app.work_sidebar_width - rail_width).max(1.0);
                let Some(value) = desktop_navigation::view_layout::drag_value(
                    &mut app.work_split_drag_anchor_x, x, app.work_split_drag_start_ratio, available_width, MIN_WORK_SPLIT_RATIO, MAX_WORK_SPLIT_RATIO,
                ) else { return Task::none(); };
                app.work_split_ratio = value;
            }
            if app.task_pet_dragging {
                app.drag_task_pet_to(x, y);
            }
            if app.terminal.is_dragging && !app.terminal.pointer_resize(y, app.window_size.1) {
                return Task::none();
            }
            desktop_task_board::drag::promote_pending_drag(
                &mut app.task_board_drag_pending, &mut app.task_board_dragging, app.cursor_position,
            );
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
                || app.is_chat_fullscreen()
                || app.is_chat_half_fullscreen()
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
            app.file_tree_interaction.hover_external_file(path);
            Task::none()
        }
        ViewMessage::DroppedFilePath(path) => {
            app.file_tree_interaction.drop_external_file(path);
            if app.file_tree_interaction.input_drop_hovered {
                return crate::app::message::chat::update(
                    app,
                    crate::app::message::ChatMessage::InputAreaDragDrop,
                );
            }
            if app.knowledge.file_drop_hovered {
                return crate::app::message::knowledge_tool::update(
                    app,
                    crate::app::message::knowledge_tool::KnowledgeToolMessage::ImportDroppedFiles,
                );
            }
            app.file_tree_interaction.pending_drop_file_paths.clear();
            Task::none()
        }
        ViewMessage::HoveredFilesLeft => {
            app.file_tree_interaction.leave_external_files();
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
            let save = if app.dragging_file_manager {
                app.dragging_file_manager = false;
                app.file_manager_drag_anchor_x = None;
                app.save_desktop_preference(
                    "file_manager_width",
                    serde_json::json!(app.file_manager_width),
                )
            } else {
                Task::none()
            };
            if app.dragging_split {
                app.dragging_split = false;
            }
            if app.dragging_work_sidebar {
                app.dragging_work_sidebar = false;
                app.work_sidebar_drag_anchor_x = None;
            }
            if app.dragging_work_split {
                app.dragging_work_split = false;
                app.work_split_drag_anchor_x = None;
            }
            if app.task_pet_dragging {
                app.finish_task_pet_drag();
            }
            if app.terminal.is_dragging {
                app.terminal.is_dragging = false;
                app.terminal.drag_anchor_y = None;
            }
            if app.git_diff_dragging {
                return Task::batch([
                    save,
                    Task::done(Message::Git(
                        crate::app::message::git::GitMessage::DiffDragSelectEnd,
                    )),
                ]);
            }
            if app.file_tree_interaction.file_tree_drag_source.is_some() && app.file_tree_interaction.file_tree_drop_target.is_some() {
                return Task::batch([
                    save,
                    crate::app::message::project::file_tree::release_tree_drop(app),
                ]);
            }
            if app.file_tree_interaction.release_chat_drop() {
                app.focus_area = FocusArea::None;
                return Task::batch([
                    save,
                    Task::done(Message::Chat(
                        crate::app::message::chat::ChatMessage::InputAreaDragDrop,
                    )),
                ]);
            }

            save
        }
        ViewMessage::GlobalCursorLeft => {
            if app.file_tree_interaction.file_tree_drag_source.is_some() {
                let _ = crate::app::message::project::update(
                    app,
                    crate::app::message::ProjectMessage::FileTreeDragCancel,
                );
            }
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
