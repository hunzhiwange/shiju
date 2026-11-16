//! Inline Markdown, web links, and GFM tables in assistant replies.

use super::fixture::Message;
use super::markdown_links::{LinkTarget, WorkspaceLinks};
pub(super) use crate::message_view::markdown_rich::{Inline, MarkdownTable};
use crate::message_view::markdown_selection::Selection;
use desktop_chat_tools::tool_text_support::chat_text_font;
use iced::widget::rich_text;
use iced::widget::text::{LineHeight, Span, Wrapping};
use iced::{Color, Element, Theme};
pub(super) fn inline_spans<'a>(
    spans: &'a [Inline],
    theme: &Theme,
    links: &WorkspaceLinks<'_>,
) -> Vec<Span<'a, LinkTarget>> {
    crate::message_view::markdown_rich::inline_spans(spans, theme, |target| {
        links.visible_target(target)
    })
}

#[cfg(test)]
pub(super) fn inline_view<'a>(
    spans: &'a [Inline],
    size: f32,
    body_color: Color,
    theme: &Theme,
    on_link: impl Fn(LinkTarget) -> Message + 'a,
) -> iced::widget::text::Rich<'a, LinkTarget, Message> {
    rich_text(inline_spans(spans, theme, &WorkspaceLinks::default()))
        .size(size)
        .font(chat_text_font())
        .color(body_color)
        .line_height(LineHeight::Relative(1.6))
        .wrapping(Wrapping::WordOrGlyph)
        .on_link_click(on_link)
}

pub(super) fn table_view<'a>(
    data: &'a MarkdownTable,
    theme: &Theme,
    color: Color,
    links: WorkspaceLinks<'a>,
    selection: Option<(Selection, usize)>,
) -> Element<'a, Message> {
    crate::message_view::markdown_body::table_view(data, theme, color, links.input(), selection)
}
