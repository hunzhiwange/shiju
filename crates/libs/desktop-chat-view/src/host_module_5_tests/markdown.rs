pub use crate::message_view::markdown::*;
pub fn markdown_body_view<'a>(
    app: &'a super::fixture::RenderFixture,
    m: usize,
    t: usize,
    blocks: &'a [MarkdownBlock],
) -> iced::Element<'a, super::fixture::Message> {
    crate::message_view::render_support::markdown_body_view(&app.snapshot(), m, t, blocks)
}
