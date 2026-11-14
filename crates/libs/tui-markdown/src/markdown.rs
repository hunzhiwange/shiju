//! Markdown 到 Ratatui 的渲染入口。
//!
//! `append_markdown` 服务于已预处理的计划与历史内容；`append_markdown_agent` 则先解包特定表格围栏。
//! 代理常把表格包进 `md` 或 `markdown` 代码围栏，若不解包就会被误渲染为等宽代码。
//! 解包器会缓存完整围栏正文，只在语言匹配且存在表头/分隔行对时处理，未闭合围栏安全退化为代码。
//! 本模块还负责把规范化后的增量块偏移映射回原始源码，无法安全映射时保持该块可变。
//! Markdown-to-ratatui rendering entry points.
//!
//! This module provides the public API surface that the rest of the TUI uses
//! to turn markdown source into `Vec<Line<'static>>`.  Two variants exist:
//!
//! - [`append_markdown`] -- general-purpose, used for plan blocks and history
//!   cells that already hold pre-processed markdown (no fence unwrapping).
//! - [`append_markdown_agent`] -- for agent responses.  Runs
//!   [`unwrap_markdown_fences`] first so that `` ```md ``/`` ```markdown ``
//!   fences containing tables are stripped and `pulldown-cmark` sees raw
//!   table syntax instead of fenced code.
//!
//! ## Why fence unwrapping exists
//!
//! LLM agents frequently wrap tables in `` ```markdown `` fences, treating
//! them as code.  Without unwrapping, `pulldown-cmark` parses those lines
//! as a fenced code block and renders them as monospace code rather than a
//! structured table.  The unwrapper is intentionally conservative: it
//! buffers the entire fence body before deciding, only unwraps fences whose
//! info string is `md` or `markdown` AND whose body contains a
//! header+delimiter pair, and degrades gracefully on unclosed fences.
use pulldown_cmark::{Event, Tag, TagEnd};
use std::ops::Range;
use ratatui::text::Line;
use std::borrow::Cow;
use std::path::Path;

use crate::table_detect;
use tui_render::terminal_hyperlinks::HyperlinkLine;

/// 把 Markdown 源码渲染为带样式 Ratatui 行并追加到 `lines`。
///
/// 已知会话工作目录的调用者应传入它，使流式与非流式渲染在进程目录不同的情况下仍展示相同相对路径。
/// Render markdown source to styled ratatui lines and append them to `lines`.
///
/// Callers that already know the session working directory should pass it here so streamed and
/// non-streamed rendering show the same relative path text even if the process cwd differs.
pub fn append_markdown(
    markdown_source: &str,
    width: Option<usize>,
    cwd: Option<&Path>,
    lines: &mut Vec<Line<'static>>,
) {
    let rendered = crate::markdown_render::render_markdown_text_with_width_and_cwd(
        markdown_source,
        width,
        cwd,
    );
    tui_render::line_utils::push_owned_lines(&rendered.lines, lines);
}

/// 把助手消息渲染为带样式 Ratatui 行。
///
/// 渲染前先调用 `unwrap_markdown_fences`，让 `md` 围栏中的表格按原生表格而非代码块呈现。
/// Rust、shell 等非 Markdown 围栏保持原样。
/// Render an agent message to styled ratatui lines.
///
/// Before rendering, the source is passed through [`unwrap_markdown_fences`] so that tables
/// wrapped in `` ```md `` fences are rendered as native tables rather than code blocks.
/// Non-markdown fences (e.g. `rust`, `sh`) are left
/// intact.
pub fn append_markdown_agent(
    markdown_source: &str,
    width: Option<usize>,
    lines: &mut Vec<Line<'static>>,
) {
    let normalized = unwrap_markdown_fences(markdown_source);
    let rendered = crate::markdown_render::render_markdown_text_with_width_and_cwd(
        &normalized,
        width,
        /*cwd*/ None,
    );
    tui_render::line_utils::push_owned_lines(&rendered.lines, lines);
}

pub fn render_markdown_agent_with_links_and_cwd(
    markdown_source: &str,
    width: Option<usize>,
    cwd: Option<&Path>,
) -> Vec<HyperlinkLine> {
    let rewritten = rewrite_inline_visualizations_without_context(markdown_source);
    let normalized = unwrap_markdown_fences(&rewritten);
    crate::markdown_render::render_markdown_lines_with_width_and_cwd(&normalized, width, cwd)
}

/// 保留复制版渲染器的无上下文行为，不激活后续批次的协议驱动可视化物化器。
/// Preserve the copied renderer's no-context behavior without activating the
/// protocol-backed visualization materializer from batch 6.
fn rewrite_inline_visualizations_without_context(markdown: &str) -> Cow<'_, str> {
    const DIRECTIVE_PREFIX: &str = "::codex-inline-vis{";
    const CONTENT_REFERENCE_PREFIX: &str = "\u{e200}visualize\u{e202}";
    const CONTENT_REFERENCE_SUFFIX: char = '\u{e201}';

    if !markdown.contains(DIRECTIVE_PREFIX) && !markdown.contains(CONTENT_REFERENCE_PREFIX) {
        return Cow::Borrowed(markdown);
    }

    let mut code_block_ranges = Vec::<Range<usize>>::new();
    let mut code_block_start = None;
    for (event, range) in pulldown_cmark::Parser::new(markdown).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_block_start = Some(range.start),
            Event::End(TagEnd::CodeBlock) => {
                if let Some(start) = code_block_start.take() {
                    code_block_ranges.push(start..range.end);
                }
            }
            _ => {}
        }
    }
    if let Some(start) = code_block_start {
        code_block_ranges.push(start..markdown.len());
    }

    let mut rewritten = String::with_capacity(markdown.len());
    let mut source_offset = 0;
    for source_line in markdown.split_inclusive('\n') {
        let line_start = source_offset;
        source_offset += source_line.len();
        let (line, newline) =
            source_line.strip_suffix('\n').map_or((source_line, ""), |line| (line, "\n"));
        let trimmed = line.trim();
        let is_code = code_block_ranges
            .iter()
            .any(|range| range.start < source_offset && line_start < range.end);
        if is_code
            || (!trimmed.starts_with(DIRECTIVE_PREFIX)
                && !trimmed.starts_with(CONTENT_REFERENCE_PREFIX))
        {
            rewritten.push_str(line);
        } else if trimmed.ends_with(CONTENT_REFERENCE_SUFFIX)
            || (trimmed.starts_with(DIRECTIVE_PREFIX) && trimmed.ends_with('}'))
        {
            rewritten.push_str("_Visualization unavailable on this device._");
        }
        rewritten.push_str(newline);
    }
    Cow::Owned(rewritten)
}

/// 渲染助手消息，并收集增量渲染所需的块元数据。
///
/// Markdown 表格围栏解包后，块偏移会映射回原始源码；若规范化边界无法表示为原始源码后缀，
/// 则丢弃该边界，使转换后的块继续保持可变。
/// Render an agent message and collect the block metadata needed for incremental rendering.
///
/// Block offsets are mapped back to `markdown_source` after Markdown table fences are unwrapped.
/// If a normalized boundary cannot be expressed as a raw-source suffix, it is discarded so the
/// transformed block remains mutable.
pub fn render_streaming_markdown_agent_with_links_and_cwd(
    markdown_source: &str,
    width: Option<usize>,
    cwd: Option<&Path>,
) -> crate::markdown_render::StreamingMarkdownRender {
    let normalized = unwrap_markdown_fences(markdown_source);
    let mut rendered = crate::markdown_render::render_streaming_markdown_lines_with_width_and_cwd(
        &normalized,
        width,
        cwd,
    );
    if normalized != markdown_source {
        // 围栏解包会移除起止行；仍是原始源码后缀的规范化尾部必然从这些移除行之后开始，
        // 因而可以安全映射回原始偏移，否则继续保持转换块可变。
        // Fence unwrapping removes opening/closing lines. A normalized tail that is still a raw
        // suffix necessarily begins after those removed lines, so its boundary can safely be
        // mapped back to the raw source; otherwise leave the transformed block mutable.
        rendered.last_top_level_block_start = rendered
            .last_top_level_block_start
            .and_then(|boundary| markdown_source.strip_suffix(&normalized[boundary..]))
            .map(str::len);
    }
    rendered
}

/// 去除包含表格的 `md` 或 `markdown` 围栏，并把正文作为裸 Markdown 发出供原生表格解析。
///
/// 其他语言围栏保持不变；没有表头与分隔行对的 Markdown 围栏也继续按代码块渲染。
/// 解包有意保持保守：先缓存完整正文再决定，输入结束时未闭合的围栏会连同起始行重新发出。
/// Strip `` ```md ``/`` ```markdown `` fences that contain tables, emitting their content as bare
/// markdown so `pulldown-cmark` parses the tables natively.
///
/// Fences whose info string is not `md` or `markdown` are passed through unchanged.  Markdown
/// fences that do *not* contain a table (detected by checking for a header row + delimiter row)
/// are also passed through so that non-table markdown inside a fence still renders as a code
/// block.
///
/// The fence unwrapping is intentionally conservative: it buffers the entire fence body before
/// deciding, and an unclosed fence at end-of-input is re-emitted with its opening line so partial
/// streams degrade to code display.
fn unwrap_markdown_fences<'a>(markdown_source: &'a str) -> Cow<'a, str> {
    // 零复制快速路径：绝大多数消息根本不包含围栏。
    // Zero-copy fast path: most messages contain no fences at all.
    if !markdown_source.contains("```") && !markdown_source.contains("~~~") {
        return Cow::Borrowed(markdown_source);
    }

    #[derive(Clone, Copy)]
    struct Fence {
        marker: u8,
        len: usize,
        is_blockquoted: bool,
    }

    // 去掉尾随换行及最多三个前导空格；四个及以上前导空格按 CommonMark 属于缩进代码行。
    // Strip a trailing newline and up to 3 leading spaces, returning the
    // trimmed slice.  Returns `None` when the line has 4+ leading spaces
    // (which makes it an indented code line per CommonMark).
    fn strip_line_indent(line: &str) -> Option<&str> {
        let without_newline = line.strip_suffix('\n').unwrap_or(line);
        let mut byte_idx = 0usize;
        let mut column = 0usize;
        for b in without_newline.as_bytes() {
            match b {
                b' ' => {
                    byte_idx += 1;
                    column += 1;
                }
                b'\t' => {
                    byte_idx += 1;
                    column += 4;
                }
                _ => break,
            }
            if column >= 4 {
                return None;
            }
        }
        Some(&without_newline[byte_idx..])
    }

    // 解析起始围栏行，返回围栏元数据以及信息串是否表示 Markdown 正文。
    // Parse an opening fence line, returning the fence metadata and whether
    // the fence info string indicates markdown content.
    fn parse_open_fence(line: &str) -> Option<(Fence, bool)> {
        let trimmed = strip_line_indent(line)?;
        let is_blockquoted = trimmed.trim_start().starts_with('>');
        let fence_scan_text = table_detect::strip_blockquote_prefix(trimmed);
        let (marker, len) = table_detect::parse_fence_marker(fence_scan_text)?;
        let is_markdown = table_detect::is_markdown_fence_info(fence_scan_text, len);
        Some((Fence { marker: marker as u8, len, is_blockquoted }, is_markdown))
    }

    fn is_close_fence(line: &str, fence: Fence) -> bool {
        let Some(trimmed) = strip_line_indent(line) else {
            return false;
        };
        let fence_scan_text = if fence.is_blockquoted {
            if !trimmed.trim_start().starts_with('>') {
                return false;
            }
            table_detect::strip_blockquote_prefix(trimmed)
        } else {
            trimmed
        };
        if let Some((marker, len)) = table_detect::parse_fence_marker(fence_scan_text) {
            marker as u8 == fence.marker
                && len >= fence.len
                && fence_scan_text[len..].trim().is_empty()
        } else {
            false
        }
    }

    fn markdown_fence_contains_table(content: &str, is_blockquoted_fence: bool) -> bool {
        let mut previous_line: Option<&str> = None;
        for line in content.lines() {
            let text = if is_blockquoted_fence {
                table_detect::strip_blockquote_prefix(line)
            } else {
                line
            };
            let trimmed = text.trim();
            if trimmed.is_empty() {
                previous_line = None;
                continue;
            }

            if let Some(previous) = previous_line
                && table_detect::is_table_header_line(previous)
                && !table_detect::is_table_delimiter_line(previous)
                && table_detect::is_table_delimiter_line(trimmed)
            {
                return true;
            }

            previous_line = Some(trimmed);
        }
        false
    }

    fn content_from_ranges(source: &str, ranges: &[Range<usize>]) -> String {
        let total_len: usize = ranges.iter().map(ExactSizeIterator::len).sum();
        let mut content = String::with_capacity(total_len);
        for range in ranges {
            content.push_str(&source[range.start..range.end]);
        }
        content
    }

    struct MarkdownCandidateData {
        fence: Fence,
        opening_range: Range<usize>,
        content_ranges: Vec<Range<usize>>,
    }

    // 装箱大型变体，使 `ActiveFence` 保持接近指针大小。
    // Box the large variant to keep ActiveFence small (~pointer-sized).
    enum ActiveFence {
        Passthrough(Fence),
        MarkdownCandidate(Box<MarkdownCandidateData>),
    }

    let mut out = String::with_capacity(markdown_source.len());
    let mut active_fence: Option<ActiveFence> = None;
    let mut source_offset = 0usize;

    let mut push_source_range = |range: Range<usize>| {
        if !range.is_empty() {
            out.push_str(&markdown_source[range]);
        }
    };

    for line in markdown_source.split_inclusive('\n') {
        let line_start = source_offset;
        source_offset += line.len();
        let line_range = line_start..source_offset;

        if let Some(active) = active_fence.take() {
            match active {
                ActiveFence::Passthrough(fence) => {
                    push_source_range(line_range);
                    if !is_close_fence(line, fence) {
                        active_fence = Some(ActiveFence::Passthrough(fence));
                    }
                }
                ActiveFence::MarkdownCandidate(mut data) => {
                    if is_close_fence(line, data.fence) {
                        if markdown_fence_contains_table(
                            &content_from_ranges(markdown_source, &data.content_ranges),
                            data.fence.is_blockquoted,
                        ) {
                            for range in data.content_ranges {
                                push_source_range(range);
                            }
                        } else {
                            push_source_range(data.opening_range);
                            for range in data.content_ranges {
                                push_source_range(range);
                            }
                            push_source_range(line_range);
                        }
                    } else {
                        data.content_ranges.push(line_range);
                        active_fence = Some(ActiveFence::MarkdownCandidate(data));
                    }
                }
            }
            continue;
        }

        if let Some((fence, is_markdown)) = parse_open_fence(line) {
            if is_markdown {
                active_fence =
                    Some(ActiveFence::MarkdownCandidate(Box::new(MarkdownCandidateData {
                        fence,
                        opening_range: line_range,
                        content_ranges: Vec::new(),
                    })));
            } else {
                push_source_range(line_range);
                active_fence = Some(ActiveFence::Passthrough(fence));
            }
            continue;
        }

        push_source_range(line_range);
    }

    if let Some(active) = active_fence {
        match active {
            ActiveFence::Passthrough(_) => {}
            ActiveFence::MarkdownCandidate(data) => {
                push_source_range(data.opening_range);
                for range in data.content_ranges {
                    push_source_range(range);
                }
            }
        }
    }

    Cow::Owned(out)
}

#[cfg(test)]
#[path = "markdown_tests.rs"]
mod tests;
