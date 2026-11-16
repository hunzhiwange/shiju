//! 处理应用视图级别的主题或布局消息。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::ViewMessage;
use crate::app::{App, FocusArea, Message};
use iced::Task;

pub(crate) use desktop_navigation::window_lifecycle::{WindowClosedOutcome, mark_window_closed};

/// 执行 update 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub fn update(app: &mut App, message: ViewMessage) -> Task<Message> {
    if let Some(local) = mobile_message(&message) { return update_mobile(app, local); }
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
                    for child in app.web_navigation.independent_webview_children.iter_mut() {
                        let child: &mut std::process::Child = child;
                        let _ = child.kill();
                    }
                    app.web_navigation.independent_webview_children.clear();
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
        message @ (ViewMessage::SplitDragStarted | ViewMessage::WorkSidebarDragStarted | ViewMessage::WorkSplitDragStarted | ViewMessage::SettingsDragStarted | ViewMessage::FileManagerDragStarted | ViewMessage::PointerMoved(_, _)) => {
            use desktop_navigation::view_layout::drag as domain;
            let local = match message {
                ViewMessage::SplitDragStarted => domain::Message::SplitDragStarted,
                ViewMessage::WorkSidebarDragStarted => domain::Message::WorkSidebarDragStarted,
                ViewMessage::WorkSplitDragStarted => domain::Message::WorkSplitDragStarted,
                ViewMessage::SettingsDragStarted => domain::Message::SettingsDragStarted,
                ViewMessage::FileManagerDragStarted => domain::Message::FileManagerDragStarted,
                ViewMessage::PointerMoved(x, y) => { app.cursor_position = iced::Point::new(x, y); domain::Message::PointerMoved(x, y) },
                _ => unreachable!(),
            };
            if !domain::update(&mut app.layout_drag, &mut app.navigation_workspace, app.navigation_sidebar.work_sidebar_collapsed, app.window_size.0, local) { return Task::none(); }
            let domain::Message::PointerMoved(x, y) = local else { return Task::none(); };
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
            app.mobile_layout.update_size(w, h);
            let blocked = app.fullscreen_layout_settling || app.is_chat_fullscreen() || app.is_chat_half_fullscreen() || app.git.git_diff_fullscreen || app.git.git_diff_half_fullscreen;
            if let Some(size) = desktop_navigation::window_lifecycle::resize_main_window(app.main_window_id, window_id, &mut app.window_size, &mut app.startup_resize_checked, blocked, w, h) {
                return iced::window::resize::<Message>(window_id, size);
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
            app.file_tree_interaction.discard_pending_drop();
            Task::none()
        }
        ViewMessage::HoveredFilesLeft => {
            app.file_tree_interaction.leave_external_files();
            Task::none()
        }
        ViewMessage::FullscreenLayoutSettled => {
            desktop_navigation::window_lifecycle::settle_fullscreen_layout(&mut app.fullscreen_layout_settling);
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
            desktop_task_board::drag::clear_pending_drag(&mut app.task_board_drag_pending, &mut app.task_board_dragging);
            let save = desktop_navigation::view_layout::drag::finish(&mut app.layout_drag, &mut app.navigation_workspace)
                .map(|width| app.save_desktop_preference("file_manager_width", serde_json::json!(width)))
                .unwrap_or_else(Task::none);
            if app.task_pet_dragging {
                app.finish_task_pet_drag();
            }
            app.terminal.finish_pointer_resize();
            if app.git.git_diff_dragging {
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

fn mobile_message(message: &ViewMessage) -> Option<desktop_navigation::mobile_layout::Message> {
 use desktop_navigation::mobile_layout::Message as Local;
 Some(match message {
 ViewMessage::LayoutSelected(preference) => Local::Selected(*preference),
 ViewMessage::MobileDrawerToggled => Local::DrawerToggled,
 ViewMessage::MobileBack => Local::Back,
 ViewMessage::MobileChat => Local::Chat,
 ViewMessage::MobileSettings => Local::Settings,
 ViewMessage::MobileSettingsTab(tab) => Local::SettingsTab(*tab),
 ViewMessage::MobileProjectSelected(path) => Local::ProjectSelected(path.clone()),
 ViewMessage::MobileSessionSelected(path, id) => Local::SessionSelected(path.clone(), id.clone()),
 ViewMessage::MobileNewSession(path) => Local::NewSession(path.clone()),
 _ => return None,
 })
}
fn update_mobile(app: &mut App, message: desktop_navigation::mobile_layout::Message) -> Task<Message> {
 use desktop_navigation::mobile_layout as domain;
 let blocked = app.question_modal_request.is_some() || app.permission_modal_request.is_some() || app.new_session_picker_project.is_some();
 let effect = domain::update(domain::Context { mobile: &mut app.mobile_layout, panels: &mut app.navigation_panels, tabs: &mut app.navigation_tabs, notifications_expanded: &mut app.notifications_expanded }, blocked, message);
 match effect {
 None => Task::none(),
 Some(domain::Effect::OpenSettingsTab(tab)) => super::update(app, ViewMessage::OpenSystemSettingsTab(tab)),
 Some(domain::Effect::OpenProject(path)) => Task::batch([
 Task::done(Message::Project(crate::app::message::ProjectMessage::OpenRecentPressed(path.clone()))),
 Task::done(Message::Project(crate::app::message::ProjectMessage::ProjectLoadSessions(path))),
 ]),
 Some(domain::Effect::OpenSession(path, id)) => Task::done(Message::Project(crate::app::message::ProjectMessage::OpenProjectSessionPressed(path, id))),
 Some(domain::Effect::CreateSession(path)) => Task::done(Message::Project(crate::app::message::ProjectMessage::ProjectCreateSession(path))),
 }
}
