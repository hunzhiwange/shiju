//! Inline Markdown, web links, and GFM tables in assistant replies.

use iced::widget::text::{LineHeight, Span, Wrapping};
use iced::widget::rich_text;
use iced::{Color, Element, Font, Theme};
use crate::app::Message;
use super::super::tool_text_support::chat_text_font;
use super::markdown_links::{LinkTarget, WorkspaceLinks};
use super::markdown_selection::{self, FragmentId, FragmentStyle, Selection};
use super::styles::MESSAGE_TEXT_SIZE;
pub(super) use desktop_chat_view::message_view::markdown_rich::{Inline, MarkdownTable};
pub(super) fn inline_spans<'a>(spans: &'a [Inline], theme: &Theme, links: &WorkspaceLinks<'_>) -> Vec<Span<'a, LinkTarget>> {
    desktop_chat_view::message_view::markdown_rich::inline_spans(spans, theme, |target| links.visible_target(target))
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
    body_color: Color,
    links: WorkspaceLinks<'a>,
    selection: Option<(Selection, usize)>,
) -> Element<'a, Message> {
    let theme = theme.clone();
    desktop_chat_view::message_view::markdown_rich::table_view(data, move |row, column, content, cell_width, alignment| {
        let font = if row == 0 {
            Font { weight: iced::font::Weight::Bold, ..chat_text_font() }
        } else {
            chat_text_font()
        };
        let spans = inline_spans(content, &theme, &links);
        let links = links.clone();
        let on_link = move |target| links.open(target);
        if let Some((selection, section)) = &selection {
            markdown_selection::fragment(
                selection.clone(),
                FragmentId { section: *section, row, column },
                spans,
                FragmentStyle {
                    size: MESSAGE_TEXT_SIZE,
                    color: Some(body_color),
                    font,
                    width: cell_width,
                    align: alignment.into(),
                },
                on_link,
            )
        } else {
            rich_text(spans)
                .size(MESSAGE_TEXT_SIZE)
                .color(body_color)
                .font(font)
                .width(cell_width)
                .align_x(alignment)
                .line_height(LineHeight::Relative(1.6))
                .wrapping(Wrapping::WordOrGlyph)
                .on_link_click(on_link)
                .into()
        }
    })
}
