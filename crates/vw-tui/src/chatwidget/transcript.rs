//! Committed, active, and live-tail cells owned by `ChatWidget`.
//! 本模块负责已提交、活动与实时尾部对话单元。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。

use std::cell::Cell;

use crate::history_cell::{HistoryCell, history_cell_for};
use crate::vibe::event::V3DisplayEvent;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_cell_commits_into_history_and_invalidates_layout() {
        let mut state = TranscriptState::default();
        state.set_active_event(Some(V3DisplayEvent::Progress {
            label: "working".into(),
            detail: None,
        }));
        state.active_cell_layout.set(Some(ActiveCellLayoutCache {
            key: ActiveCellLayoutCacheKey {
                cell_identity: 1,
                revision: state.active_cell_revision,
                width: 80,
                syntax_theme_revision: 0,
            },
            desired_height: 1,
        }));

        state.commit_active_cell();

        assert_eq!(state.committed_history.len(), 1);
        assert!(state.active_cell.is_none());
        assert_eq!(state.active_cell_layout.get(), None);
    }

    #[test]
    fn transcript_lines_join_committed_active_and_live_cells() {
        let mut state = TranscriptState::default();
        state.push_history(V3DisplayEvent::System { text: "committed".into() });
        state.set_active_event(Some(V3DisplayEvent::Progress {
            label: "active".into(),
            detail: None,
        }));
        state.set_live_tail_event(Some(V3DisplayEvent::Assistant {
            markdown: "live".into(),
        }));

        let committed = state
            .transcript_lines(40, false)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let full = state
            .transcript_lines(40, true)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(committed.contains("committed"));
        assert!(!committed.contains("active"));
        assert!(!committed.contains("live"));
        assert!(full.contains("committed"));
        assert!(full.contains("active"));
        assert!(full.contains("live"));
    }

    #[test]
    fn session_header_metadata_updates_in_place() {
        let mut state = TranscriptState::default();
        let display = |title: &str| crate::V3SessionDisplayData {
            version: "0.4.0".into(),
            cwd: "/workspace".into(),
            gateway_endpoint: "127.0.0.1:42617".into(),
            model: "token-plan/qwen3.7-plus".into(),
            permissions: "Default · full access".into(),
            session_id: Some("session-1".into()),
            session_title: Some(title.into()),
            token_usage: crate::V3TokenUsage::default(),
            resumed: true,
        };
        state.push_cell(Box::new(crate::history_cell::SessionHistoryCell::header(display("Old"))));
        state.push_history(V3DisplayEvent::System { text: "history".into() });

        state.set_session_header(display("Generated title"));

        assert_eq!(state.history_len(), 2);
        let rendered = state
            .transcript_lines(80, false)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("session:     Generated title"));
        assert!(!rendered.contains("session:     Old"));
        assert!(rendered.contains("history"));
    }
}
