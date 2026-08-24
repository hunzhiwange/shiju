//! Render composition for the copied main chat surface.
//! 本模块负责主聊天界面的渲染组合。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use super::status_surfaces::StatusSurface;
use super::transcript::{ActiveCellLayoutCache, ActiveCellLayoutCacheKey};
use super::*;
use crate::render::Insets;
use crate::render::renderable::{FlexRenderable, Renderable, RenderableExt, RenderableItem};
use ratatui::text::{Line, Text};
use ratatui::widgets::{
    Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget, Wrap,
};

impl ChatWidget {
    // 计算 `as_renderable` 所需的界面结果。
    pub(crate) fn as_renderable(&self) -> RenderableItem<'_> {
        let mut flex = FlexRenderable::new();
        flex.push(
            1,
            RenderableItem::Owned(Box::new(TranscriptAreaRenderable {
                transcript: &self.transcript,
            })),
        );

        let status = StatusSurface::new(self.status_state.current(), self.turn_lifecycle.elapsed());
        if status.is_visible() {
            flex.push(0, RenderableItem::Owned(Box::new(status)));
        }

        let bottom_surface = self.overlay.as_ref().map_or_else(
            || RenderableItem::Borrowed(&self.bottom_pane as &dyn Renderable),
            |overlay| RenderableItem::Borrowed(overlay as &dyn Renderable),
        );
        flex.push(
            0,
            bottom_surface.inset(Insets::tlbr(
                /* top */ 1, /* left */ 0, /* bottom */ 0, /* right */ 0,
            )),
        );
        RenderableItem::Owned(Box::new(flex))
    }

    // 计算 `note_rendered_width` 在当前界面流程中需要的局部结果。
    fn note_rendered_width(&self, width: u16) {
        self.last_rendered_width.set(Some(width));
    }
}

/// 描述 `TranscriptAreaRenderable` 在当前界面流程中传递的数据。
struct TranscriptAreaRenderable<'a> {
    transcript: &'a TranscriptState,
}

impl TranscriptAreaRenderable<'_> {
    fn lines(&self, width: u16) -> Vec<Line<'static>> {
        let width = width.max(1);
        let mut lines = self.transcript.committed_lines(width, true);
        let mut push_cell = |cell: &dyn HistoryCell| {
            if !lines.is_empty() {
                lines.push(Line::default());
            }
            lines.extend(cell.display_lines(width));
        };

        if let Some(cell) = self.transcript.active_cell.as_ref() {
            push_cell(cell.as_ref());
        }
        if let Some(cell) = self.transcript.live_tail.as_ref() {
            push_cell(cell.as_ref());
        }
        lines
    }

    fn active_desired_height(&self, width: u16) -> u16 {
        let Some(cell) = self.transcript.active_cell.as_ref() else {
            return 0;
        };
        let key = ActiveCellLayoutCacheKey {
            cell_identity: cell.as_ref() as *const dyn HistoryCell as *const () as usize,
            revision: self.transcript.active_cell_revision,
            width,
            syntax_theme_revision: crate::render::highlight::syntax_theme_revision(),
        };
        if let Some(layout) = self.transcript.active_cell_layout.get()
            && layout.key == key
        {
            return layout.desired_height;
        }

        let desired_height = cell.desired_height(width.max(1));
        self.transcript.active_cell_layout.set(Some(ActiveCellLayoutCache { key, desired_height }));
        desired_height
    }
}

impl Renderable for TranscriptAreaRenderable<'_> {
    // 计算 `render` 所需的界面结果。
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let content_area = Rect::new(area.x, area.y, area.width.saturating_sub(1), area.height);
        let content_width = content_area.width.max(1);
        let paragraph =
            Paragraph::new(Text::from(self.lines(content_width))).wrap(Wrap { trim: false });
        let rendered_height = paragraph.line_count(content_width);
        let viewport_height = usize::from(content_area.height.max(1));
        let scroll_top = self.transcript.viewport_scroll_top(rendered_height, viewport_height);
        Clear.render(area, buf);
        paragraph
            .scroll((u16::try_from(scroll_top).unwrap_or(u16::MAX), 0))
            .render(content_area, buf);
        if rendered_height > viewport_height && area.width > 1 {
            let scrollbar_area = Rect::new(area.right().saturating_sub(1), area.y, 1, area.height);
            let max_top = rendered_height.saturating_sub(viewport_height);
            let mut state = ScrollbarState::new(max_top)
                .position(scroll_top)
                .viewport_content_length(viewport_height);
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(Some("│"))
                .thumb_symbol("┃")
                .render(scrollbar_area, buf, &mut state);
        }
        self.transcript.finish_render(area, content_area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        // Keep the copied active-cell height cache hot while measuring the full
        // static transcript; committed and live cells remain immutable inputs.
        // 测量完整静态对话记录时保持活动单元高度缓存可用；已提交单元与活动单元仍作为不可变输入。
        let _ = self.active_desired_height(width);
        Paragraph::new(Text::from(self.lines(width)))
            .wrap(Wrap { trim: false })
            .line_count(width.max(1))
            .try_into()
            .unwrap_or(u16::MAX)
    }
}

impl Renderable for ChatWidget {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.as_renderable().render(area, buf);
        self.note_rendered_width(area.width);
    }

    // 计算 `desired_height` 所需的界面结果。
    fn desired_height(&self, width: u16) -> u16 {
        self.as_renderable().desired_height(width)
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.as_renderable().cursor_pos(area)
    }

    fn cursor_style(&self, area: Rect) -> SetCursorStyle {
        self.as_renderable().cursor_style(area)
    }
}

#[cfg(test)]
#[path = "rendering_tests.rs"]
mod tests;
