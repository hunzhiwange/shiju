//! Chat-surface adapter for the copied working indicator.

use crate::render::renderable::Renderable;
use crate::status_indicator_widget::StatusIndicatorWidget;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::V3ChatStatus;

pub(super) struct StatusSurface<'a> {
    indicator: StatusIndicatorWidget<'a>,
}

impl<'a> StatusSurface<'a> {
    pub(super) fn new(status: &'a V3ChatStatus) -> Self {
        Self { indicator: StatusIndicatorWidget::new(status) }
    }

    pub(super) fn is_visible(&self) -> bool {
        self.indicator.is_visible()
    }
}

impl Renderable for StatusSurface<'_> {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.indicator.render(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.indicator.desired_height(width)
    }
}
