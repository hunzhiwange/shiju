use crate::app::{App, Message};
pub(super) use desktop_chat_view::message_view::markdown_selection::{Selection, FragmentId, FragmentStyle, fragment};
use desktop_chat_view::message_view::markdown_selection as selection;
impl selection::SelectionMessage for Message {
    fn none() -> Self { Self::None }
    fn copy(text: String) -> Self { Self::CopyCode(text) }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        crate::app::components::text_editor_context_menu::text_context_message(effect)
    }
    fn selection(session_id: String, message_idx: usize, text_idx: usize, text: Option<String>) -> Self {
        Self::ChatMarkdownSelectionChanged { session_id, message_idx, text_idx, text }
    }
}
pub(super) fn area<'a>(content: iced::Element<'a, Message>, selection: Selection, app: &'a App, message_idx: usize, text_idx: usize) -> iced::Element<'a, Message> {
    let selected = app.chat_markdown_selection.as_ref().is_some_and(|s| s.message_idx == message_idx && s.text_idx == text_idx && app.active_session_id.as_deref() == Some(s.session_id.as_str()));
    selection::area(content, selection, app.active_session_id.as_deref(), selected, message_idx, text_idx)
}
pub(in crate::app::components) use self::selection::selectable_titled_text;
#[cfg(test)]
use iced::{Element, Font, Point, Rectangle, Renderer, Size, mouse};
#[cfg(test)]
use iced::advanced::{Layout, Shell, layout, widget::Tree};
#[cfg(test)]
#[path = "markdown_selection_tests.rs"]
mod tests;
