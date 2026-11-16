//! Full-transcript pager adapted from the copied Codex overlay.
//! 从复制的 Codex 覆盖层适配而来的完整记录分页器。
//!
//! 分页器只持有展示状态；它接收由 v3 历史单元和活动单元生成的行，
//! 并在重绘及终端尺寸变化时稳定保持顶部、中部和末尾位置。
//! The pager owns display state only. It receives lines built from v3 history/active cells and
//! keeps top, middle and end positions stable across redraws and terminal resizes.

mod scrolling;
mod search;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize as _};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    StatefulWidget, Widget,
};

use scrolling::{clamp_offset, max_scroll_offset};

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
    search: Option<search::Search>,
    source_revision: Option<u64>,
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
            search: None,
            source_revision: None,
        }
    }

    pub fn set_live_tail(&mut self, lines: Vec<Line<'static>>) {
        // 只有仍在跟随底部时，新尾部才把偏移重置为末尾哨兵。
        if self.live_tail != lines {
            if let Some(search) = &mut self.search {
                search.invalidate_from(self.committed.len());
            }
        }
        self.live_tail = lines;
        if self.follow_bottom {
            self.scroll_offset = usize::MAX;
        }
    }

    pub fn replace_committed(&mut self, lines: Vec<Line<'static>>) {
        if self.committed != lines {
            let first_changed = self
                .committed
                .iter()
                .zip(&lines)
                .position(|(old, new)| old != new)
                .unwrap_or(self.committed.len().min(lines.len()));
            if let Some(search) = &mut self.search {
                search.invalidate_from(first_changed);
            }
        }
        self.committed = lines;
        if self.follow_bottom {
            self.scroll_offset = usize::MAX;
        }
    }

    /// Claims a changed source revision before the caller materializes its lines.
    pub fn needs_source_refresh(&mut self, revision: u64) -> bool {
        if self.source_revision == Some(revision) {
            return false;
        }
        self.source_revision = Some(revision);
        true
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
        if let Some(search) = &mut self.search {
            match event.code {
                KeyCode::Esc => {
                    let saved = search.saved;
                    self.scroll_offset = saved.0;
                    self.follow_bottom = saved.1;
                    self.search = None;
                    return;
                }
                KeyCode::Enter => search.editing = false,
                KeyCode::Backspace if search.editing => {
                    search.query.pop();
                    search.restart();
                }
                KeyCode::Char(ch)
                    if search.editing
                        && !event
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    search.push(ch)
                }
                KeyCode::Char('n') => search.navigate(false),
                KeyCode::Char('N') => search.navigate(true),
                KeyCode::Char('/') => search.editing = true,
                _ => {}
            }
            return;
        }
        match event.code {
            KeyCode::Char('/') => {
                self.search = Some(search::Search {
                    saved: (self.scroll_offset, self.follow_bottom),
                    editing: true,
                    ..Default::default()
                });
            }
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
        if let Some(search) = &mut self.search {
            let spent = search.scan(&self.committed, search::SCAN_BYTES);
            // The tail is a separate slice; scan it using logical indices without cloning.
            if search.line >= self.committed.len() && spent < search::SCAN_BYTES {
                search.line -= self.committed.len();
                let first = search.hits.len();
                search.scan(&self.live_tail, search::SCAN_BYTES - spent);
                for hit in &mut search.hits[first..] {
                    hit.0 += self.committed.len();
                }
                search.line += self.committed.len();
            }
            if let Some(&(line, _)) = search.hits.get(search.selected) {
                self.scroll_offset = line;
                self.follow_bottom = false;
            }
        }
        let content_height = self.content_height();
        let max_offset = max_scroll_offset(content_height, self.last_viewport_height);
        self.scroll_offset = if self.follow_bottom || self.scroll_offset == usize::MAX {
            max_offset
        } else {
            clamp_offset(self.scroll_offset, content_height, self.last_viewport_height)
        };

        let mut screen_row = 0u16;
        for (row, line) in self
            .committed
            .iter()
            .chain(&self.live_tail)
            .skip(self.scroll_offset)
            .take(usize::from(content_area.height))
            .enumerate()
        {
            if screen_row >= content_area.height {
                break;
            }
            let selected = self.search.as_ref().and_then(|search| search.hits.get(search.selected));
            let horizontal = selected
                .filter(|hit| hit.0 == self.scroll_offset + row)
                .map(|hit| {
                    let mut remaining = hit.1;
                    let mut width = 0;
                    for span in &line.spans {
                        let bytes = remaining.min(span.content.len());
                        width += ratatui::text::Span::raw(&span.content[..bytes]).width();
                        remaining -= bytes;
                        if remaining == 0 {
                            break;
                        }
                    }
                    width.min(usize::from(u16::MAX)) as u16
                })
                .unwrap_or(0);
            let paragraph = Paragraph::new(line.clone());
            let height = if horizontal > 0 {
                1
            } else {
                paragraph
                    .clone()
                    .wrap(ratatui::widgets::Wrap { trim: false })
                    .line_count(content_area.width.max(1))
                    .max(1)
                    .min(usize::from(content_area.height - screen_row)) as u16
            };
            let paragraph = if horizontal > 0 {
                paragraph.scroll((0, horizontal))
            } else {
                paragraph.wrap(ratatui::widgets::Wrap { trim: false })
            };
            paragraph.render(
                Rect::new(content_area.x, content_area.y + screen_row, content_area.width, height),
                buf,
            );
            screen_row += height;
        }

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
        let footer = if let Some(search) = &self.search {
            let status = if search.query.is_empty() {
                "type literal query".to_string()
            } else if search.line < content_height {
                format!("scanning; {} hits", search.hits.len())
            } else if search.hits.is_empty() {
                "no match".to_string()
            } else {
                format!("{}/{}", search.selected + 1, search.hits.len())
            };
            Line::from(format!(
                " /{}  {status} | loaded only | Enter read n/N next/prev Esc restore",
                search.query
            ))
        } else {
            Line::from(vec![
                format!(" {position} ").bold(),
                "↑/↓ scroll  PgUp/PgDn page  Home/End jump  Esc close  / search loaded".dim(),
            ])
        };
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

#[cfg(test)]
mod workflow_req_01_tests;
