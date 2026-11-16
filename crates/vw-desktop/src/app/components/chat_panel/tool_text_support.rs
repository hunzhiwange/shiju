use crate::app::{App, Message, message};
use iced::{Element, Theme, Color};
use desktop_chat_tools::tool_text_support as domain;
pub use domain::{ToolTextTarget,tool_text_key,message_target_from_context,chat_text_line_height,chat_text_font_name,chat_text_font,content_safe_font,chat_text_selection_color,read_only_text_style,tool_text_style,tool_text_style_with_danger,READ_ONLY_EDITOR_MAX_HEIGHT,is_safe_for_text_editor};
pub(crate) fn text_state(app: &App) -> domain::ToolTextState<'_> {
    domain::ToolTextState {
        chat_message_editors: &app.chat_view.chat_message_editors,
        chat_special_text_editors: &app.chat_view.chat_special_text_editors,
        chat_tool_text_editors: &app.chat_view.chat_tool_text_editors,
        chat_think_editors: &app.chat_view.chat_think_editors,
        chat_markdown_block_editors: &app.chat_view.chat_markdown_block_editors,
        markdown_selection: app.chat_interaction.chat_markdown_selection.as_ref()
            .filter(|selection| app.session_state.active_session_id.as_deref() == Some(selection.session_id.as_str()))
            .map(|selection| (selection.message_idx, selection.text.as_str())),
        theme: app.theme(),
    }
}
pub(crate) fn map_message(message: domain::Message) -> Message {
    Message::Chat(match message {
        domain::Message::SpecialTextEditorAction(m, t, a) => message::ChatMessage::SpecialTextEditorAction(m, t, a),
        domain::Message::ToolTextEditorAction(m, t, x, a) => message::ChatMessage::ToolTextEditorAction(m, t, x, a),
    })
}
pub fn selected_chat_text_for_message(app: &App, msg_idx: usize) -> Option<String> { domain::selected_chat_text_for_message(&text_state(app), msg_idx) }
pub fn selected_chat_text_for_tool(app: &App, msg_idx: usize, tool_idx: usize) -> Option<String> { domain::selected_chat_text_for_tool(&text_state(app), msg_idx, tool_idx) }
pub fn selected_chat_text_for_target(app: &App, target: u64) -> Option<String> { domain::selected_chat_text_for_target(&text_state(app), target) }
pub fn tool_text_editor<'a>(app: &'a App, target: ToolTextTarget, font_name: &'static str, size: f32, enable_highlight: bool, danger: bool) -> Option<Element<'a, Message>> {
    domain::tool_text_editor(&text_state(app), target, font_name, size, enable_highlight, danger).map(|element| element.map(map_message))
}
pub fn tool_inline_text_editor<'a, F>(app: &'a App, target: ToolTextTarget, font_name: &'static str, size: f32, color_fn: F) -> Option<Element<'a, Message>> where F: Fn(&Theme) -> Color + Copy + 'a {
    domain::tool_inline_text_editor(&text_state(app), target, font_name, size, color_fn).map(|element| element.map(map_message))
}
