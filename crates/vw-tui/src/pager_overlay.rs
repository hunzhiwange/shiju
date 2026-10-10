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
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn pager() -> TranscriptPager {
        TranscriptPager::new(
            (0..24).map(|index| Line::from(format!("transcript line {index:02}"))).collect(),
        )
    }

    fn buffer_text(buffer: &Buffer, area: Rect) -> String {
        (area.y..area.bottom())
            .map(|y| {
                (area.x..area.right())
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    // 初始末尾、跳到顶部和手动下移应分别显示对应位置标签。
    fn transcript_pager_tracks_top_middle_and_end() {
        let area = Rect::new(0, 0, 40, 10);
        let mut pager = pager();
        let mut buffer = Buffer::empty(area);
        pager.render(area, &mut buffer);
        assert!(buffer_text(&buffer, area).contains(" end "));

        pager.scroll_to_top();
        pager.render(area, &mut buffer);
        assert!(buffer_text(&buffer, area).contains(" top "));

        pager.scroll_lines(5);
        pager.render(area, &mut buffer);
        assert!(buffer_text(&buffer, area).contains(" middle "));
    }

    #[test]
    // 手动偏移在缩放后保持，而末尾跟随应按新视口重新钉住底部。
    fn transcript_pager_resize_preserves_manual_position_and_end_pin() {
        let mut pager = pager();
        let mut wide = Buffer::empty(Rect::new(0, 0, 60, 12));
        pager.render(*wide.area(), &mut wide);
        pager.scroll_to_top();
        pager.scroll_lines(4);

        let mut narrow = Buffer::empty(Rect::new(0, 0, 28, 7));
        pager.render(*narrow.area(), &mut narrow);
        assert_eq!(pager.scroll_offset, 4);

        pager.scroll_to_end();
        let mut tall = Buffer::empty(Rect::new(0, 0, 60, 18));
        pager.render(*tall.area(), &mut tall);
        assert_eq!(pager.scroll_offset, max_scroll_offset(24, 15));
    }

    #[test]
    // 快照覆盖窄、标准和宽三种尺寸下的边框、正文、滚动条与页脚。
    fn transcript_pager_three_size_snapshots() {
        let mut output = String::new();
        for (name, area) in [
            ("narrow", Rect::new(0, 0, 28, 7)),
            ("standard", Rect::new(0, 0, 40, 10)),
            ("wide", Rect::new(0, 0, 60, 12)),
        ] {
            let mut pager = pager();
            pager.scroll_to_top();
            pager.scroll_lines(3);
            let mut buffer = Buffer::empty(area);
            pager.render(area, &mut buffer);
            output.push_str(&format!("--- {name} ---\n{}\n", buffer_text(&buffer, area)));
        }
        assert_snapshot!("v3_transcript_pager_three_sizes", output);
    }
}
