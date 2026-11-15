//! Copied one-line working indicator over read-only v3 activity state.
//! 在只读 v3 活动状态之上复用单行工作指示器。
//!
//! 控件只解释展示快照，不拥有取消任务或更新运行状态的能力。

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;

use tui_contracts::presentation::V3ChatStatus;
use tui_text::line_truncation::truncate_line_with_ellipsis_if_overflow;
use tui_render::renderable::Renderable;
use tui_text::wrapping::RtOptions;
use tui_text::wrapping::word_wrap_lines;

const DETAILS_PREFIX: &str = "  └ ";
const WORKING_SPINNER_FRAME_MS: u128 = 90;
const WORKING_SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn working_spinner_frame(elapsed: Duration) -> &'static str {
    let frame = elapsed.as_millis() / WORKING_SPINNER_FRAME_MS;
    WORKING_SPINNER_FRAMES[(frame % WORKING_SPINNER_FRAMES.len() as u128) as usize]
}

/// Displays the copied `• ⠋ Working` surface without owning runtime actions.
/// 展示复用的 `• ⠋ Working` 界面，但不拥有任何运行时动作。
pub(crate) struct StatusIndicatorWidget<'a> {
    status: &'a V3ChatStatus,
    elapsed: Duration,
}

impl<'a> StatusIndicatorWidget<'a> {
    /// 从借用的聊天状态与当前轮次耗时构造指示器。
    pub(crate) fn new(status: &'a V3ChatStatus, elapsed: Duration) -> Self {
        Self { status, elapsed }
    }

    /// 仅在任务运行且有标签时占用布局空间。
    pub(crate) fn is_visible(&self) -> bool {
        self.status.is_task_running && self.status.label.is_some()
    }

    fn wrapped_details_lines(&self, width: u16) -> Vec<Line<'static>> {
        // 详情首行带树形前缀，续行与详情文本对齐。
        let Some(details) = self.status.detail.as_deref().filter(|detail| !detail.is_empty())
        else {
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
        // 标题在可用宽度内整体截断，避免覆盖相邻区域。
        let label = self.status.label.as_deref().unwrap_or("Working");
        let spinner = working_spinner_frame(self.elapsed);
        truncate_line_with_ellipsis_if_overflow(
            Line::from(vec![
                format!("• {spinner} ").cyan(),
                label.to_string().bold(),
                format!(" ({}s • ", self.elapsed.as_secs()).dim(),
                "esc".into(),
                " to interrupt)".dim(),
            ]),
            usize::from(width),
        )
    }
}

impl Renderable for StatusIndicatorWidget<'_> {
    // 渲染高度有限时优先保留标题，再截取可容纳的详情行。
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
#[path = "status_indicator_widget_tests.rs"]
mod tests;
