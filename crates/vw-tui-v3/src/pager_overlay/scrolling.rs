//! Scroll calculations and visible-row rendering shared by the transcript pager.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Paragraph, Widget, Wrap};

pub(super) fn max_scroll_offset(content_height: usize, viewport_height: usize) -> usize {
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
    Paragraph::new(Text::from(lines.to_vec()))
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset.try_into().unwrap_or(u16::MAX), 0))
        .render(area, buffer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_offset_is_clamped_for_resize() {
        assert_eq!(max_scroll_offset(20, 8), 12);
        assert_eq!(clamp_offset(17, 20, 8), 12);
        assert_eq!(clamp_offset(3, 20, 8), 3);
        assert_eq!(clamp_offset(3, 2, 8), 0);
    }
}
