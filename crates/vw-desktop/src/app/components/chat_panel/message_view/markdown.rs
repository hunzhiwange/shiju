//! 助手消息正文的轻量 Markdown 渲染。
//!
//! 普通正文保留只读编辑器，代码块、表格和包含链接的块单独渲染。

use iced::widget::{column, text, text_editor, rule};
use iced::{Element, Length};
use crate::app::components::text_editor_scroll_panel::pass_wheel_to_parent;
use crate::app::{App, Message, message};
use super::super::tool_text_support::{READ_ONLY_EDITOR_MAX_HEIGHT, chat_text_font, read_only_text_style};
use super::markdown_code::code_block_view;
use super::markdown_links::WorkspaceLinks;
use super::markdown_rich;
use super::markdown_selection::{self, FragmentId, FragmentStyle, Selection};
use super::styles::{MESSAGE_TEXT_SIZE, message_body_text_color};
pub(crate) use desktop_chat_view::message_view::markdown::*;
/// 渲染单个 Markdown 块为可划选元素。
fn markdown_block_view<'a>(
    app: &'a App,
    block_idx: usize,
    section: &'a [MarkdownBlock],
    selection: &Selection,
    links: &WorkspaceLinks<'a>,
) -> Element<'a, Message> {
    let block = &section[0];
    let theme = app.theme();
    let size = block_text_size(&block.kind);
    let font = if matches!(block.kind, MarkdownBlockKind::Heading(_)) {
        iced::Font { weight: iced::font::Weight::Bold, ..chat_text_font() }
    } else {
        markdown_block_font(block)
    };
    let body_color = message_body_text_color(&theme, false);

    match &block.kind {
        MarkdownBlockKind::Table(table) => {
            return markdown_rich::table_view(
                table,
                &theme,
                body_color,
                links.clone(),
                Some((selection.clone(), block_idx)),
            );
        }
        MarkdownBlockKind::Rule => return rule::horizontal(1).into(),
        _ => {}
    }

    let spans = if section.len() > 1 {
        vec![iced::widget::span(markdown_editor_text(section))]
    } else if block.inline.is_empty() {
        vec![iced::widget::span(block.text.as_str())]
    } else {
        markdown_rich::inline_spans(&block.inline, &theme, links)
    };
    let links = links.clone();
    let editor = markdown_selection::fragment(
        selection.clone(),
        FragmentId { section: block_idx, row: 0, column: 0 },
        spans,
        FragmentStyle {
            size,
            color: Some(body_color),
            font,
            width: Length::Fill,
            align: iced::widget::text::Alignment::Default,
        },
        move |target| links.open(target),
    );

    desktop_chat_view::message_view::markdown::decorate_block(block, editor, body_color, |language, code, editor| code_block_view(app, language, code, editor))
}

/// 保留相邻普通块的连续选区，代码块、表格和链接块使用独立视图。
pub(crate) fn markdown_body_view<'a>(
    app: &'a App,
    msg_idx: usize,
    text_idx: usize,
    blocks: &'a [MarkdownBlock],
) -> Element<'a, Message> {
    let mut body = column![].spacing(markdown_visual_contract().block_spacing).width(Length::Fill);
    if blocks.iter().all(MarkdownBlock::joins_prose) {
        let key = markdown_block_key(msg_idx, text_idx, 0);
        let body_color = message_body_text_color(&app.theme(), false);
        if !blocks.is_empty()
            && let Some(content) = app.chat_markdown_block_editors.get(&key)
        {
            body = body.push(pass_wheel_to_parent(
                text_editor(content)
                    .on_action(move |action| {
                        Message::Chat(message::ChatMessage::MarkdownBlockEditorAction(
                            msg_idx, text_idx, 0, action,
                        ))
                    })
                    .size(MESSAGE_TEXT_SIZE)
                    .font(chat_text_font())
                    .padding(0)
                    .height(Length::Shrink)
                    .max_height(READ_ONLY_EDITOR_MAX_HEIGHT)
                    .line_height(iced::widget::text::LineHeight::Relative(1.6))
                    .style(move |theme, _| read_only_text_style(theme, body_color)),
            ));
        } else if !blocks.is_empty() {
            body = body.push(
                text(markdown_editor_text(blocks))
                    .size(MESSAGE_TEXT_SIZE)
                    .font(chat_text_font())
                    .line_height(iced::widget::text::LineHeight::Relative(1.6))
                    .color(body_color),
            );
        }
        return markdown_selection::area(body.into(), Selection::default(), app, msg_idx, text_idx);
    }

    let selection = Selection::default();
    let links = WorkspaceLinks::new(app);
    for (index, section) in markdown_sections(blocks).enumerate() {
        body = body.push(markdown_block_view(app, index, section, &selection, &links));
    }
    markdown_selection::area(body.into(), selection, app, msg_idx, text_idx)
}
