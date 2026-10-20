//! 助手消息正文的轻量 Markdown 渲染。
//!
//! iced 自带的 markdown widget 基于 rich_text，无法划选复制。为兼顾块级格式与
//! 复制能力，这里自实现一个简易块级解析：把正文切成标题/段落/列表/代码块/引用，
//! 每个块用只读 `text_editor` 渲染（原生支持划选复制），行内 `**`、`` ` ``、链接
//! 等标记被清理为纯文本，仅保留块级排版。

use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::mouse;
use iced::widget::{Space, column, container, row, text, text_editor};
use iced::{
    Background, Border, Color, Element, Event, Length, Padding, Point, Rectangle, Size, Theme,
};

use crate::app::components::text_editor_scroll_panel::pass_wheel_to_parent;
use crate::app::{App, Message, message};

use super::super::tool_text_support::{
    READ_ONLY_EDITOR_MAX_HEIGHT, chat_text_font, chat_text_line_height, read_only_text_style,
};
use super::styles::{
    MARKDOWN_BLOCK_SPACING, MESSAGE_TEXT_SIZE, is_dark_theme, message_body_text_color,
};

/// Markdown 块类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MarkdownBlockKind {
    /// 标题，携带级别（1-6）。
    Heading(u8),
    /// 普通段落。
    Paragraph,
    /// 无序列表项，携带缩进层级（0 起）。
    BulletItem(u8),
    /// 有序列表项，携带序号文本（如 "1."）。
    OrderedItem(String),
    /// 代码块，携带语言标注（可空）。
    CodeBlock(Option<String>),
    /// 引用块。
    Quote,
}

/// 单个 Markdown 块：类型 + 已清理为纯文本的内容。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarkdownBlock {
    pub(crate) kind: MarkdownBlockKind,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MarkdownVisualContract {
    pub(crate) heading_sizes: [f32; 6],
    pub(crate) block_spacing: f32,
    pub(crate) list_base_indent: f32,
    pub(crate) list_nested_indent: f32,
    pub(crate) quote_bar_width: f32,
    pub(crate) quote_content_inset: f32,
    pub(crate) code_padding: Padding,
    pub(crate) code_corner_radius: f32,
    pub(crate) code_border_width: f32,
}

pub(crate) fn markdown_visual_contract() -> MarkdownVisualContract {
    MarkdownVisualContract {
        heading_sizes: [
            MESSAGE_TEXT_SIZE + 6.0,
            MESSAGE_TEXT_SIZE + 4.0,
            MESSAGE_TEXT_SIZE + 2.0,
            MESSAGE_TEXT_SIZE + 1.0,
            MESSAGE_TEXT_SIZE + 1.0,
            MESSAGE_TEXT_SIZE + 1.0,
        ],
        block_spacing: MARKDOWN_BLOCK_SPACING,
        list_base_indent: 18.0,
        list_nested_indent: 14.0,
        quote_bar_width: 2.0,
        quote_content_inset: 12.0,
        code_padding: [10, 12].into(),
        code_corner_radius: 8.0,
        code_border_width: 1.0,
    }
}

/// Markdown 块编辑器键：消息索引 + 文本块索引 + 块索引。
pub(crate) fn markdown_block_key(msg_idx: usize, text_idx: usize, block_idx: usize) -> u128 {
    ((msg_idx as u128) << 64) | ((text_idx as u128) << 32) | (block_idx as u128)
}

/// 把 Markdown 链接 URI 映射为外部打开消息。
///
/// 当前块级渲染把链接清理为纯文本（仅保留标签），暂未提供可点击链接；
/// 该映射保留给后续“可点击链接”能力与测试复用。
#[cfg(test)]
pub(crate) fn link_click_message(url: String) -> Message {
    Message::View(message::ViewMessage::OpenUrlExternal(url))
}

/// 去除行内 Markdown 标记，仅保留可读纯文本。
///
/// 处理 `**粗**` / `*斜*` / `__粗__` / `_斜_` / `` `码` `` / `~~删~~` /
/// `[文本](链接)` / 裸 `<url>`。不做嵌套语法解析，按常见写法线性清理。
pub(crate) fn strip_inline_markdown(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        match c {
            // 行内代码：保留内容，去掉反引号。
            '`' => {
                let mut ticks = 0;
                while i < chars.len() && chars[i] == '`' {
                    ticks += 1;
                    i += 1;
                }
                let fence: String = std::iter::repeat('`').take(ticks).collect();
                let start = i;
                while i < chars.len() {
                    if chars[i] == '`' {
                        let mut run = 0;
                        while i + run < chars.len() && chars[i + run] == '`' {
                            run += 1;
                        }
                        if run == ticks {
                            out.extend(&chars[start..i]);
                            i += ticks;
                            break;
                        }
                    }
                    i += 1;
                    if i >= chars.len() {
                        // 未闭合，原样吐回内容（含起始反引号）。
                        out.push_str(&fence);
                        out.extend(&chars[start..]);
                    }
                }
            }
            // 强调标记：成对的 * 或 _，去掉标记保留内容。
            '*' | '_' => {
                let run_start = i;
                let mut marks = 0;
                while i < chars.len() && chars[i] == c {
                    marks += 1;
                    i += 1;
                }
                // 两侧都是空白/边界视为独立符号（如乘号），原样保留。
                let prev_is_space = run_start == 0 || chars[run_start - 1].is_whitespace();
                let next_is_space = i >= chars.len() || chars[i].is_whitespace();
                if marks > 3 || (prev_is_space && next_is_space) {
                    for _ in 0..marks {
                        out.push(c);
                    }
                }
                // 否则丢弃标记（开标记与闭标记都会落入此分支被去掉）。
            }
            // 删除线 ~~：去标记保留内容。
            '~' if i + 1 < chars.len() && chars[i + 1] == '~' => {
                i += 2;
            }
            // 链接 [文本](url)：保留文本。
            '[' => {
                if let Some((label, consumed)) = parse_inline_link(&chars[i..]) {
                    out.push_str(&label);
                    i += consumed;
                } else {
                    out.push('[');
                    i += 1;
                }
            }
            // 裸尖括号链接 <https://...>：保留 URL。
            '<' => {
                if let Some((url, consumed)) = parse_autolink(&chars[i..]) {
                    out.push_str(&url);
                    i += consumed;
                } else {
                    out.push('<');
                    i += 1;
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }

    out
}

/// 解析 `[文本](链接)`，返回（文本, 消费字符数）。
fn parse_inline_link(chars: &[char]) -> Option<(String, usize)> {
    debug_assert_eq!(chars.first(), Some(&'['));
    let close = chars.iter().position(|&c| c == ']')?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let paren_close = chars[close + 2..].iter().position(|&c| c == ')')? + close + 2;
    let label: String = chars[1..close].iter().collect();
    Some((strip_inline_markdown(&label), paren_close + 1))
}

/// 解析 `<https://...>` 自动链接，返回（URL, 消费字符数）。
fn parse_autolink(chars: &[char]) -> Option<(String, usize)> {
    debug_assert_eq!(chars.first(), Some(&'<'));
    let close = chars.iter().position(|&c| c == '>')?;
    let inner: String = chars[1..close].iter().collect();
    if inner.starts_with("http://") || inner.starts_with("https://") {
        Some((inner, close + 1))
    } else {
        None
    }
}

/// 把一段正文解析为 Markdown 块列表。
///
/// 仅识别常见块级语法：ATX 标题、围栏代码块、无序/有序列表、引用、空行分段。
/// 不支持表格、缩进式代码块等复杂结构（退化为普通段落）。
pub(crate) fn parse_markdown_blocks(input: &str) -> Vec<MarkdownBlock> {
    let mut blocks: Vec<MarkdownBlock> = Vec::new();
    let mut paragraph: Vec<String> = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    let flush_paragraph = |blocks: &mut Vec<MarkdownBlock>, paragraph: &mut Vec<String>| {
        if paragraph.is_empty() {
            return;
        }
        let joined = paragraph.join("\n");
        paragraph.clear();
        let cleaned = strip_inline_markdown(joined.trim());
        if !cleaned.trim().is_empty() {
            blocks.push(MarkdownBlock { kind: MarkdownBlockKind::Paragraph, text: cleaned });
        }
    };

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim_start();

        // 围栏代码块。
        if let Some(lang) = trimmed.strip_prefix("```").or_else(|| trimmed.strip_prefix("~~~")) {
            flush_paragraph(&mut blocks, &mut paragraph);
            let fence = if trimmed.starts_with("```") { "```" } else { "~~~" };
            let language = lang.trim();
            let language = (!language.is_empty()).then(|| language.to_string());
            let mut code_lines: Vec<&str> = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with(fence) {
                code_lines.push(lines[i]);
                i += 1;
            }
            // 跳过闭合围栏。
            if i < lines.len() {
                i += 1;
            }
            blocks.push(MarkdownBlock {
                kind: MarkdownBlockKind::CodeBlock(language),
                text: code_lines.join("\n"),
            });
            continue;
        }

        // 空行：结束当前段落。
        if trimmed.is_empty() {
            flush_paragraph(&mut blocks, &mut paragraph);
            i += 1;
            continue;
        }

        // 水平分割线视为空段落分隔（不单独渲染）。
        if is_thematic_break(trimmed) {
            flush_paragraph(&mut blocks, &mut paragraph);
            i += 1;
            continue;
        }

        // ATX 标题。
        if let Some((level, rest)) = parse_atx_heading(trimmed) {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock {
                kind: MarkdownBlockKind::Heading(level),
                text: strip_inline_markdown(rest.trim()),
            });
            i += 1;
            continue;
        }

        // 引用块。
        if let Some(rest) = trimmed.strip_prefix("> ").or_else(|| trimmed.strip_prefix(">")) {
            flush_paragraph(&mut blocks, &mut paragraph);
            let mut quote_lines = vec![rest];
            i += 1;
            while i < lines.len() {
                let q = lines[i].trim_start();
                if let Some(rest) = q.strip_prefix("> ").or_else(|| q.strip_prefix(">")) {
                    quote_lines.push(rest);
                    i += 1;
                } else {
                    break;
                }
            }
            blocks.push(MarkdownBlock {
                kind: MarkdownBlockKind::Quote,
                text: strip_inline_markdown(quote_lines.join("\n").trim()),
            });
            continue;
        }

        // 无序列表项。
        if let Some((indent, rest)) = parse_bullet_item(line) {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock {
                kind: MarkdownBlockKind::BulletItem(indent),
                text: strip_inline_markdown(rest.trim()),
            });
            i += 1;
            continue;
        }

        // 有序列表项。
        if let Some((marker, rest)) = parse_ordered_item(trimmed) {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock {
                kind: MarkdownBlockKind::OrderedItem(marker),
                text: strip_inline_markdown(rest.trim()),
            });
            i += 1;
            continue;
        }

        // 普通文本，累积为段落。
        paragraph.push(line.to_string());
        i += 1;
    }

    flush_paragraph(&mut blocks, &mut paragraph);
    blocks
}

fn is_thematic_break(trimmed: &str) -> bool {
    let compact: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    compact.len() >= 3
        && (compact.chars().all(|c| c == '-')
            || compact.chars().all(|c| c == '*')
            || compact.chars().all(|c| c == '_'))
}

fn parse_atx_heading(trimmed: &str) -> Option<(u8, &str)> {
    let hashes = trimmed.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    // 标题井号后必须是空格，否则视为普通文本（如 "#标签"）。
    if rest.starts_with(' ') { Some((hashes as u8, rest.trim_start())) } else { None }
}

fn parse_bullet_item(line: &str) -> Option<(u8, &str)> {
    let indent_spaces = line.len() - line.trim_start().len();
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))?;
    // 每 2 空格算一级缩进，最多 4 级。
    let level = ((indent_spaces / 2).min(4)) as u8;
    Some((level, rest))
}

fn parse_ordered_item(trimmed: &str) -> Option<(String, &str)> {
    let digits = trimmed.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 || digits > 3 {
        return None;
    }
    let after = &trimmed[digits..];
    let rest = after.strip_prefix(". ").or_else(|| after.strip_prefix(") "))?;
    let sep = if after.starts_with(". ") { "." } else { ")" };
    Some((format!("{}{}", &trimmed[..digits], sep), rest))
}

fn block_text_size(kind: &MarkdownBlockKind) -> f32 {
    let contract = markdown_visual_contract();
    match kind {
        MarkdownBlockKind::Heading(level) => {
            contract.heading_sizes[usize::from((*level).clamp(1, 6) - 1)]
        }
        MarkdownBlockKind::CodeBlock(_) => MESSAGE_TEXT_SIZE - 1.0,
        _ => MESSAGE_TEXT_SIZE,
    }
}

pub(crate) fn markdown_block_font(block: &MarkdownBlock) -> iced::Font {
    if matches!(block.kind, MarkdownBlockKind::CodeBlock(_)) && block.text.is_ascii() {
        iced::Font::MONOSPACE
    } else {
        chat_text_font()
    }
}

/// 将 Markdown 块转换为单个只读编辑器使用的连续文本。
pub(crate) fn markdown_editor_text(blocks: &[MarkdownBlock]) -> String {
    blocks
        .iter()
        .map(|block| match &block.kind {
            MarkdownBlockKind::BulletItem(level) => {
                format!("{}• {}", "  ".repeat(usize::from(*level)), block.text)
            }
            MarkdownBlockKind::OrderedItem(marker) => format!("{marker} {}", block.text),
            MarkdownBlockKind::Quote => {
                block.text.lines().map(|line| format!("│ {line}")).collect::<Vec<_>>().join("\n")
            }
            MarkdownBlockKind::Heading(_)
            | MarkdownBlockKind::Paragraph
            | MarkdownBlockKind::CodeBlock(_) => block.text.clone(),
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// 渲染单个 Markdown 块为可划选元素。
fn markdown_block_view<'a>(
    app: &'a App,
    msg_idx: usize,
    text_idx: usize,
    block_idx: usize,
    block: &'a MarkdownBlock,
    whole_response: bool,
) -> Element<'a, Message> {
    let contract = markdown_visual_contract();
    let theme = app.theme();
    let key = markdown_block_key(msg_idx, text_idx, block_idx);
    let size = if whole_response { MESSAGE_TEXT_SIZE } else { block_text_size(&block.kind) };
    let font = if whole_response { chat_text_font() } else { markdown_block_font(block) };
    let body_color = message_body_text_color(&theme, false);

    let editor: Element<'a, Message> =
        if let Some(content) = app.chat_markdown_block_editors.get(&key) {
            pass_wheel_to_parent(
                text_editor(content)
                    .on_action(move |action| {
                        Message::Chat(message::ChatMessage::MarkdownBlockEditorAction(
                            msg_idx, text_idx, block_idx, action,
                        ))
                    })
                    .size(size)
                    .line_height(chat_text_line_height())
                    .padding(0)
                    .height(Length::Shrink)
                    .max_height(READ_ONLY_EDITOR_MAX_HEIGHT)
                    .font(font)
                    .style(move |theme: &Theme, _status| read_only_text_style(theme, body_color)),
            )
        } else {
            // 编辑器缺失时回退为静态文本，仍可被气泡复制按钮覆盖。
            text(block.text.clone())
                .size(size)
                .font(font)
                .line_height(chat_text_line_height())
                .style(move |_theme: &Theme| iced::widget::text::Style { color: Some(body_color) })
                .into()
        };

    if whole_response {
        return container(editor).width(Length::Fill).into();
    }

    match &block.kind {
        MarkdownBlockKind::CodeBlock(_) => container(editor)
            .padding(contract.code_padding)
            .width(Length::Fill)
            .style(move |theme: &Theme| code_block_style(theme))
            .into(),
        MarkdownBlockKind::Quote => {
            let line_count = block.text.lines().count().max(1) as f32;
            let quote_bar_height = size * 1.52 * line_count;
            row![
                container(Space::new())
                    .width(Length::Fixed(contract.quote_bar_width))
                    .height(Length::Fixed(quote_bar_height))
                    .style(move |theme: &Theme| quote_bar_style(theme)),
                container(editor)
                    .padding(iced::Padding {
                        top: 0.0,
                        right: 0.0,
                        bottom: 0.0,
                        left: contract.quote_content_inset,
                    })
                    .width(Length::Fill),
            ]
            .into()
        }
        MarkdownBlockKind::BulletItem(level) => {
            let indent =
                contract.list_base_indent + f32::from(*level) * contract.list_nested_indent;
            row![
                container(text("•").size(size).style(move |_theme: &Theme| {
                    iced::widget::text::Style { color: Some(body_color) }
                }))
                .padding(iced::Padding { top: 0.0, right: 6.0, bottom: 0.0, left: 0.0 })
                .width(Length::Fixed(indent))
                .align_x(iced::alignment::Horizontal::Right),
                container(editor).width(Length::Fill),
            ]
            .into()
        }
        MarkdownBlockKind::OrderedItem(marker) => row![
            container(text(marker.clone()).size(size).font(chat_text_font()).style(
                move |_theme: &Theme| iced::widget::text::Style { color: Some(body_color) }
            ))
            .padding(iced::Padding { top: 0.0, right: 8.0, bottom: 0.0, left: 0.0 })
            .width(Length::Shrink),
            container(editor).width(Length::Fill),
        ]
        .into(),
        MarkdownBlockKind::Heading(_) | MarkdownBlockKind::Paragraph => {
            container(editor).width(Length::Fill).into()
        }
    }
}

fn code_block_style(theme: &Theme) -> iced::widget::container::Style {
    let contract = markdown_visual_contract();
    let is_dark = is_dark_theme(theme);
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba8(255, 255, 255, 0.06)
        } else {
            Color::from_rgba8(15, 23, 42, 0.05)
        })),
        border: Border {
            width: contract.code_border_width,
            color: if is_dark {
                Color::from_rgba8(255, 255, 255, 0.10)
            } else {
                Color::from_rgba8(15, 23, 42, 0.10)
            },
            radius: contract.code_corner_radius.into(),
        },
        ..Default::default()
    }
}

fn quote_bar_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(theme.palette().text.scale_alpha(0.28))),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 2.0.into() },
        ..Default::default()
    }
}

/// 使用单个只读编辑器渲染整段助手正文。
pub(crate) fn markdown_body_view<'a>(
    app: &'a App,
    msg_idx: usize,
    text_idx: usize,
    blocks: &'a [MarkdownBlock],
) -> Element<'a, Message> {
    let mut body = column![].spacing(markdown_visual_contract().block_spacing).width(Length::Fill);
    if let Some(block) = blocks.first() {
        let key = markdown_block_key(msg_idx, text_idx, 0);
        if app.chat_markdown_block_editors.contains_key(&key) {
            body = body.push(markdown_block_view(app, msg_idx, text_idx, 0, block, true));
        } else {
            let body_color = message_body_text_color(&app.theme(), false);
            body = body.push(
                text(markdown_editor_text(blocks))
                    .size(MESSAGE_TEXT_SIZE)
                    .font(chat_text_font())
                    .line_height(chat_text_line_height())
                    .style(move |_theme: &Theme| iced::widget::text::Style {
                        color: Some(body_color),
                    }),
            );
        }
    }
    Element::new(MarkdownSelectionArea {
        content: body.into(),
        msg_idx,
        text_idx,
        block_count: usize::from(!blocks.is_empty()),
    })
}

#[derive(Debug, Default)]
struct MarkdownSelectionState {
    anchor_block: Option<usize>,
    dragging: bool,
}

struct MarkdownSelectionArea<'a> {
    content: Element<'a, Message>,
    msg_idx: usize,
    text_idx: usize,
    block_count: usize,
}

fn block_at_position(layout: Layout<'_>, position: Point) -> Option<(usize, Rectangle)> {
    layout.children().enumerate().find_map(|(index, child)| {
        child.bounds().contains(position).then_some((index, child.bounds()))
    })
}

pub(crate) fn selection_actions(
    anchor: usize,
    current: usize,
    current_position: Point,
    current_bounds: Rectangle,
    block_count: usize,
) -> Vec<(usize, text_editor::Action)> {
    let (start, end) = if anchor <= current { (anchor, current) } else { (current, anchor) };
    let mut actions = Vec::with_capacity(block_count + 2);

    for block_idx in 0..block_count {
        if block_idx < start || block_idx > end {
            actions.push((block_idx, text_editor::Action::Click(Point::ORIGIN)));
        } else if block_idx != anchor && block_idx != current {
            actions.push((block_idx, text_editor::Action::SelectAll));
        }
    }

    let local = Point::new(
        (current_position.x - current_bounds.x).max(0.0),
        (current_position.y - current_bounds.y).max(0.0),
    );
    if anchor < current {
        actions.push((anchor, text_editor::Action::Drag(Point::new(f32::MAX, f32::MAX))));
        actions.push((current, text_editor::Action::Click(Point::ORIGIN)));
        actions.push((current, text_editor::Action::Drag(local)));
    } else if anchor > current {
        actions.push((anchor, text_editor::Action::Drag(Point::ORIGIN)));
        actions.push((current, text_editor::Action::Click(Point::new(f32::MAX, f32::MAX))));
        actions.push((current, text_editor::Action::Drag(local)));
    }

    actions
}

impl Widget<Message, Theme, iced::Renderer> for MarkdownSelectionArea<'_> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<MarkdownSelectionState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(MarkdownSelectionState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<MarkdownSelectionState>();
        let position = cursor.position();

        if matches!(event, Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)))
            && let Some(position) = position
            && let Some((block_idx, _)) = block_at_position(layout, position)
        {
            state.anchor_block = Some(block_idx);
            state.dragging = true;
            for other_idx in 0..self.block_count {
                if other_idx != block_idx {
                    shell.publish(Message::Chat(message::ChatMessage::MarkdownBlockEditorAction(
                        self.msg_idx,
                        self.text_idx,
                        other_idx,
                        text_editor::Action::Click(Point::ORIGIN),
                    )));
                }
            }
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if matches!(event, Event::Mouse(mouse::Event::CursorMoved { .. }))
            && state.dragging
            && let Some(anchor) = state.anchor_block
            && let Some(position) = position
            && let Some((current, bounds)) = block_at_position(layout, position)
            && current != anchor
        {
            for (block_idx, action) in
                selection_actions(anchor, current, position, bounds, self.block_count)
            {
                shell.publish(Message::Chat(message::ChatMessage::MarkdownBlockEditorAction(
                    self.msg_idx,
                    self.text_idx,
                    block_idx,
                    action,
                )));
            }
        }

        if matches!(event, Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))) {
            state.dragging = false;
            state.anchor_block = None;
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }
}
