use crate::app::Message;
use desktop_chat_view::message_view::markdown_selection as selection;
impl selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        crate::app::components::text_editor_context_menu::text_context_message(effect)
    }
    fn selection(
        session_id: String,
        message_idx: usize,
        text_idx: usize,
        text: Option<String>,
    ) -> Self {
        Self::ChatMarkdownSelectionChanged { session_id, message_idx, text_idx, text }
    }
}
pub(in crate::app::components) use self::selection::selectable_titled_text;
