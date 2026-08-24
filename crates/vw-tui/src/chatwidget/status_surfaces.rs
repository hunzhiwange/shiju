//! Chat-surface adapter for the copied working indicator.
//! 本模块负责工作状态到聊天界面的展示适配。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use crate::render::renderable::Renderable;
use crate::status_indicator_widget::StatusIndicatorWidget;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::time::Duration;

use super::V3ChatStatus;

pub(super) struct StatusSurface<'a> {
    indicator: StatusIndicatorWidget<'a>,
}

impl<'a> StatusSurface<'a> {
    pub(super) fn new(status: &'a V3ChatStatus, elapsed: Duration) -> Self {
        Self { indicator: StatusIndicatorWidget::new(status, elapsed) }
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
