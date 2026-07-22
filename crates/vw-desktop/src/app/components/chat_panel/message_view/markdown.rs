//! 助手消息正文的轻量 Markdown 渲染。
//!
//! iced 自带的 markdown widget 基于 rich_text，无法划选复制。为兼顾块级格式与
//! 复制能力，这里自实现一个简易块级解析：把正文切成标题/段落/列表/代码块/引用，
//! 每个块用只读 `text_editor` 渲染（原生支持划选复制），行内 `**`、`` ` ``、链接
//! 等标记被清理为纯文本，仅保留块级排版。

use iced::widget::{Space, column, container, row, text, text_editor};
use iced::{Background, Border, Color, Element, Length, Theme};

use crate::app::{App, Message, message};

use super::super::tool_text_support::{
    chat_text_font, chat_text_line_height, read_only_text_style,
};
use super::styles::{MESSAGE_TEXT_SIZE, is_dark_theme, message_body_text_color};

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
    match kind {
        MarkdownBlockKind::Heading(1) => MESSAGE_TEXT_SIZE + 8.0,
        MarkdownBlockKind::Heading(2) => MESSAGE_TEXT_SIZE + 5.0,
        MarkdownBlockKind::Heading(3) => MESSAGE_TEXT_SIZE + 3.0,
        MarkdownBlockKind::Heading(_) => MESSAGE_TEXT_SIZE + 1.0,
        MarkdownBlockKind::CodeBlock(_) => MESSAGE_TEXT_SIZE - 1.0,
        _ => MESSAGE_TEXT_SIZE,
    }
}

/// 渲染单个 Markdown 块为可划选元素。
fn markdown_block_view<'a>(
    app: &'a App,
    msg_idx: usize,
    text_idx: usize,
    block_idx: usize,
    block: &'a MarkdownBlock,
) -> Element<'a, Message> {
    let theme = app.theme();
    let key = markdown_block_key(msg_idx, text_idx, block_idx);
    let size = block_text_size(&block.kind);
    let is_code = matches!(block.kind, MarkdownBlockKind::CodeBlock(_));
    let font = if is_code { iced::Font::MONOSPACE } else { chat_text_font() };
    let body_color = message_body_text_color(&theme, false);

    let editor: Element<'a, Message> =
        if let Some(content) = app.chat_markdown_block_editors.get(&key) {
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
                .font(font)
                .style(move |theme: &Theme, _status| read_only_text_style(theme, body_color))
                .into()
        } else {
            // 编辑器缺失时回退为静态文本，仍可被气泡复制按钮覆盖。
            text(block.text.clone())
                .size(size)
                .font(font)
                .line_height(chat_text_line_height())
                .style(move |_theme: &Theme| iced::widget::text::Style { color: Some(body_color) })
                .into()
        };

    match &block.kind {
        MarkdownBlockKind::CodeBlock(_) => container(editor)
            .padding([8, 10])
            .width(Length::Fill)
            .style(move |theme: &Theme| code_block_style(theme))
            .into(),
        MarkdownBlockKind::Quote => row![
            container(Space::new())
                .width(Length::Fixed(3.0))
                .height(Length::Shrink)
                .style(move |theme: &Theme| quote_bar_style(theme)),
            container(editor)
                .padding(iced::Padding { top: 0.0, right: 0.0, bottom: 0.0, left: 10.0 })
                .width(Length::Fill),
        ]
        .into(),
        MarkdownBlockKind::BulletItem(level) => {
            let indent = 14.0 + f32::from(*level) * 14.0;
            row![
                container(text("•").size(size).style(move |_theme: &Theme| {
                    iced::widget::text::Style { color: Some(body_color) }
                }))
                .padding(iced::Padding { top: 0.0, right: 6.0, bottom: 0.0, left: 0.0 })
                .width(Length::Fixed(indent)),
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
    let is_dark = is_dark_theme(theme);
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba8(255, 255, 255, 0.06)
        } else {
            Color::from_rgba8(15, 23, 42, 0.05)
        })),
        border: Border {
            width: 1.0,
            color: if is_dark {
                Color::from_rgba8(255, 255, 255, 0.10)
            } else {
                Color::from_rgba8(15, 23, 42, 0.10)
            },
            radius: 8.0.into(),
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

/// 渲染整段助手正文的 Markdown 块为可划选列表。
pub(crate) fn markdown_body_view<'a>(
    app: &'a App,
    msg_idx: usize,
    text_idx: usize,
    blocks: &'a [MarkdownBlock],
) -> Element<'a, Message> {
    let mut body = column![].spacing(8).width(Length::Fill);
    for (block_idx, block) in blocks.iter().enumerate() {
        body = body.push(markdown_block_view(app, msg_idx, text_idx, block_idx, block));
    }
    body.into()
}
