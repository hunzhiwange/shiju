//! 进行中转录流的增量 Markdown 渲染。
//!
//! 已完成顶层块会被保留，只有最后一块保持可变，从而避免增量到达时重复渲染稳定前缀。
//! 引用定义和内联可视化可能影响全局，因此遇到它们时安全回退到完整渲染。
//! Incremental markdown rendering for in-flight transcript streams.
//!
//! Completed top-level blocks are retained while the final block stays mutable, avoiding repeated
//! rendering of the stable prefix as newline-bearing deltas arrive.

use super::StreamRenderMode;
use super::code_fence::OpenCodeFence;
use super::raw_lines_from_source;
use crate::markdown::render_markdown_agent_with_links_and_cwd;
use crate::markdown::render_streaming_markdown_agent_with_links_and_cwd;
use crate::render::highlight::syntax_theme_revision;
use crate::terminal_hyperlinks::HyperlinkLine;
use crate::terminal_hyperlinks::plain_hyperlink_lines;
use ratatui::text::Line;
use std::path::Path;

fn contains_inline_visualization(markdown: &str) -> bool {
    markdown.contains("::codex-inline-vis{") || markdown.contains('\u{e200}')
}

/// 在源码边界与渲染行边界处分割的增量渲染状态。
///
/// 两个边界之前的前缀不可变，只有最后一个顶层 Markdown 块随已提交源码重新渲染。
/// Incremental render state split at source and rendered-line boundaries.
///
/// The prefix before both boundaries is immutable; only the final top-level Markdown block is
/// re-rendered as committed source arrives.
pub(super) struct StreamingRender {
    pub(super) lines: Vec<HyperlinkLine>,
    /// 仅包含已完成顶层 Markdown 块的源码前缀。
    /// Source prefix containing only completed top-level markdown blocks.
    stable_source_len: usize,
    /// 与 `stable_source_len` 对应的渲染行边界。
    /// Rendered-line boundary corresponding to `stable_source_len`.
    stable_rendered_len: usize,
    /// 引用式链接定义可能影响任意更早或更晚的 Markdown 块。
    /// Reference-style link definitions can affect any earlier or later markdown block.
    has_reference_link_definition: bool,
    /// 内联可视化指令一旦提交就要求重写完整源码。
    /// Inline visualization directives require source-wide rewriting once one is committed.
    has_inline_visualization_directive: bool,
    /// 可直接追加的开放顶层代码围栏解析状态。
    /// Parser state for a directly appendable, open top-level code fence.
    open_code_fence: Option<OpenCodeFence>,
}

impl StreamingRender {
    pub(super) fn new() -> Self {
        Self {
            lines: Vec::with_capacity(64),
            stable_source_len: 0,
            stable_rendered_len: 0,
            has_reference_link_definition: false,
            has_inline_visualization_directive: false,
            open_code_fence: None,
        }
    }

    pub(super) fn clear(&mut self) {
        self.lines.clear();
        self.stable_source_len = 0;
        self.stable_rendered_len = 0;
        self.has_reference_link_definition = false;
        self.has_inline_visualization_directive = false;
        self.open_code_fence = None;
    }

    /// 重新渲染完整源码，并重置两个稳定前缀边界。
    ///
    /// 宽度或渲染模式变化，以及全局渲染状态使前缀复用不安全时使用。
    /// Re-render the full source and reset both stable-prefix boundaries.
    ///
    /// This is used when width or render mode changes, and whenever source-wide rendering state
    /// makes retaining previously rendered blocks unsafe.
    pub(super) fn recompute(
        &mut self,
        source: &str,
        width: Option<usize>,
        cwd: &Path,
        render_mode: StreamRenderMode,
    ) {
        self.open_code_fence = None;
        self.has_inline_visualization_directive = contains_inline_visualization(source);
        self.lines = match render_mode {
            StreamRenderMode::Rich if !self.has_inline_visualization_directive => {
                let rendered =
                    render_streaming_markdown_agent_with_links_and_cwd(source, width, Some(cwd));
                self.has_reference_link_definition = rendered.has_reference_link_definition;
                rendered.lines
            }
            _ => {
                self.has_reference_link_definition = false;
                render_source(source, width, cwd, render_mode)
            }
        };
        self.stable_source_len = 0;
        self.stable_rendered_len = 0;
    }

    /// 追加新提交源码，同时只让最终 Markdown 块保持可变。
    ///
    /// 最终块可能因后续行改变含义；更早顶层块只渲染一次并保留。
    /// 引用定义与内联可视化会影响全局，因此回退到完整渲染。
    /// Append newly committed source while retaining only the final markdown block as mutable.
    ///
    /// The final top-level block can still change meaning when another line arrives (for example,
    /// list tightness, a setext heading, a fenced block, or a table). Earlier top-level blocks are
    /// rendered once and retained. Reference-style link definitions and inline visualization
    /// rewriting fall back to a full render because they can affect source-wide rendering state.
    pub(super) fn append(
        &mut self,
        raw_source: &str,
        committed_source: &str,
        width: Option<usize>,
        cwd: &Path,
        render_mode: StreamRenderMode,
    ) {
        if render_mode == StreamRenderMode::Raw {
            self.lines.extend(plain_hyperlink_lines(raw_lines_from_source(committed_source)));
            return;
        }

        self.has_inline_visualization_directive |= contains_inline_visualization(committed_source);
        if self.has_inline_visualization_directive {
            self.recompute(raw_source, width, cwd, render_mode);
            return;
        }

        if self.has_reference_link_definition {
            self.recompute(raw_source, width, cwd, render_mode);
            return;
        }

        if let Some(fence) = self.open_code_fence.take()
            && let Some((fence, lines)) = fence.append(raw_source, committed_source)
        {
            self.lines.extend(lines);
            self.open_code_fence = Some(fence);
            return;
        }

        let pending_source = &raw_source[self.stable_source_len..];
        let theme_revision = syntax_theme_revision();
        let pending =
            render_streaming_markdown_agent_with_links_and_cwd(pending_source, width, Some(cwd));
        if pending.has_reference_link_definition {
            self.has_reference_link_definition = true;
            self.recompute(raw_source, width, cwd, render_mode);
            return;
        }

        let final_block_start = pending.last_top_level_block_start.unwrap_or(/*default*/ 0);
        self.open_code_fence = OpenCodeFence::detect(
            &pending_source[final_block_start..],
            raw_source.len(),
            theme_revision,
        );

        let mut newly_stable_rendered_len = None;
        if let Some(boundary) = pending.last_top_level_block_start {
            let newly_stable_source = &pending_source[..boundary];
            let newly_stable = render_source(newly_stable_source, width, cwd, render_mode);
            self.stable_source_len += boundary;
            newly_stable_rendered_len = Some(newly_stable.len());
        }

        self.lines.truncate(self.stable_rendered_len);
        if !self.lines.is_empty()
            && (!pending.lines.is_empty() || !pending_source.trim().is_empty())
            && !pending.first_top_level_block_is_html
        {
            self.lines.push(HyperlinkLine::new(Line::default()));
        }
        let pending_render_start = self.lines.len();
        self.lines.extend(pending.lines);
        if let Some(newly_stable_rendered_len) = newly_stable_rendered_len {
            self.stable_rendered_len = pending_render_start + newly_stable_rendered_len;
        }
    }
}

pub(super) fn render_source(
    source: &str,
    width: Option<usize>,
    cwd: &Path,
    render_mode: StreamRenderMode,
) -> Vec<HyperlinkLine> {
    match render_mode {
        StreamRenderMode::Rich => {
            render_markdown_agent_with_links_and_cwd(source, width, Some(cwd))
        }
        StreamRenderMode::Raw => plain_hyperlink_lines(raw_lines_from_source(source)),
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
