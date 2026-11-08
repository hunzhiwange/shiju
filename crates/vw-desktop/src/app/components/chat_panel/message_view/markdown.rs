//! 助手消息正文的轻量 Markdown 渲染。
//!
//! 普通正文保留只读编辑器，代码块、表格和包含链接的块单独渲染。

use std::collections::BTreeMap;

use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::mouse;
use iced::widget::{Space, column, container, row, rule, text, text_editor};
use iced::{
    Background, Border, Color, Element, Event, Length, Padding, Point, Rectangle, Size, Theme,
};

use crate::app::components::text_editor_scroll_panel::pass_wheel_to_parent;
use crate::app::{App, Message, message};

use super::super::tool_text_support::{
    READ_ONLY_EDITOR_MAX_HEIGHT, chat_text_font, read_only_text_style,
};
use super::super::utils::markdown_code_fence;
use super::markdown_code::{code_block_view, is_package_command};
use super::markdown_links::{LinkTarget, link_click_message};
use super::markdown_rich::{self, Inline, MarkdownTable};
use super::styles::{MARKDOWN_BLOCK_SPACING, MESSAGE_TEXT_SIZE, message_body_text_color};

/// Markdown 块类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum MarkdownBlockKind {
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
    Rule,
    TaskItem {
        indent: u8,
        number: Option<String>,
        checked: bool,
    },
    Table(MarkdownTable),
}

/// 单个 Markdown 块：类型 + 已清理为纯文本的内容。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarkdownBlock {
    pub(super) kind: MarkdownBlockKind,
    pub(crate) text: String,
    pub(super) inline: Vec<Inline>,
}

impl MarkdownBlock {
    fn from_text(
        kind: MarkdownBlockKind,
        source: &str,
        references: &BTreeMap<String, String>,
    ) -> Self {
        let mut inline = markdown_rich::parse_inline(source, references);
        let text = if inline.is_empty() {
            strip_inline_markdown(source)
        } else {
            inline.iter().map(|span| span.text.as_str()).collect()
        };
        if inline.iter().all(|span| span.target.is_none() && span.style == Default::default()) {
            inline = Vec::new();
        }
        Self { kind, text, inline }
    }

    pub(crate) fn stored_bytes(&self) -> usize {
        let table_bytes = match &self.kind {
            MarkdownBlockKind::Table(table) => table.stored_bytes(),
            _ => 0,
        };
        self.text.len() + table_bytes + self.inline.iter().map(Inline::stored_bytes).sum::<usize>()
    }

    pub(crate) fn uses_editor(&self) -> bool {
        self.inline.is_empty()
            && !matches!(self.kind, MarkdownBlockKind::Table(_) | MarkdownBlockKind::Rule)
    }

    fn joins_prose(&self) -> bool {
        self.uses_editor() && matches!(self.kind, MarkdownBlockKind::Paragraph)
    }
}

pub(crate) fn markdown_sections(
    blocks: &[MarkdownBlock],
) -> impl Iterator<Item = &[MarkdownBlock]> {
    blocks.chunk_by(|left, right| left.joins_prose() && right.joins_prose())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MarkdownVisualContract {
    pub(crate) heading_sizes: [f32; 6],
    pub(crate) line_height: f32,
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
        line_height: 1.6,
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
/// 识别常见块级语法和 GFM 表格。
pub(crate) fn parse_markdown_blocks(input: &str) -> Vec<MarkdownBlock> {
    let mut blocks: Vec<MarkdownBlock> = Vec::new();
    let mut paragraph: Vec<String> = Vec::new();
    let lines: Vec<(usize, &str)> = input
        .split_inclusive('\n')
        .scan(0, |offset, line| {
            let start = *offset;
            *offset += line.len();
            Some((start, line.trim_end_matches(['\r', '\n'])))
        })
        .collect();
    let document = markdown_rich::parse_document(input);
    let mut tables = document.tables;
    let mut definitions = document.definitions;
    let from_text =
        |kind, source: &str| MarkdownBlock::from_text(kind, source, &document.references);
    let mut i = 0;

    let flush_paragraph = |blocks: &mut Vec<MarkdownBlock>, paragraph: &mut Vec<String>| {
        if paragraph.is_empty() {
            return;
        }
        let joined = paragraph.join("\n");
        paragraph.clear();
        let block = from_text(MarkdownBlockKind::Paragraph, joined.trim());
        if !block.text.trim().is_empty() || !block.inline.is_empty() {
            blocks.push(block);
        }
    };

    while i < lines.len() {
        let (start, line) = lines[i];
        let trimmed = line.trim_start();

        if let Some(end) = definitions.remove(&start) {
            flush_paragraph(&mut blocks, &mut paragraph);
            i += lines[i..].partition_point(|(offset, _)| *offset < end);
            continue;
        }

        if let Some((end, table)) = tables.remove(&start) {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock {
                kind: MarkdownBlockKind::Table(table),
                text: input[start..end].trim_end().to_owned(),
                inline: Vec::new(),
            });
            i += lines[i..].partition_point(|(offset, _)| *offset < end);
            continue;
        }

        // 围栏代码块。闭合围栏须使用相同符号且至少与开头一样长。
        if let Some((fence, info)) = markdown_code_fence(line) {
            flush_paragraph(&mut blocks, &mut paragraph);
            let language = info.split_whitespace().next().map(str::to_owned);
            i += 1;
            let start = i;
            while i < lines.len()
                && !markdown_code_fence(lines[i].1).is_some_and(|(close, suffix)| {
                    close.starts_with(fence) && suffix.trim().is_empty()
                })
            {
                i += 1;
            }
            blocks.push(MarkdownBlock {
                kind: MarkdownBlockKind::CodeBlock(language),
                text: lines[start..i].iter().map(|(_, line)| *line).collect::<Vec<_>>().join("\n"),
                inline: Vec::new(),
            });
            i += usize::from(i < lines.len());
            continue;
        }

        // 空行：结束当前段落。
        if trimmed.is_empty() {
            flush_paragraph(&mut blocks, &mut paragraph);
            i += 1;
            continue;
        }

        if !paragraph.is_empty()
            && let Some(level) = setext_level(trimmed.trim_end())
        {
            let title = paragraph.join("\n");
            paragraph.clear();
            blocks.push(from_text(MarkdownBlockKind::Heading(level), title.trim()));
            i += 1;
            continue;
        }

        if is_thematic_break(trimmed) {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(MarkdownBlock {
                kind: MarkdownBlockKind::Rule,
                text: String::new(),
                inline: Vec::new(),
            });
            i += 1;
            continue;
        }

        // ATX 标题。
        if let Some((level, rest)) = parse_atx_heading(trimmed) {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(from_text(MarkdownBlockKind::Heading(level), rest));
            i += 1;
            continue;
        }

        // 引用块。
        if let Some(rest) = trimmed.strip_prefix("> ").or_else(|| trimmed.strip_prefix(">")) {
            flush_paragraph(&mut blocks, &mut paragraph);
            let mut quote_lines = vec![rest];
            i += 1;
            while i < lines.len() {
                let q = lines[i].1.trim_start();
                if let Some(rest) = q.strip_prefix("> ").or_else(|| q.strip_prefix(">")) {
                    quote_lines.push(rest);
                    i += 1;
                } else {
                    break;
                }
            }
            blocks.push(from_text(MarkdownBlockKind::Quote, quote_lines.join("\n").trim()));
            continue;
        }

        if let Some((indent, rest)) = parse_bullet_item(line) {
            flush_paragraph(&mut blocks, &mut paragraph);
            let (kind, label) = if let Some((checked, label)) = task_marker(rest) {
                (MarkdownBlockKind::TaskItem { indent, number: None, checked }, label)
            } else {
                (MarkdownBlockKind::BulletItem(indent), rest.trim())
            };
            blocks.push(from_text(kind, label));
            i += 1;
            continue;
        }

        if let Some((marker, rest)) = parse_ordered_item(trimmed) {
            flush_paragraph(&mut blocks, &mut paragraph);
            let (kind, label) = if let Some((checked, label)) = task_marker(rest) {
                (
                    MarkdownBlockKind::TaskItem {
                        indent: list_indent(line),
                        number: Some(marker),
                        checked,
                    },
                    label,
                )
            } else {
                (MarkdownBlockKind::OrderedItem(marker), rest.trim())
            };
            blocks.push(from_text(kind, label));
            i += 1;
            continue;
        }

        // 连续出现的裸包管理器命令共享一个代码块。
        if is_package_command(line) {
            flush_paragraph(&mut blocks, &mut paragraph);
            let start = i;
            while i < lines.len() && is_package_command(lines[i].1) {
                i += 1;
            }
            blocks.push(MarkdownBlock {
                kind: MarkdownBlockKind::CodeBlock(Some("shell".to_owned())),
                text: lines[start..i]
                    .iter()
                    .map(|(_, line)| line.trim_start())
                    .collect::<Vec<_>>()
                    .join("\n"),
                inline: Vec::new(),
            });
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

fn setext_level(marker: &str) -> Option<u8> {
    let first = marker.as_bytes().first()?;
    if marker.bytes().all(|byte| byte == *first) {
        match first {
            b'=' => Some(1),
            b'-' => Some(2),
            _ => None,
        }
    } else {
        None
    }
}

fn parse_atx_heading(trimmed: &str) -> Option<(u8, &str)> {
    let hashes = trimmed.bytes().take_while(|&byte| byte == b'#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = trimmed[hashes..].strip_prefix([' ', '\t'])?.trim_end();
    let without_closing = rest.trim_end_matches('#');
    let title = if without_closing.ends_with([' ', '\t']) { without_closing } else { rest };
    Some((hashes as u8, title.trim()))
}

fn list_indent(line: &str) -> u8 {
    ((line.len().saturating_sub(line.trim_start().len()) / 2).min(4)) as u8
}

fn parse_bullet_item(line: &str) -> Option<(u8, &str)> {
    let trimmed = line.trim_start();
    let rest = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .or_else(|| trimmed.strip_prefix("+ "))?;
    Some((list_indent(line), rest))
}

fn task_marker(source: &str) -> Option<(bool, &str)> {
    let source = source.trim_start();
    let (marker, rest) = source.split_at_checked(3)?;
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }
    match marker {
        "[ ]" => Some((false, rest.trim())),
        "[x]" | "[X]" => Some((true, rest.trim())),
        _ => None,
    }
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

/// 将 Markdown 块转换为连续可选文本，块间留空行，相邻列表项保持紧凑。
pub(crate) fn markdown_editor_text(blocks: &[MarkdownBlock]) -> String {
    let mut output = String::new();
    for (index, block) in blocks.iter().enumerate() {
        if index > 0 {
            let is_list_item = |kind: &MarkdownBlockKind| {
                matches!(
                    kind,
                    MarkdownBlockKind::BulletItem(_)
                        | MarkdownBlockKind::OrderedItem(_)
                        | MarkdownBlockKind::TaskItem { .. }
                )
            };
            if is_list_item(&blocks[index - 1].kind) && is_list_item(&block.kind) {
                output.push('\n');
            } else {
                output.push_str("\n\n");
            }
        }
        let text = match &block.kind {
            MarkdownBlockKind::BulletItem(level) => {
                format!("{}• {}", "  ".repeat(usize::from(*level)), block.text)
            }
            MarkdownBlockKind::OrderedItem(marker) => format!("{marker} {}", block.text),
            MarkdownBlockKind::TaskItem { indent, number, checked } => format!(
                "{}{}[{}] {}",
                "  ".repeat(usize::from(*indent)),
                number.as_ref().map_or(String::new(), |number| format!("{number} ")),
                if *checked { 'x' } else { ' ' },
                block.text,
            ),
            MarkdownBlockKind::Rule => "---".to_owned(),
            MarkdownBlockKind::Quote => {
                block.text.lines().map(|line| format!("│ {line}")).collect::<Vec<_>>().join("\n")
            }
            MarkdownBlockKind::Heading(_)
            | MarkdownBlockKind::Paragraph
            | MarkdownBlockKind::CodeBlock(_)
            | MarkdownBlockKind::Table(_) => block.text.clone(),
        };
        output.push_str(&text);
    }
    output
}

/// 渲染单个 Markdown 块为可划选元素。
fn markdown_block_view<'a>(
    app: &'a App,
    msg_idx: usize,
    text_idx: usize,
    block_idx: usize,
    block: &'a MarkdownBlock,
) -> Element<'a, Message> {
    let join_prose = block.joins_prose();
    let contract = markdown_visual_contract();
    let theme = app.theme();
    let key = markdown_block_key(msg_idx, text_idx, block_idx);
    let size = block_text_size(&block.kind);
    let font = if matches!(block.kind, MarkdownBlockKind::Heading(_)) {
        iced::Font { weight: iced::font::Weight::Bold, ..chat_text_font() }
    } else {
        markdown_block_font(block)
    };
    let body_color = message_body_text_color(&theme, false);

    let on_link = move |target| {
        let session_directory = if matches!(
            &target,
            LinkTarget::File(path) if !std::path::Path::new(path).is_absolute() && !path.starts_with("~/")
        ) {
            app.active_session_id.as_deref().and_then(|id| app.known_session_directory(id))
        } else {
            None
        };
        link_click_message(target, session_directory.as_deref().or(app.project_path.as_deref()))
    };
    match &block.kind {
        MarkdownBlockKind::Table(table) => {
            return markdown_rich::table_view(table, &theme, body_color, on_link);
        }
        MarkdownBlockKind::Rule => return rule::horizontal(1).into(),
        _ => {}
    }

    let editor: Element<'a, Message> = if !block.inline.is_empty() {
        markdown_rich::inline_view(&block.inline, size, body_color, &theme, on_link)
            .font(font)
            .width(Length::Fill)
            .into()
    } else if let Some(content) = app.chat_markdown_block_editors.get(&key) {
        pass_wheel_to_parent(
            text_editor(content)
                .on_action(move |action| {
                    Message::Chat(message::ChatMessage::MarkdownBlockEditorAction(
                        msg_idx, text_idx, block_idx, action,
                    ))
                })
                .size(size)
                .line_height(iced::widget::text::LineHeight::Relative(contract.line_height))
                .padding(0)
                .height(Length::Shrink)
                .max_height(READ_ONLY_EDITOR_MAX_HEIGHT)
                .font(font)
                .style(move |theme: &Theme, _status| read_only_text_style(theme, body_color)),
        )
    } else {
        // 编辑器缺失时回退为静态文本，仍可被气泡复制按钮覆盖。
        text(block.text.as_str())
            .size(size)
            .font(font)
            .line_height(iced::widget::text::LineHeight::Relative(contract.line_height))
            .style(move |_theme: &Theme| iced::widget::text::Style { color: Some(body_color) })
            .into()
    };

    if join_prose {
        return container(editor).width(Length::Fill).into();
    }

    match &block.kind {
        MarkdownBlockKind::CodeBlock(language) => {
            code_block_view(app, language.as_deref(), &block.text, editor)
        }
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
            list_item_view(editor, *level, "•".to_owned(), body_color)
        }
        MarkdownBlockKind::TaskItem { indent, number, checked } => {
            let marker = if *checked { "☑" } else { "☐" };
            let marker = number
                .as_ref()
                .map_or_else(|| marker.to_owned(), |number| format!("{number} {marker}"));
            list_item_view(editor, *indent, marker, body_color)
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
        MarkdownBlockKind::Heading(_)
        | MarkdownBlockKind::Paragraph
        | MarkdownBlockKind::Table(_)
        | MarkdownBlockKind::Rule => container(editor).width(Length::Fill).into(),
    }
}

fn list_item_view<'a>(
    body: Element<'a, Message>,
    level: u8,
    marker: String,
    color: Color,
) -> Element<'a, Message> {
    let contract = markdown_visual_contract();
    row![
        Space::new().width(f32::from(level) * contract.list_nested_indent),
        text(marker).size(MESSAGE_TEXT_SIZE).font(chat_text_font()).color(color),
        container(body).width(Length::Fill),
    ]
    .spacing(6)
    .into()
}

fn quote_bar_style(theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Background::Color(theme.palette().text.scale_alpha(0.28))),
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 2.0.into() },
        ..Default::default()
    }
}

/// 保留相邻普通块的连续选区，代码块、表格和链接块使用独立视图。
pub(crate) fn markdown_body_view<'a>(
    app: &'a App,
    msg_idx: usize,
    text_idx: usize,
    blocks: &'a [MarkdownBlock],
) -> Element<'a, Message> {
    let mut body = column![].spacing(markdown_visual_contract().block_spacing).width(Length::Fill);
    let mut block_count = 0;
    for (section_idx, section) in markdown_sections(blocks).enumerate() {
        block_count += 1;
        let block = &section[0];
        let key = markdown_block_key(msg_idx, text_idx, section_idx);
        if !block.joins_prose() || app.chat_markdown_block_editors.contains_key(&key) {
            body = body.push(markdown_block_view(app, msg_idx, text_idx, section_idx, block));
        } else {
            let body_color = message_body_text_color(&app.theme(), false);
            body = body.push(
                text(markdown_editor_text(section))
                    .size(MESSAGE_TEXT_SIZE)
                    .font(chat_text_font())
                    .line_height(iced::widget::text::LineHeight::Relative(
                        markdown_visual_contract().line_height,
                    ))
                    .style(move |_theme: &Theme| iced::widget::text::Style {
                        color: Some(body_color),
                    }),
            );
        }
    }
    Element::new(MarkdownSelectionArea { content: body.into(), msg_idx, text_idx, block_count })
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
