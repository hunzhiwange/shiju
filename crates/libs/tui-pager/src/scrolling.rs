//! Scroll calculations and visible-row rendering shared by the transcript pager.
//! 转录分页器共享的滚动计算与可见行渲染。

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Paragraph, Widget, Wrap};

pub(super) fn max_scroll_offset(content_height: usize, viewport_height: usize) -> usize {
    // 饱和减法兼容内容短于视口的情况，并把零高视口按一行处理。
    content_height.saturating_sub(viewport_height.max(1))
}

pub(super) fn clamp_offset(offset: usize, content_height: usize, viewport_height: usize) -> usize {
    offset.min(max_scroll_offset(content_height, viewport_height))
}

pub(super) fn render_scrolled_lines(
    lines: &[Line<'static>],
    scroll_offset: usize,
    area: Rect,
    buffer: &mut Buffer,
) {
    // Ratatui 偏移为 u16；超大 usize 偏移饱和到最大值，避免转换失败导致回卷。
    Paragraph::new(Text::from(lines.to_vec()))
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset.try_into().unwrap_or(u16::MAX), 0))
        .render(area, buffer);
}

#[cfg(test)]
#[path = "scrolling_tests.rs"]
mod tests;
