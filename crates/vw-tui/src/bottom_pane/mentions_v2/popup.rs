//! 本模块集中实现 `popup` 在底部面板中的职责。
//! 关键声明 `Popup`、`new`、`set_candidates`、`set_query` 共同界定本文件的对外交互面。
//! 状态读取与更新均沿用既有类型，不在渲染路径中引入额外业务副作用。
//! 交互分支通过明确枚举或返回值向上层传递选择结果。
//! 空数据、越界索引与不可用选项按现有分支保持可预期的回退。
//! 布局计算以当前可用区域为边界，避免向小终端尺寸外绘制。
//! 键盘输入只在当前界面状态内解释，未匹配输入继续遵循上层约定。


use codex_file_search::FileMatch;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::WidgetRef;

use super::candidate::Candidate;
use super::candidate::SearchResult;
use super::candidate::Selection;
use super::filter::filtered_candidates;
use super::render::render_popup;
use super::search_mode::SearchMode;
use crate::bottom_pane::popup_consts::MAX_POPUP_ROWS;
use crate::bottom_pane::scroll_state::ScrollState;

pub(crate) struct Popup {
    query: String,
    file_search: FileSearch,
    candidates: Vec<Candidate>,
    rows: Vec<SearchResult>,
    search_mode: SearchMode,
    state: ScrollState,
}

impl Popup {
    pub(crate) fn new(candidates: Vec<Candidate>, query: &str) -> Self {
        let mut file_search = FileSearch::default();
        file_search.set_query(query);
        let mut popup = Self {
            query: query.to_string(),
            file_search,
            candidates,
            rows: Vec::new(),
            search_mode: SearchMode::Results,
            state: ScrollState::new(),
        };
        popup.refresh_rows();
        popup
    }

    pub(crate) fn set_candidates(&mut self, candidates: Vec<Candidate>) {
        self.candidates = candidates;
        self.refresh_rows();
    }

    pub(crate) fn set_query(&mut self, query: &str) {
        if self.query == query {
            return;
        }
        self.query = query.to_string();
        self.file_search.set_query(query);
        self.refresh_rows();
    }

    pub(crate) fn set_file_matches(&mut self, query: &str, matches: Vec<FileMatch>) {
        self.file_search.set_matches(query, matches);
        self.refresh_rows();
    }

    pub(crate) fn selected(&self) -> Option<Selection> {
        let idx = self.state.selected_idx?;
        self.rows.get(idx).map(|row| row.selection.clone())
    }

    pub(crate) fn move_up(&mut self) {
        let len = self.rows.len();
        self.state.move_up_wrap(len);
        self.state.ensure_visible(len, MAX_POPUP_ROWS.min(len));
    }

    pub(crate) fn move_down(&mut self) {
        let len = self.rows.len();
        self.state.move_down_wrap(len);
        self.state.ensure_visible(len, MAX_POPUP_ROWS.min(len));
    }

    pub(crate) fn previous_search_mode(&mut self) {
        self.search_mode = self.search_mode.previous();
        self.refresh_rows();
    }

    pub(crate) fn next_search_mode(&mut self) {
        self.search_mode = self.search_mode.next();
        self.refresh_rows();
    }

    pub(crate) fn calculate_required_height(&self, _width: u16) -> u16 {
        let visible = self.rows.len().clamp(1, MAX_POPUP_ROWS);
        (visible as u16).saturating_add(2)
    }

    /// 在搜索输入改变后保持选择有效，并重新缓存行数据。
    /// Rebuilds cached rows and keeps selection valid after search inputs change.
    fn refresh_rows(&mut self) {
        self.rows = filtered_candidates(
            &self.candidates,
            &self.file_search.matches,
            &self.query,
            self.search_mode,
            self.file_search.should_show_matches(),
        );
        let len = self.rows.len();
        self.state.clamp_selection(len);
        self.state.ensure_visible(len, MAX_POPUP_ROWS.min(len));
    }
}

impl WidgetRef for Popup {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        render_popup(
            area,
            buf,
            &self.rows,
            &self.state,
            self.file_search.empty_message(),
            self.search_mode,
        );
    }
}

#[derive(Default)]
struct FileSearch {
    pending_query: String,
    display_query: String,
    waiting: bool,
    matches: Vec<FileMatch>,
}

impl FileSearch {
    fn set_query(&mut self, query: &str) {
        if query.is_empty() {
            self.pending_query.clear();
            self.display_query.clear();
            self.waiting = false;
            self.matches.clear();
        } else if query != self.pending_query {
            self.pending_query = query.to_string();
            self.waiting = true;
        }
    }

    fn set_matches(&mut self, query: &str, matches: Vec<FileMatch>) {
        if query != self.pending_query {
            return;
        }

        self.display_query = query.to_string();
        self.matches = matches.into_iter().take(MAX_POPUP_ROWS).collect();
        self.waiting = false;
    }

    fn should_show_matches(&self) -> bool {
        !self.matches.is_empty()
    }

    fn empty_message(&self) -> &'static str {
        if self.waiting {
            "loading..."
        } else {
            "no matches"
        }
    }
}
