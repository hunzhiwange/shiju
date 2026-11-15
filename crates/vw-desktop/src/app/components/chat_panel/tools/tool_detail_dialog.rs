//! 会话状态借用投影与详情动作映射。
use crate::app::{App, Message, message::ChatMessage};
use desktop_chat_tools::tools::{detail_input::{DetailInput, DialogInput, Message as Local}, tool_detail_dialog as domain};
fn input(app: &App) -> DetailInput<'_> {
 DetailInput { i18n: crate::app::i18n::I18n::new(app.desktop_locale), line_height: app.current_line_height,
 dialog: app.tool_detail_dialog.as_ref().map(|d| DialogInput {title: &d.title, editor: &d.editor, editor_id: &d.editor_id, scroll_top_line: d.scroll_top_line, context_menu_open: d.context_menu_open, context_menu_pos: d.context_menu_pos}) }
}
fn map(message: Local) -> Message {
 Message::Chat(match message {
 Local::CloseToolDetail => ChatMessage::CloseToolDetail,
 Local::ToolDetailEditorAction(action) => ChatMessage::ToolDetailEditorAction(action),
 Local::ToolDetailEditorWheelScrolled {delta, viewport_height} => ChatMessage::ToolDetailEditorWheelScrolled {delta, viewport_height},
 Local::ToolDetailScrollbarChanged {top_line, viewport_height} => ChatMessage::ToolDetailScrollbarChanged {top_line, viewport_height},
 Local::ToolDetailOpenContextMenu {x,y} => ChatMessage::ToolDetailOpenContextMenu {x,y},
 Local::ToolDetailUndo => ChatMessage::ToolDetailUndo,
 Local::ToolDetailRedo => ChatMessage::ToolDetailRedo,
 Local::ToolDetailCloseContextMenu => ChatMessage::ToolDetailCloseContextMenu,
 Local::ToolDetailContextMenuCopy => ChatMessage::ToolDetailContextMenuCopy,
 Local::ToolDetailContextMenuCut => ChatMessage::ToolDetailContextMenuCut,
 Local::ToolDetailContextMenuPaste => ChatMessage::ToolDetailContextMenuPaste,
 Local::ToolDetailContextMenuDelete => ChatMessage::ToolDetailContextMenuDelete,
 })
}
pub fn tool_detail_dialog_view(app: &App) -> Option<iced::Element<'_, Message>> {
 domain::tool_detail_dialog_view(input(app)).map(|v| v.map(map))
}
