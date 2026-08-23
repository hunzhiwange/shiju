//! Copied one-line working indicator over read-only v3 activity state.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;

use crate::V3ChatStatus;
use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::render::renderable::Renderable;
use crate::wrapping::RtOptions;
use crate::wrapping::word_wrap_lines;

const DETAILS_PREFIX: &str = "  └ ";

/// Displays the copied `• Working` surface without owning runtime actions.
pub(crate) struct StatusIndicatorWidget<'a> {
    status: &'a V3ChatStatus,
    elapsed_seconds: u64,
}

impl<'a> StatusIndicatorWidget<'a> {
    pub(crate) fn new(status: &'a V3ChatStatus) -> Self {
        Self { status, elapsed_seconds: 0 }
    }

    pub(crate) fn is_visible(&self) -> bool {
        self.status.is_task_running && self.status.label.is_some()
    }

    fn wrapped_details_lines(&self, width: u16) -> Vec<Line<'static>> {
        let Some(details) = self.status.detail.as_deref().filter(|detail| !detail.is_empty()) else {
            return Vec::new();
        };
        if width == 0 {
            return Vec::new();
        }
        let opts = RtOptions::new(usize::from(width))
            .initial_indent(Line::from(DETAILS_PREFIX.dim()))
            .subsequent_indent(Line::from("    ".dim()))
            .break_words(true);
        word_wrap_lines(details.lines().map(|line| vec![line.dim()]), opts)
    }

    fn header_line(&self, width: u16) -> Line<'static> {
        let label = self.status.label.as_deref().unwrap_or("Working");
        truncate_line_with_ellipsis_if_overflow(
            Line::from(vec![
                "• ".cyan(),
                label.to_string().bold(),
                format!(" ({}s • ", self.elapsed_seconds).dim(),
                "esc".into(),
                " to interrupt)".dim(),
            ]),
            usize::from(width),
        )
    }
}

impl Renderable for StatusIndicatorWidget<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.is_visible() || area.is_empty() {
            return;
        }
        let mut lines = vec![self.header_line(area.width)];
        lines.extend(
            self.wrapped_details_lines(area.width)
                .into_iter()
                .take(usize::from(area.height.saturating_sub(1))),
        );
        Paragraph::new(lines).wrap(Wrap { trim: false }).render(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        if !self.is_visible() {
            return 0;
        }
        1 + u16::try_from(self.wrapped_details_lines(width).len()).unwrap_or(u16::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_indicator_keeps_working_and_narrow_layout() {
        let status = V3ChatStatus::running("Working", Some("reading copied components".to_string()));
        let widget = StatusIndicatorWidget::new(&status);
        let wide_area = Rect::new(0, 0, 80, 2);
        let mut wide = Buffer::empty(wide_area);
        widget.render(wide_area, &mut wide);
        let narrow_area = Rect::new(0, 0, 24, 3);
        let mut narrow = Buffer::empty(narrow_area);
        widget.render(narrow_area, &mut narrow);

        insta::assert_snapshot!("status_indicator_vibe_wide", format!("{wide:?}"));
        insta::assert_snapshot!("status_indicator_vibe_narrow", format!("{narrow:?}"));
    }
}
