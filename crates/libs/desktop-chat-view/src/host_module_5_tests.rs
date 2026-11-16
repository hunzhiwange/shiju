//! Complete message rendering suite migrated from the desktop host.
mod assistant_body;
mod fixture;
mod markdown;
mod markdown_links;
mod markdown_rich;
pub use crate::message_view::contracts::processing_duration_label;
pub use crate::message_view::parse::hash_chat_content;
pub use crate::message_view::render_cache::build_render_cache_entry;
use fixture::{Message, RenderFixture};
use iced::Element;
mod text {
    pub use crate::message_view::text::message_text_body;
}
mod assistant_error {
    pub fn assistant_api_error_view<'a>(
        app: &'a super::RenderFixture,
        m: usize,
        t: Option<usize>,
        content: &str,
    ) -> Option<iced::Element<'a, super::Message>> {
        crate::message_view::assistant_error_render::assistant_api_error_view(
            &app.snapshot(),
            m,
            t,
            content,
        )
    }
}
mod think_block {
    pub fn think_block_view<'a>(
        app: &'a super::RenderFixture,
        m: usize,
        t: usize,
        content: String,
        open: bool,
        timing: Option<&'a session_ui_types::ThinkTiming>,
    ) -> iced::Element<'a, super::Message> {
        crate::message_view::render_support::think_block_view_with_streaming(
            &app.snapshot(),
            m,
            t,
            content,
            open,
            timing,
            app.is_requesting && m + 1 == app.chat.len(),
        )
    }
}
fn selected_chat_text_for_message(app: &RenderFixture, m: usize) -> Option<String> {
    desktop_chat_tools::tool_text_support::selected_chat_text_for_message(&app.text_state(), m)
}
fn message_view<'a>(
    app: &'a RenderFixture,
    m: usize,
    role: session_ui_types::ChatRole,
    content: &'a str,
    timing: &'a [session_ui_types::ThinkTiming],
    processing: Option<&session_ui_types::ProcessingTiming>,
    meta: Option<std::borrow::Cow<'a, str>>,
    cache: &'a crate::render_cache::ChatRenderCacheEntry,
    heavy: bool,
) -> Element<'a, Message> {
    crate::message_view::bubble::message_view_with_runtime(
        &app.snapshot(),
        m,
        role,
        content,
        timing,
        processing,
        meta,
        cache,
        heavy,
        app.is_requesting,
        None,
    )
}
fn copy_chat_message_action(s: &str, m: usize) -> Message {
    fixture::action(crate::message_view::bubble::copy_chat_message_action(s, m))
}
fn open_chat_message_context_menu_action(t: u64, p: iced::Point, s: &str, m: usize) -> Message {
    fixture::action(crate::message_view::bubble::open_chat_message_context_menu_action(t, p, s, m))
}
mod assistant_body_tests;
mod assistant_error_tests;
mod markdown_selection_tests;
mod markdown_tests;
mod message_view_tests;
mod think_block_tests;
