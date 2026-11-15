//! Full-transcript pager adapted from the copied Codex overlay.
//! 从复制的 Codex 覆盖层适配而来的完整记录分页器。
//!
//! 分页器只持有展示状态；它接收由 v3 历史单元和活动单元生成的行，
//! 并在重绘及终端尺寸变化时稳定保持顶部、中部和末尾位置。
//! The pager owns display state only. It receives lines built from v3 history/active cells and
//! keeps top, middle and end positions stable across redraws and terminal resizes.

mod scrolling;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize as _};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    StatefulWidget, Widget,
};

use scrolling::{clamp_offset, max_scroll_offset, render_scrolled_lines};

/// 已提交记录及可选流式尾部的分页状态。
/// Pager state for the committed transcript plus an optional in-flight tail.
#[derive(Debug)]
pub struct TranscriptPager {
    committed: Vec<Line<'static>>,
    live_tail: Vec<Line<'static>>,
    scroll_offset: usize,
    follow_bottom: bool,
    done: bool,
    last_viewport_height: usize,
}

impl TranscriptPager {
    pub fn new(committed: Vec<Line<'static>>) -> Self {
        Self {
            committed,
            live_tail: Vec::new(),
            scroll_offset: usize::MAX,
            follow_bottom: true,
            done: false,
            last_viewport_height: 1,
        }
    }

    pub fn set_live_tail(&mut self, lines: Vec<Line<'static>>) {
        // 只有仍在跟随底部时，新尾部才把偏移重置为末尾哨兵。
        self.live_tail = lines;
        if self.follow_bottom {
            self.scroll_offset = usize::MAX;
        }
    }

    pub fn replace_committed(&mut self, lines: Vec<Line<'static>>) {
        self.committed = lines;
        if self.follow_bottom {
            self.scroll_offset = usize::MAX;
        }
    }

    pub fn is_done(&self) -> bool {
        self.done
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
        self.follow_bottom = false;
    }

    pub fn scroll_to_end(&mut self) {
        self.scroll_offset = usize::MAX;
        self.follow_bottom = true;
    }

    pub fn scroll_lines(&mut self, delta: isize) {
        // 先把末尾哨兵解析为实际偏移，再做有符号饱和移动和边界夹取。
        let current = if self.scroll_offset == usize::MAX {
            max_scroll_offset(self.content_height(), self.last_viewport_height)
        } else {
            self.scroll_offset
        };
        self.scroll_offset = current.saturating_add_signed(delta);
        self.scroll_offset =
            clamp_offset(self.scroll_offset, self.content_height(), self.last_viewport_height);
        self.follow_bottom = self.scroll_offset
            == max_scroll_offset(self.content_height(), self.last_viewport_height);
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Esc | KeyCode::Char('q') => self.done = true,
            KeyCode::Home | KeyCode::Char('g') => self.scroll_to_top(),
            KeyCode::End | KeyCode::Char('G') => self.scroll_to_end(),
            KeyCode::Up | KeyCode::Char('k') => self.scroll_lines(-1),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_lines(1),
            KeyCode::PageUp => self.scroll_lines(-(self.last_viewport_height as isize)),
            KeyCode::PageDown | KeyCode::Char(' ') => {
                self.scroll_lines(self.last_viewport_height as isize)
            }
            _ => {}
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // 内容区为右侧滚动条和底部帮助各预留一列/一行。
        Clear.render(area, buf);
        let block = Block::default().borders(Borders::ALL).title(" Full transcript ");
        let inner = block.inner(area);
        block.render(area, buf);
        if inner.is_empty() {
            return;
        }

        let content_area = Rect::new(
            inner.x,
            inner.y,
            inner.width.saturating_sub(1),
            inner.height.saturating_sub(1),
        );
        self.last_viewport_height = usize::from(content_area.height.max(1));
        let content_height = self.content_height();
        let max_offset = max_scroll_offset(content_height, self.last_viewport_height);
        self.scroll_offset = if self.follow_bottom || self.scroll_offset == usize::MAX {
            max_offset
        } else {
            clamp_offset(self.scroll_offset, content_height, self.last_viewport_height)
        };

        let mut lines = self.committed.clone();
        lines.extend(self.live_tail.iter().cloned());
        render_scrolled_lines(&lines, self.scroll_offset, content_area, buf);

        // 仅在内容溢出时渲染一列宽滚动条。
        if content_height > self.last_viewport_height {
            let scrollbar_area =
                Rect::new(inner.right().saturating_sub(1), inner.y, 1, content_area.height);
            let mut state = ScrollbarState::new(max_offset).position(self.scroll_offset);
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(Some("│"))
                .thumb_symbol("┃")
                .render(scrollbar_area, buf, &mut state);
        }

        let position = if self.scroll_offset == 0 {
            "top"
        } else if self.scroll_offset == max_offset {
            "end"
        } else {
            "middle"
        };
        let footer = Line::from(vec![
            format!(" {position} ").bold(),
            "↑/↓ scroll  PgUp/PgDn page  Home/End jump  Esc close".dim(),
        ]);
        Paragraph::new(footer)
            .style(Style::default())
            .render(Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1), buf);
    }

    fn content_height(&self) -> usize {
        self.committed.len().saturating_add(self.live_tail.len())
    }
}

#[cfg(test)]
mod tests;
