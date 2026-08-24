//! Committed, active, and live-tail cells owned by `ChatWidget`.
//! 本模块负责已提交、活动与实时尾部对话单元。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use std::cell::{Cell, RefCell};

use crate::history_cell::{HistoryCell, history_cell_for};
use crate::vibe::event::V3DisplayEvent;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// 描述 `ActiveCellLayoutCacheKey` 在当前界面流程中传递的数据。
pub(super) struct ActiveCellLayoutCacheKey {
    pub(super) cell_identity: usize,
    pub(super) revision: u64,
    pub(super) width: u16,
    pub(super) syntax_theme_revision: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// 描述 `ActiveCellLayoutCache` 在本模块状态转换中传递的数据。
pub(super) struct ActiveCellLayoutCache {
    pub(super) key: ActiveCellLayoutCacheKey,
    pub(super) desired_height: u16,
}

pub(super) struct TranscriptState {
    pub(super) committed_history: Vec<Box<dyn HistoryCell>>,
    pub(super) active_cell: Option<Box<dyn HistoryCell>>,
    pub(super) live_tail: Option<Box<dyn HistoryCell>>,
    pub(super) active_cell_revision: u64,
    pub(super) active_cell_layout: Cell<Option<ActiveCellLayoutCache>>,
    scroll_top: Cell<usize>,
    follow_bottom: Cell<bool>,
    last_content_height: Cell<usize>,
    last_viewport_height: Cell<usize>,
    last_area: Cell<Rect>,
    last_content_area: Cell<Rect>,
    visible_cells: RefCell<Vec<Vec<String>>>,
    scrollbar_drag_offset: Cell<Option<usize>>,
    selection_anchor: Cell<Option<(u16, u16)>>,
    selection_head: Cell<Option<(u16, u16)>>,
    selecting: Cell<bool>,
}

pub(super) enum TranscriptMouseAction {
    None,
    Copy(String),
}

impl Default for TranscriptState {
    fn default() -> Self {
        Self {
            committed_history: Vec::new(),
            active_cell: None,
            live_tail: None,
            active_cell_revision: 0,
            active_cell_layout: Cell::new(None),
            scroll_top: Cell::new(0),
            follow_bottom: Cell::new(true),
            last_content_height: Cell::new(0),
            last_viewport_height: Cell::new(1),
            last_area: Cell::new(Rect::ZERO),
            last_content_area: Cell::new(Rect::ZERO),
            visible_cells: RefCell::new(Vec::new()),
            scrollbar_drag_offset: Cell::new(None),
            selection_anchor: Cell::new(None),
            selection_head: Cell::new(None),
            selecting: Cell::new(false),
        }
    }
}

impl TranscriptState {
    // 构造并展示 `push_cell` 对应的界面内容。
    pub(super) fn push_cell(&mut self, cell: Box<dyn HistoryCell>) {
        self.committed_history.push(cell);
    }

    pub(super) fn push_history(&mut self, event: V3DisplayEvent) {
        self.committed_history.push(history_cell_for(&event));
    }

    pub(super) fn set_session_header(&mut self, data: crate::V3SessionDisplayData) {
        if let Some(cell) = self
            .committed_history
            .iter_mut()
            .find(|cell| cell.as_any().is::<crate::history_cell::SessionHistoryCell>())
        {
            *cell = Box::new(crate::history_cell::SessionHistoryCell::header(data));
        } else {
            self.committed_history.insert(
                0,
                Box::new(crate::history_cell::SessionHistoryCell::header(data)),
            );
        }
    }

    pub(super) fn set_active_event(&mut self, event: Option<V3DisplayEvent>) {
        self.active_cell = event.as_ref().map(history_cell_for);
        self.bump_active_cell_revision();
    }

    // 更新 `set_live_tail_event` 对应的本地展示状态。
    pub(super) fn set_live_tail_event(&mut self, event: Option<V3DisplayEvent>) {
        self.live_tail = event.as_ref().map(history_cell_for);
    }

    pub(super) fn commit_active_cell(&mut self) {
        if let Some(cell) = self.active_cell.take() {
            self.committed_history.push(cell);
            self.bump_active_cell_revision();
        }
    }

    pub(super) fn bump_active_cell_revision(&mut self) {
        self.active_cell_revision = self.active_cell_revision.wrapping_add(1);
        self.active_cell_layout.set(None);
    }

    pub(super) fn history_len(&self) -> usize {
        self.committed_history.len()
    }

    // 计算 `scroll_lines` 所需的界面结果。
    pub(super) fn scroll_lines(&self, delta: isize) {
        let max_top = self
            .last_content_height
            .get()
            .saturating_sub(self.last_viewport_height.get().max(1));
        let current = if self.follow_bottom.get() { max_top } else { self.scroll_top.get() };
        let next = current.saturating_add_signed(delta).min(max_top);
        self.scroll_top.set(next);
        self.follow_bottom.set(next == max_top);
        self.clear_selection();
    }

    pub(super) fn scroll_page(&self, pages: isize) {
        let lines = isize::try_from(self.last_viewport_height.get().max(1)).unwrap_or(isize::MAX);
        self.scroll_lines(lines.saturating_mul(pages));
    }

    pub(super) fn scroll_to_top(&self) {
        self.scroll_top.set(0);
        self.follow_bottom.set(false);
    }

    pub(super) fn scroll_to_bottom(&self) {
        self.follow_bottom.set(true);
    }

    // 计算 `viewport_scroll_top` 所需的界面结果。
    pub(super) fn viewport_scroll_top(
        &self,
        content_height: usize,
        viewport_height: usize,
    ) -> usize {
        let viewport_height = viewport_height.max(1);
        let max_top = content_height.saturating_sub(viewport_height);
        let top = if self.follow_bottom.get() {
            max_top
        } else {
            self.scroll_top.get().min(max_top)
        };
        self.scroll_top.set(top);
        self.last_content_height.set(content_height);
        self.last_viewport_height.set(viewport_height);
        top
    }

    pub(super) fn transcript_lines(&self, width: u16, include_active: bool) -> Vec<Line<'static>> {
        let mut lines = self
            .committed_history
            .iter()
            .flat_map(|cell| cell.display_lines(width))
            .collect::<Vec<_>>();
        if include_active {
            if let Some(cell) = self.active_cell.as_ref() {
                lines.extend(cell.display_lines(width));
            }
            if let Some(cell) = self.live_tail.as_ref() {
                lines.extend(cell.display_lines(width));
            }
        }
        lines
    }

    pub(super) fn finish_render(&self, area: Rect, content_area: Rect, buf: &mut Buffer) {
        self.last_area.set(area);
        self.last_content_area.set(content_area);
        let visible_cells = content_area
            .rows()
            .map(|row| {
                content_area
                    .columns()
                    .map(|column| buf[(column.x, row.y)].symbol().to_string())
                    .collect()
            })
            .collect();
        *self.visible_cells.borrow_mut() = visible_cells;
        self.render_selection(content_area, buf);
    }

    pub(super) fn handle_mouse_event(&self, event: MouseEvent) -> TranscriptMouseAction {
        match event.kind {
            MouseEventKind::ScrollUp => {
                self.scroll_lines(-3);
                TranscriptMouseAction::None
            }
            MouseEventKind::ScrollDown => {
                self.scroll_lines(3);
                TranscriptMouseAction::None
            }
            MouseEventKind::Down(MouseButton::Left) if self.is_scrollbar_point(event.column, event.row) => {
                let Some((thumb_start, thumb_length)) = self.scrollbar_thumb() else {
                    return TranscriptMouseAction::None;
                };
                let area = self.last_area.get();
                let pointer = usize::from(event.row.saturating_sub(area.y));
                let drag_offset = if pointer >= thumb_start
                    && pointer < thumb_start.saturating_add(thumb_length)
                {
                    pointer.saturating_sub(thumb_start)
                } else {
                    thumb_length / 2
                };
                self.scrollbar_drag_offset.set(Some(drag_offset));
                self.scroll_from_pointer(event.row, drag_offset);
                self.clear_selection();
                TranscriptMouseAction::None
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(drag_offset) = self.scrollbar_drag_offset.get() {
                    self.scroll_from_pointer(event.row, drag_offset);
                } else if self.selecting.get() {
                    self.selection_head.set(self.clamp_selection_point(event.column, event.row));
                }
                TranscriptMouseAction::None
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if let Some(drag_offset) = self.scrollbar_drag_offset.take() {
                    self.scroll_from_pointer(event.row, drag_offset);
                    return TranscriptMouseAction::None;
                }
                if !self.selecting.replace(false) {
                    return TranscriptMouseAction::None;
                }
                self.selection_head.set(self.clamp_selection_point(event.column, event.row));
                self.selected_text().map_or(TranscriptMouseAction::None, TranscriptMouseAction::Copy)
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let Some(point) = self.selection_start_point(event.column, event.row) else {
                    return TranscriptMouseAction::None;
                };
                self.selection_anchor.set(Some(point));
                self.selection_head.set(Some(point));
                self.selecting.set(true);
                TranscriptMouseAction::None
            }
            _ => TranscriptMouseAction::None,
        }
    }

    fn scrollbar_thumb(&self) -> Option<(usize, usize)> {
        let area = self.last_area.get();
        let track_length = usize::from(area.height);
        let content_height = self.last_content_height.get();
        let viewport_height = self.last_viewport_height.get().max(1);
        let max_top = content_height.saturating_sub(viewport_height);
        if track_length == 0 || max_top == 0 {
            return None;
        }
        let thumb_length = viewport_height
            .saturating_mul(track_length)
            .div_ceil(content_height)
            .clamp(1, track_length);
        let travel = track_length.saturating_sub(thumb_length);
        let thumb_start = self.scroll_top.get().min(max_top).saturating_mul(travel) / max_top;
        Some((thumb_start, thumb_length))
    }

    fn is_scrollbar_point(&self, column: u16, row: u16) -> bool {
        let area = self.last_area.get();
        self.scrollbar_thumb().is_some()
            && area.width > 1
            && column == area.right().saturating_sub(1)
            && row >= area.y
            && row < area.bottom()
    }

    fn scroll_from_pointer(&self, row: u16, drag_offset: usize) {
        let Some((_, thumb_length)) = self.scrollbar_thumb() else {
            return;
        };
        let area = self.last_area.get();
        let track_length = usize::from(area.height);
        let travel = track_length.saturating_sub(thumb_length);
        let pointer = usize::from(row.saturating_sub(area.y)).min(track_length.saturating_sub(1));
        let thumb_start = pointer.saturating_sub(drag_offset).min(travel);
        let max_top = self
            .last_content_height
            .get()
            .saturating_sub(self.last_viewport_height.get().max(1));
        let next = if travel == 0 {
            0
        } else {
            thumb_start.saturating_mul(max_top).saturating_add(travel / 2) / travel
        };
        self.scroll_top.set(next);
        self.follow_bottom.set(next == max_top);
    }

    fn clamp_selection_point(&self, column: u16, row: u16) -> Option<(u16, u16)> {
        let area = self.last_content_area.get();
        if area.is_empty() {
            return None;
        }
        Some((
            column.clamp(area.x, area.right().saturating_sub(1)),
            row.clamp(area.y, area.bottom().saturating_sub(1)),
        ))
    }

    fn selection_start_point(&self, column: u16, row: u16) -> Option<(u16, u16)> {
        let area = self.last_content_area.get();
        (column >= area.x
            && column < area.right()
            && row >= area.y
            && row < area.bottom())
            .then_some((column, row))
    }

    fn selected_text(&self) -> Option<String> {
        let anchor = self.selection_anchor.get()?;
        let head = self.selection_head.get()?;
        if anchor == head {
            self.clear_selection();
            return None;
        }
        let (start, end) = ordered_points(anchor, head);
        let area = self.last_content_area.get();
        let cells = self.visible_cells.borrow();
        let mut lines = Vec::new();
        for row in start.1..=end.1 {
            let row_index = usize::from(row.saturating_sub(area.y));
            let row_cells = cells.get(row_index)?;
            let first_column = if row == start.1 { start.0 } else { area.x };
            let last_column = if row == end.1 {
                end.0
            } else {
                area.right().saturating_sub(1)
            };
            let first = usize::from(first_column.saturating_sub(area.x));
            let last = usize::from(last_column.saturating_sub(area.x));
            let line = row_cells
                .get(first..=last)?
                .iter()
                .map(String::as_str)
                .collect::<String>();
            lines.push(line.trim_end().to_string());
        }
        let text = lines.join("\n");
        (!text.is_empty()).then_some(text)
    }

    fn render_selection(&self, area: Rect, buf: &mut Buffer) {
        let (Some(anchor), Some(head)) = (self.selection_anchor.get(), self.selection_head.get())
        else {
            return;
        };
        let (start, end) = ordered_points(anchor, head);
        for row in start.1..=end.1 {
            let first = if row == start.1 { start.0 } else { area.x };
            let last = if row == end.1 { end.0 } else { area.right().saturating_sub(1) };
            for column in first..=last {
                buf[(column, row)].set_style(Style::default().add_modifier(Modifier::REVERSED));
            }
        }
    }

    fn clear_selection(&self) {
        self.selection_anchor.set(None);
        self.selection_head.set(None);
        self.selecting.set(false);
    }
}

fn ordered_points(left: (u16, u16), right: (u16, u16)) -> ((u16, u16), (u16, u16)) {
    if (left.1, left.0) <= (right.1, right.0) { (left, right) } else { (right, left) }
}

#[cfg(test)]
#[path = "transcript_tests.rs"]
mod tests;
