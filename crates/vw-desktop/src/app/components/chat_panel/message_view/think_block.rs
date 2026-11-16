use crate::app::{App, Message, message, models};
use iced::Element;
pub use desktop_chat_view::message_view::think_block::{think_block_is_running, think_block_default_expanded, think_block_resolved_expanded, should_render_think_block};
#[cfg(test)]
pub(super) fn think_block_view<'a>(
    app: &'a App,
    msg_idx: usize,
    think_idx: usize,
    content: String,
    open: bool,
    timing: Option<&'a models::ThinkTiming>,
) -> Element<'a, Message> {
    let is_streaming_msg =
        app.current_session_runtime_ref().is_some_and(|runtime| runtime.request_lifecycle.is_requesting)
            && msg_idx + 1 == app.chat.len();
    think_block_view_with_streaming(
        app,
        msg_idx,
        think_idx,
        content,
        open,
        timing,
        is_streaming_msg,
    )
}

pub(super) fn think_block_view_with_streaming<'a>(app: &'a App, msg_idx: usize, think_idx: usize, content: String, open: bool, timing: Option<&'a models::ThinkTiming>, is_streaming_msg: bool) -> Element<'a, Message> {
    use desktop_chat_view::message_view::think_block::{self as local, ThinkBlockState};
    local::think_block_view_with_streaming(ThinkBlockState {
        show_reasoning_summary: app.dialogue_flow_show_reasoning_summary,
        expanded: &app.chat_think_expanded, collapsed: &app.chat_think_collapsed,
        hovered: app.chat_think_hovered_idx, editors: &app.chat_think_editors,
        i18n: crate::app::i18n::I18n::new(app.desktop_locale),
        now_ms: crate::app::time::now_ms(), animation_frame: app.status_animation_frame,
    }, msg_idx, think_idx, content, open, timing, is_streaming_msg).map(|event| Message::Chat(match event {
        local::Message::ThinkHover(msg, idx) => message::ChatMessage::ThinkHover(msg, idx),
        local::Message::ThinkHoverLeave => message::ChatMessage::ThinkHoverLeave,
        local::Message::ToggleThink(msg, idx, default) => message::ChatMessage::ToggleThink(msg, idx, default),
        local::Message::ThinkEditorAction(msg, idx, action) => message::ChatMessage::ThinkEditorAction(msg, idx, action),
    }))
}
