//! 预览编辑器的跨域命令与局部任务映射。
use super::PreviewMessage;
use crate::app::{App, FocusArea, Message};
use iced::Task;
use desktop_editor::preview::editor::{self as domain, Effect};
pub fn update(app: &mut App, message: PreviewMessage) -> Task<Message> {
 if let PreviewMessage::EditorEvent(ref evt) = message {
   if matches!(evt, iced_code_editor::Message::CustomContextMenuAction(_) | iced_code_editor::Message::RevealInFileManager)
       && app.editor_popover == Some(crate::app::message::editor::EditorPopover::Actions) {
       app.editor_popover = None;
   }
   if let iced_code_editor::Message::CustomContextMenuAction(id) = evt {
     return match id.as_str() {
       desktop_editor::components::editor::APP_CONTEXT_MENU_INSERT_SELECTION => Task::done(Message::Chat(crate::app::message::ChatMessage::InsertSelected)),
       desktop_editor::components::editor::APP_CONTEXT_MENU_INSERT_SELECTION_POSITIONS => Task::done(Message::Chat(crate::app::message::ChatMessage::InsertSelectionPositions)),
       unknown => { tracing::warn!(action_id = unknown, "忽略未知的编辑器自定义右键菜单动作"); Task::none() }
     };
   }
   if matches!(evt, iced_code_editor::Message::RevealInFileManager) {
     if app.active_preview_path.as_ref().is_some_and(|path| app.preview_tabs.iter().any(|tab| tab.path == *path && tab.read_only)) { return Task::none(); }
     return match app.active_preview_path.clone() {
       Some(path) => Task::done(Message::View(crate::app::message::ViewMessage::OpenPathInFinder(path))),
       None => app.show_localized_toast(crate::app::state::ToastKind::Error, "无法在文件管理器中显示：没有活动文件", vec![]),
     };
   }
 }
 if matches!(message, PreviewMessage::ContextMenuOpenForActiveEditor(..)) && app.active_preview_path.is_some() {
   super::dismiss_preview_popup_menus(app);
   app.git.git_diff_context_menu = None;
 }
 #[cfg(not(target_arch = "wasm32"))]
 if matches!(message, PreviewMessage::LspToggleDisabled(_)) { app.note_startup_setting_edit("lsp_enabled"); }
 let mut focus_preview = false;
 #[cfg(not(target_arch = "wasm32"))] let mut persist_lsp = None;
 #[cfg(not(target_arch = "wasm32"))] let mut notifications = Vec::new();
 let task = domain::update(
    &mut app.active_preview_path,
    &mut app.cursor_position,
    &mut app.editor_popover,
    #[cfg(not(target_arch = "wasm32"))]
    &mut app.lsp_applying_completion,
    #[cfg(not(target_arch = "wasm32"))]
    &mut app.lsp_disabled,
    #[cfg(not(target_arch = "wasm32"))]
    &mut app.lsp_hover_anchor,
    #[cfg(not(target_arch = "wasm32"))]
    &mut app.lsp_hover_hide_deadline,
    #[cfg(not(target_arch = "wasm32"))]
    &mut app.lsp_hover_pending,
    #[cfg(not(target_arch = "wasm32"))]
    &mut app.lsp_manager,
    #[cfg(not(target_arch = "wasm32"))]
    &mut app.lsp_overlay,
    #[cfg(not(target_arch = "wasm32"))]
    &mut app.lsp_overlay_path,
    #[cfg(not(target_arch = "wasm32"))]
    &mut app.lsp_progress,
    #[cfg(not(target_arch = "wasm32"))]
    &mut app.lsp_status,
    #[cfg(not(target_arch = "wasm32"))]
    &mut app.lsp_status_message,
    #[cfg(not(target_arch = "wasm32"))]
    &mut notifications,
    &mut app.preview_auto_save_mode,
    &mut app.preview_context_menu_pos,
    &mut app.preview_context_target,
    #[cfg(not(target_arch = "wasm32"))]
    &mut app.preview_nav_popup,
    &mut app.preview_tabs,
    #[cfg(not(target_arch = "wasm32"))]
    &mut app.project_path,
    &mut app.show_preview_context_menu,
    &mut app.show_preview_fullscreen_overlay,
    &mut focus_preview,
    #[cfg(not(target_arch = "wasm32"))] &mut persist_lsp,
    message,
 );
 if focus_preview { app.focus_area = FocusArea::Preview; }
 #[cfg(not(target_arch = "wasm32"))]
 for (source, arguments) in notifications { app.push_localized_notification(source, arguments); }
 let task = task.then(|effect| match effect {
   Effect::Preview(message) => Task::done(Message::Preview(message)),
   Effect::Editor(message) => Task::done(Message::Editor(message)),
   Effect::AutoSave { path, revision } => crate::app::message::after(std::time::Duration::from_millis(700), Message::Preview(PreviewMessage::AutoSaveDelayElapsed { path, revision })),
 });
 #[cfg(not(target_arch = "wasm32"))]
 if let Some(enabled) = persist_lsp {
   return Task::batch([task, crate::app::config::update_system_settings_config_async(move |system| { system.lsp_enabled = enabled; })]);
 }
 task
}
#[cfg(test)] #[path = "editor_tests.rs"] mod editor_tests;
