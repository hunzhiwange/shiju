//! 渲染并格式化统一执行背景会话摘要文本。 本模块提供一组标准摘要字符串，使底部面板既能渲染独立的脚本文档行，又能复用同一文本在状态行中直接嵌入，无需重复复制或语法逻辑。
//! `summary_text` 统一生成未缩进文本，具体分隔符与缩进由调用位置决定。
//! 宽度不足或没有后台进程时不绘制内容，避免占用无意义的页脚行。
//! Renders and formats unified-exec background session summary text.
//!
//! This module provides one canonical summary string so the bottom pane can
//! either render a dedicated footer row or reuse the same text inline in the
//! status row without duplicating copy/grammar logic.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

use crate::live_wrap::take_prefix_by_width;
use crate::render::renderable::Renderable;

/// 跟踪活跃的统一执行进程并生成紧凑摘要
/// Tracks active unified-exec processes and renders a compact summary.
pub(crate) struct UnifiedExecFooter {
    processes: Vec<String>,
}

impl UnifiedExecFooter {
    pub(crate) fn new() -> Self {
        Self {
            processes: Vec::new(),
        }
    }

    pub(crate) fn set_processes(&mut self, processes: Vec<String>) -> bool {
        if self.processes == processes {
            return false;
        }
        self.processes = processes;
        true
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.processes.is_empty()
    }

    /// 返回用于同时由脚本报表和状态行渲染使用的未缩进摘要文本。 返回的字符串故意省略首尾空格和分隔符，以便调用者可以选择布局特定的框架（内联分隔符 vs. 行缩进）。返回`None`表示没有内容需要呈现。
    /// Returns the unindented summary text used by both footer and status-row rendering.
    ///
    /// The returned string intentionally omits leading spaces and separators so
    /// callers can choose layout-specific framing (inline separator vs. row
    /// indentation). Returning `None` means there is nothing to surface.
    pub(crate) fn summary_text(&self) -> Option<String> {
        if self.processes.is_empty() {
            return None;
        }

        let count = self.processes.len();
        let plural = if count == 1 { "" } else { "s" };
        Some(format!(
            "{count} background terminal{plural} running · /ps to view · /stop to close"
        ))
    }

    fn render_lines(&self, width: u16) -> Vec<Line<'static>> {
        if width < 4 {
            return Vec::new();
        }
        let Some(summary) = self.summary_text() else {
            return Vec::new();
        };
        let message = format!("  {summary}");
        let (truncated, _, _) = take_prefix_by_width(&message, width as usize);
        vec![Line::from(truncated.dim())]
    }
}

impl Renderable for UnifiedExecFooter {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }

        Paragraph::new(self.render_lines(area.width)).render(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.render_lines(width).len() as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use pretty_assertions::assert_eq;

    // 验证 `desired_height_empty` 场景的触发条件与预期结果。
    #[test]
    fn desired_height_empty() {
        let footer = UnifiedExecFooter::new();
        assert_eq!(footer.desired_height(/*width*/ 40), 0);
    }

    // 验证 `render_more_sessions` 场景的触发条件与预期结果。
    #[test]
    fn render_more_sessions() {
        let mut footer = UnifiedExecFooter::new();
        footer.set_processes(vec!["rg \"foo\" src".to_string()]);
        let width = 50;
        let height = footer.desired_height(width);
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        footer.render(Rect::new(0, 0, width, height), &mut buf);
        assert_snapshot!("render_more_sessions", format!("{buf:?}"));
    }

    // 验证 `render_many_sessions` 场景的触发条件与预期结果。
    #[test]
    fn render_many_sessions() {
        let mut footer = UnifiedExecFooter::new();
        footer.set_processes((0..123).map(|idx| format!("cmd {idx}")).collect());
        let width = 50;
        let height = footer.desired_height(width);
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        footer.render(Rect::new(0, 0, width, height), &mut buf);
        assert_snapshot!("render_many_sessions", format!("{buf:?}"));
    }
}
