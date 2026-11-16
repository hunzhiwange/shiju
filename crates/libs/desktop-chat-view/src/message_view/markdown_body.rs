//! Markdown body orchestration over a borrowed local render snapshot.
use super::markdown::*;
use super::markdown_links::LinkTarget;
use super::markdown_rich::{Inline, MarkdownTable};
use super::markdown_selection::{self, FragmentId, FragmentStyle, Selection, SelectionMessage};
use super::styles::{MESSAGE_TEXT_SIZE, message_body_text_color};
use desktop_chat_tools::tool_text_support::{
    READ_ONLY_EDITOR_MAX_HEIGHT, chat_text_font, read_only_text_style,
};
use desktop_ui_controls::text_editor_scroll_panel::pass_wheel_to_parent;
use iced::widget::{column, rule, text, text_editor};
use iced::{Element, Length, Theme};
use std::{collections::HashMap, rc::Rc};

pub struct WorkspaceLinks<'a, M> {
    pub visible: Rc<dyn Fn(&LinkTarget) -> Option<LinkTarget> + 'a>,
    pub open: Rc<dyn Fn(LinkTarget) -> M + 'a>,
}
impl<M> Clone for WorkspaceLinks<'_, M> {
    fn clone(&self) -> Self {
        Self { visible: self.visible.clone(), open: self.open.clone() }
    }
}
pub struct MarkdownView<'a, M> {
    pub theme: Theme,
    pub editors: &'a HashMap<u128, text_editor::Content>,
    pub session_id: Option<&'a str>,
    pub selected: bool,
    pub links: WorkspaceLinks<'a, M>,
    pub editor_action: fn(usize, usize, usize, text_editor::Action) -> M,
    pub code_block: Rc<dyn Fn(Option<&'a str>, &'a str, Element<'a, M>) -> Element<'a, M> + 'a>,
}
fn inline_spans<'a, M>(
    spans: &'a [Inline],
    theme: &Theme,
    links: &WorkspaceLinks<'_, M>,
) -> Vec<iced::widget::text::Span<'a, LinkTarget>> {
    super::markdown_rich::inline_spans(spans, theme, |target| (links.visible)(target))
}
use iced::widget::rich_text;
use iced::widget::text::{LineHeight, Wrapping};
use iced::{Color, Font};
/// 渲染单个 Markdown 块为可划选元素。
fn markdown_block_view<'a, M: SelectionMessage + 'a>(
    view: &MarkdownView<'a, M>,
    block_idx: usize,
    section: &'a [MarkdownBlock],
    selection: &Selection,
    links: &WorkspaceLinks<'a, M>,
) -> Element<'a, M> {
    let block = &section[0];
    let theme = view.theme.clone();
    let size = block_text_size(&block.kind);
    let font = if matches!(block.kind, MarkdownBlockKind::Heading(_)) {
        iced::Font { weight: iced::font::Weight::Bold, ..chat_text_font() }
    } else {
        markdown_block_font(block)
    };
    let body_color = message_body_text_color(&theme, false);

    match &block.kind {
        MarkdownBlockKind::Table(table) => {
            return table_view(
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
        inline_spans(&block.inline, &theme, links)
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
        move |target| (links.open)(target),
    );

    desktop_chat_view::message_view::markdown::decorate_block(
        block,
        editor,
        body_color,
        |language, code, editor| (view.code_block)(language, code, editor),
    )
}

/// 保留相邻普通块的连续选区，代码块、表格和链接块使用独立视图。
pub fn markdown_body_view<'a, M: SelectionMessage + 'a>(
    view: &MarkdownView<'a, M>,
    msg_idx: usize,
    text_idx: usize,
    blocks: &'a [MarkdownBlock],
) -> Element<'a, M> {
    let editor_action = view.editor_action;
    let mut body = column![].spacing(markdown_visual_contract().block_spacing).width(Length::Fill);
    if blocks.iter().all(MarkdownBlock::joins_prose) {
        let key = markdown_block_key(msg_idx, text_idx, 0);
        let body_color = message_body_text_color(&view.theme.clone(), false);
        if !blocks.is_empty()
            && let Some(content) = view.editors.get(&key)
        {
            body = body.push(pass_wheel_to_parent(
                text_editor(content)
                    .on_action(move |action| (editor_action)(msg_idx, text_idx, 0, action))
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
        return markdown_selection::area(
            body.into(),
            Selection::default(),
            view.session_id,
            view.selected,
            msg_idx,
            text_idx,
        );
    }

    let selection = Selection::default();
    let links = view.links.clone();
    for (index, section) in markdown_sections(blocks).enumerate() {
        body = body.push(markdown_block_view(view, index, section, &selection, &links));
    }
    markdown_selection::area(
        body.into(),
        selection,
        view.session_id,
        view.selected,
        msg_idx,
        text_idx,
    )
}

fn table_view<'a, M: SelectionMessage + 'a>(
    data: &'a MarkdownTable,
    theme: &Theme,
    body_color: Color,
    links: WorkspaceLinks<'a, M>,
    selection: Option<(Selection, usize)>,
) -> Element<'a, M> {
    let theme = theme.clone();
    super::markdown_rich::table_view(data, move |row, column, content, cell_width, alignment| {
        let font = if row == 0 {
            Font { weight: iced::font::Weight::Bold, ..chat_text_font() }
        } else {
            chat_text_font()
        };
        let spans = inline_spans(content, &theme, &links);
        let links = links.clone();
        let on_link = move |target| (links.open)(target);
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
