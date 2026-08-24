//! 本模块集中实现 `file_search_popup` 在底部面板中的职责。
//! 关键声明 `MAX_POPUP_ROWS`、`FileSearchPopup`、`new`、`set_query` 共同界定本文件的对外交互面。
//! 状态读取与更新均沿用既有类型，不在渲染路径中引入额外业务副作用。
//! 交互分支通过明确枚举或返回值向上层传递选择结果。
//! 空数据、越界索引与不可用选项按现有分支保持可预期的回退。


//! 确定性的 VibeWindow 提供的匹配项搜索弹窗
//! File-search popup over deterministic VibeWindow-provided matches.
use crate::bottom_pane::scroll_state::ScrollState;
use crate::bottom_pane::selection_popup_common::{GenericDisplayRow, render_rows};
use crate::vibe::action::{V3Action, V3FileSearchMatch};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::WidgetRef;
const MAX_POPUP_ROWS: usize = 8;
pub(crate) struct FileSearchPopup {
    display_query: String,
    pending_query: String,
    waiting: bool,
    error_message: Option<String>,
    matches: Vec<V3FileSearchMatch>,
    state: ScrollState,
}
impl FileSearchPopup {
    pub(crate) fn new() -> Self {
        Self {
            display_query: String::new(),
            pending_query: String::new(),
            waiting: true,
            error_message: None,
            matches: Vec::new(),
            state: ScrollState::new(),
        }
    }
    pub(crate) fn set_query(&mut self, query: &str) {
        if query != self.pending_query {
            self.pending_query = query.to_string();
            self.waiting = true;
            self.error_message = None;
        }
    }
    pub(crate) fn set_empty_prompt(&mut self) {
        self.display_query.clear();
        self.pending_query.clear();
        self.waiting = false;
        self.error_message = None;
        self.matches.clear();
        self.state.reset();
    }
    pub(crate) fn set_matches(&mut self, query: &str, matches: Vec<V3FileSearchMatch>) {
        if query != self.pending_query {
            return;
        }
        self.display_query = query.to_string();
        self.matches = matches.into_iter().take(MAX_POPUP_ROWS).collect();
        self.waiting = false;
        self.error_message = None;
        self.state.clamp_selection(self.matches.len());
        self.state.ensure_visible(self.matches.len(), MAX_POPUP_ROWS);
    }
    pub(crate) fn set_error(&mut self, query: &str, message: String) {
        if query != self.pending_query {
            return;
        }
        self.display_query = query.to_string();
        self.matches.clear();
        self.waiting = false;
        self.error_message = Some(message);
        self.state.reset();
    }
    pub(crate) fn selected_match(&self) -> Option<&V3FileSearchMatch> {
        self.state.selected_idx.and_then(|idx| self.matches.get(idx))
    }
    pub(crate) fn handle_key_event(&mut self, event: KeyEvent) -> V3Action {
        let len = self.matches.len();
        match event.code {
            KeyCode::Up => self.state.move_up_wrap(len),
            KeyCode::Down => self.state.move_down_wrap(len),
            KeyCode::Enter => {
                if let Some(item) = self.selected_match() {
                    return V3Action::Select {
                        surface: "file_search".into(),
                        item_id: if item.id.is_empty() {
                            item.path.clone()
                        } else {
                            item.id.clone()
                        },
                    };
                }
            }
            KeyCode::Esc => return V3Action::Cancel,
            _ => {}
        }
        self.state.ensure_visible(len, MAX_POPUP_ROWS.min(len.max(1)));
        V3Action::None
    }
    pub(crate) fn calculate_required_height(&self) -> u16 {
        self.matches.len().clamp(1, MAX_POPUP_ROWS) as u16
    }
}
impl WidgetRef for &FileSearchPopup {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let rows = self
            .matches
            .iter()
            .map(|item| GenericDisplayRow {
                name: item.path.clone(),
                match_indices: (!item.match_indices.is_empty())
                    .then_some(item.match_indices.clone()),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        render_rows(
            area,
            buf,
            &rows,
            &self.state,
            MAX_POPUP_ROWS,
            self.error_message
                .as_deref()
                .unwrap_or(if self.waiting { "loading..." } else { "no matches" }),
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    // 验证 `stale_file_search_results_are_ignored` 场景的触发条件与预期结果。
    #[test]
    fn stale_file_search_results_are_ignored() {
        let mut popup = FileSearchPopup::new();
        popup.set_query("src");
        popup.set_matches(
            "old",
            vec![V3FileSearchMatch {
                id: "old".into(),
                path: "old.rs".into(),
                ..Default::default()
            }],
        );
        assert!(popup.selected_match().is_none());
    }
    // 验证 `file_search_selection_emits_only_an_action` 场景的触发条件与预期结果。
    #[test]
    fn file_search_selection_emits_only_an_action() {
        let mut popup = FileSearchPopup::new();
        popup.set_query("src");
        popup.set_matches(
            "src",
            vec![V3FileSearchMatch {
                id: "main".into(),
                path: "src/main.rs".into(),
                ..Default::default()
            }],
        );
        assert_eq!(
            popup.handle_key_event(KeyEvent::from(KeyCode::Enter)),
            V3Action::Select { surface: "file_search".into(), item_id: "main".into() }
        );
    }
}
