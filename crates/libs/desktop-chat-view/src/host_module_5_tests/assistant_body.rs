use super::fixture::{Message, RenderFixture};
use crate::message_view::ParsedChatBlock;
pub use crate::message_view::assistant_body::{
    deduped_tool_last_indices, deduped_tool_last_indices_with_metadata,
};
pub use crate::message_view::assistant_render::count_visible_non_explore_tool_cards;
use iced::widget::Column;
use session_ui_types as models;
pub(super) fn render_assistant_body<'a, T, U, V, W>(
    app: &'a RenderFixture,
    idx: usize,
    think_timing: &'a [models::ThinkTiming],
    special_text_blocks: T,
    display_text: U,
    blocks: V,
    tool_block_metadata: W,
    has_special: bool,
    is_streaming_assistant: bool,
    visible_for_copy: &str,
) -> Column<'a, Message>
where
    T: AsRef<[String]>,
    U: AsRef<str>,
    V: AsRef<[ParsedChatBlock]>,
    W: AsRef<[Option<crate::render_cache::ToolBlockRenderMetadata>]>,
{
    crate::message_view::assistant_render::render_assistant_body(
        &app.snapshot(),
        idx,
        think_timing,
        special_text_blocks,
        display_text,
        blocks,
        tool_block_metadata,
        has_special,
        is_streaming_assistant,
        visible_for_copy,
    )
}
