use crate::app::components::chat_panel::tools::{
    is_mcp_tool_raw,
    workflow_node_detail_body,
};
use crate::app::{App, Message};
use super::ChatMessage;
use iced::{Task, mouse, widget::text_editor};
use desktop_session::message::chat::input::tool_detail as domain;
pub(super) fn handle_open_tool_detail(
    app: &mut App,
    msg_idx: usize,
    tool_idx: usize,
    raw: String,
) -> Task<Message> {
 let mut state = domain::ToolDetailState { tool_detail_dialog: app.tool_detail_dialog.take(), current_line_height: app.current_line_height };
 let task = domain::handle_open_tool_detail(&mut state, msg_idx, tool_idx, tool_detail_from_raw(crate::app::i18n::I18n::new(app.desktop_locale), &raw));
 app.tool_detail_dialog = state.tool_detail_dialog;
 task.map(|message| match message { domain::ToolDetailMessage::EditorAction(action) => Message::Chat(ChatMessage::ToolDetailEditorAction(action)) })
}
pub(super) fn handle_tool_detail_editor_action(
    app: &mut App,
    action: text_editor::Action,
) -> Task<Message> {
 let mut state = domain::ToolDetailState { tool_detail_dialog: app.tool_detail_dialog.take(), current_line_height: app.current_line_height };
 let task = domain::handle_tool_detail_editor_action(&mut state , action);
 app.tool_detail_dialog = state.tool_detail_dialog;
 task.map(|message| match message { domain::ToolDetailMessage::EditorAction(action) => Message::Chat(ChatMessage::ToolDetailEditorAction(action)) })
}
pub(super) fn handle_tool_detail_undo(app: &mut App) -> Task<Message> {
 let mut state = domain::ToolDetailState { tool_detail_dialog: app.tool_detail_dialog.take(), current_line_height: app.current_line_height };
 let task = domain::handle_tool_detail_undo(&mut state );
 app.tool_detail_dialog = state.tool_detail_dialog;
 task.map(|message| match message { domain::ToolDetailMessage::EditorAction(action) => Message::Chat(ChatMessage::ToolDetailEditorAction(action)) })
}
pub(super) fn handle_tool_detail_redo(app: &mut App) -> Task<Message> {
 let mut state = domain::ToolDetailState { tool_detail_dialog: app.tool_detail_dialog.take(), current_line_height: app.current_line_height };
 let task = domain::handle_tool_detail_redo(&mut state );
 app.tool_detail_dialog = state.tool_detail_dialog;
 task.map(|message| match message { domain::ToolDetailMessage::EditorAction(action) => Message::Chat(ChatMessage::ToolDetailEditorAction(action)) })
}
pub(super) fn handle_tool_detail_open_context_menu(app: &mut App, x: f32, y: f32) -> Task<Message> {
 let mut state = domain::ToolDetailState { tool_detail_dialog: app.tool_detail_dialog.take(), current_line_height: app.current_line_height };
 let task = domain::handle_tool_detail_open_context_menu(&mut state , x, y);
 app.tool_detail_dialog = state.tool_detail_dialog;
 task.map(|message| match message { domain::ToolDetailMessage::EditorAction(action) => Message::Chat(ChatMessage::ToolDetailEditorAction(action)) })
}
pub(super) fn handle_tool_detail_close_context_menu(app: &mut App) -> Task<Message> {
 let mut state = domain::ToolDetailState { tool_detail_dialog: app.tool_detail_dialog.take(), current_line_height: app.current_line_height };
 let task = domain::handle_tool_detail_close_context_menu(&mut state );
 app.tool_detail_dialog = state.tool_detail_dialog;
 task.map(|message| match message { domain::ToolDetailMessage::EditorAction(action) => Message::Chat(ChatMessage::ToolDetailEditorAction(action)) })
}
pub(super) fn handle_tool_detail_context_menu_copy(app: &mut App) -> Task<Message> {
 let mut state = domain::ToolDetailState { tool_detail_dialog: app.tool_detail_dialog.take(), current_line_height: app.current_line_height };
 let task = domain::handle_tool_detail_context_menu_copy(&mut state );
 app.tool_detail_dialog = state.tool_detail_dialog;
 task.map(|message| match message { domain::ToolDetailMessage::EditorAction(action) => Message::Chat(ChatMessage::ToolDetailEditorAction(action)) })
}
pub(super) fn handle_tool_detail_context_menu_cut(app: &mut App) -> Task<Message> {
 let mut state = domain::ToolDetailState { tool_detail_dialog: app.tool_detail_dialog.take(), current_line_height: app.current_line_height };
 let task = domain::handle_tool_detail_context_menu_cut(&mut state );
 app.tool_detail_dialog = state.tool_detail_dialog;
 task.map(|message| match message { domain::ToolDetailMessage::EditorAction(action) => Message::Chat(ChatMessage::ToolDetailEditorAction(action)) })
}
pub(super) fn handle_tool_detail_context_menu_paste(app: &mut App) -> Task<Message> {
 let mut state = domain::ToolDetailState { tool_detail_dialog: app.tool_detail_dialog.take(), current_line_height: app.current_line_height };
 let task = domain::handle_tool_detail_context_menu_paste(&mut state );
 app.tool_detail_dialog = state.tool_detail_dialog;
 task.map(|message| match message { domain::ToolDetailMessage::EditorAction(action) => Message::Chat(ChatMessage::ToolDetailEditorAction(action)) })
}
pub(super) fn handle_tool_detail_context_menu_delete(app: &mut App) -> Task<Message> {
 let mut state = domain::ToolDetailState { tool_detail_dialog: app.tool_detail_dialog.take(), current_line_height: app.current_line_height };
 let task = domain::handle_tool_detail_context_menu_delete(&mut state );
 app.tool_detail_dialog = state.tool_detail_dialog;
 task.map(|message| match message { domain::ToolDetailMessage::EditorAction(action) => Message::Chat(ChatMessage::ToolDetailEditorAction(action)) })
}
pub(super) fn handle_tool_detail_editor_wheel_scrolled(
    app: &mut App,
    delta: mouse::ScrollDelta,
    viewport_height: f32,
) -> Task<Message> {
 let mut state = domain::ToolDetailState { tool_detail_dialog: app.tool_detail_dialog.take(), current_line_height: app.current_line_height };
 let task = domain::handle_tool_detail_editor_wheel_scrolled(&mut state , delta, viewport_height);
 app.tool_detail_dialog = state.tool_detail_dialog;
 task.map(|message| match message { domain::ToolDetailMessage::EditorAction(action) => Message::Chat(ChatMessage::ToolDetailEditorAction(action)) })
}
pub(super) fn handle_tool_detail_scrollbar_changed(
    app: &mut App,
    top_line: f32,
    viewport_height: f32,
) -> Task<Message> {
 let mut state = domain::ToolDetailState { tool_detail_dialog: app.tool_detail_dialog.take(), current_line_height: app.current_line_height };
 let task = domain::handle_tool_detail_scrollbar_changed(&mut state , top_line, viewport_height);
 app.tool_detail_dialog = state.tool_detail_dialog;
 task.map(|message| match message { domain::ToolDetailMessage::EditorAction(action) => Message::Chat(ChatMessage::ToolDetailEditorAction(action)) })
}
pub(super) fn handle_close_tool_detail(app: &mut App) -> Task<Message> {
 let mut state = domain::ToolDetailState { tool_detail_dialog: app.tool_detail_dialog.take(), current_line_height: app.current_line_height };
 let task = domain::handle_close_tool_detail(&mut state );
 app.tool_detail_dialog = state.tool_detail_dialog;
 task.map(|message| match message { domain::ToolDetailMessage::EditorAction(action) => Message::Chat(ChatMessage::ToolDetailEditorAction(action)) })
}
fn tool_detail_from_raw(i18n: crate::app::i18n::I18n, raw: &str) -> Option<(String, String)> {
 domain::tool_detail_from_raw(i18n, raw, is_mcp_tool_raw(raw), workflow_node_detail_body)
}

#[cfg(test)]
#[path = "tool_detail_tests.rs"]
mod tool_detail_tests;
