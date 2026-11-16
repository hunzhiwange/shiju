pub use desktop_chat_view::message_view::styles::*;

pub(super) fn thinking_status_text<'a>(label: &str, now_ms: u64, animation_frame: usize) -> iced::Element<'a, crate::app::Message> {
    desktop_chat_view::message_view::styles::thinking_status_text(label, now_ms, animation_frame).map(super::text::map_message)
}
