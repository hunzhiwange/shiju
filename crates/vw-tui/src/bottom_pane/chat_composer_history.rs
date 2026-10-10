//! 本模块集中实现 `chat_composer_history` 在底部面板中的职责。
//! 关键声明 `HistoryEntry`、`new`、`with_draft_state`、`HistorySearchDirection` 共同界定本文件的对外交互面。
//! 状态读取与更新均沿用既有类型，不在渲染路径中引入额外业务副作用。
//! 交互分支通过明确枚举或返回值向上层传递选择结果。

//! v3 输入核心本地历史模块
//! Local composer history for the v3 input core.
//!
//! History recall is deliberately local in this batch. Persistent session lookup belongs to the
//! later CLI bridge, so this module stores drafts, deduplicates adjacent submissions, and performs
//! reverse search without knowing about Gateway requests, permissions, or session identifiers.

use crate::bottom_pane::MentionBinding;
use crate::input_types::TextElement;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HistoryEntry {
    pub(crate) text: String,
    pub(crate) text_elements: Vec<TextElement>,
    pub(crate) local_image_paths: Vec<PathBuf>,
    pub(crate) remote_image_urls: Vec<String>,
    pub(crate) mention_bindings: Vec<MentionBinding>,
    pub(crate) pending_pastes: Vec<(String, String)>,
}

impl HistoryEntry {
    pub(crate) fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            text_elements: Vec::new(),
            local_image_paths: Vec::new(),
            remote_image_urls: Vec::new(),
            mention_bindings: Vec::new(),
            pending_pastes: Vec::new(),
        }
    }

    pub(crate) fn with_draft_state(
        text: impl Into<String>,
        text_elements: Vec<TextElement>,
        pending_pastes: Vec<(String, String)>,
    ) -> Self {
        Self {
            text: text.into(),
            text_elements,
            local_image_paths: Vec::new(),
            remote_image_urls: Vec::new(),
            mention_bindings: Vec::new(),
            pending_pastes,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HistorySearchDirection {
    Older,
    Newer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum HistorySearchResult {
    Found(HistoryEntry),
    AtBoundary,
    NotFound,
}

/// 编辑器在会话桥接前使用的内存式历史
/// In-memory shell-style history used by the editor before a session bridge is attached.
#[derive(Debug, Default)]
pub(crate) struct ChatComposerHistory {
    entries: Vec<HistoryEntry>,
    cursor: Option<usize>,
    draft_before_navigation: Option<String>,
    search_query: Option<String>,
    search_index: Option<usize>,
}

impl ChatComposerHistory {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    pub(crate) fn record_local_submission(&mut self, entry: HistoryEntry) {
        if entry.text.is_empty() || self.entries.last() == Some(&entry) {
            return;
        }
        self.entries.push(entry);
        self.reset_navigation();
    }

    pub(crate) fn should_handle_navigation(&self, text: &str, cursor: usize) -> bool {
        cursor == 0 || cursor == text.len() || self.cursor.is_some()
    }

    pub(crate) fn navigate_up(&mut self, current_text: &str) -> Option<HistoryEntry> {
        if self.entries.is_empty() {
            return None;
        }
        if self.cursor.is_none() {
            self.draft_before_navigation = Some(current_text.to_string());
            self.cursor = Some(self.entries.len() - 1);
        } else if let Some(cursor) = self.cursor
            && cursor > 0
        {
            self.cursor = Some(cursor - 1);
        }
        self.cursor.and_then(|index| self.entries.get(index).cloned())
    }

    pub(crate) fn navigate_down(&mut self) -> Option<HistoryEntry> {
        let cursor = self.cursor?;
        if cursor + 1 < self.entries.len() {
            self.cursor = Some(cursor + 1);
            return self.entries.get(cursor + 1).cloned();
        }
        let draft = self.draft_before_navigation.take().unwrap_or_default();
        self.reset_navigation();
        Some(HistoryEntry::new(draft))
    }

    pub(crate) fn reset_navigation(&mut self) {
        self.cursor = None;
        self.draft_before_navigation = None;
        self.search_query = None;
        self.search_index = None;
    }

    pub(crate) fn search(
        &mut self,
        query: &str,
        direction: HistorySearchDirection,
        restart: bool,
    ) -> HistorySearchResult {
        if self.entries.is_empty() {
            return HistorySearchResult::NotFound;
        }
        if restart || self.search_query.as_deref() != Some(query) {
            self.search_query = Some(query.to_string());
            self.search_index = match direction {
                HistorySearchDirection::Older => Some(self.entries.len()),
                HistorySearchDirection::Newer => None,
            };
        }

        let next = match direction {
            HistorySearchDirection::Older => {
                self.search_index.unwrap_or(self.entries.len()).checked_sub(1)
            }
            HistorySearchDirection::Newer => self
                .search_index
                .and_then(|index| (index + 1 < self.entries.len()).then_some(index + 1)),
        };
        let Some(mut index) = next else {
            return HistorySearchResult::AtBoundary;
        };

        let query = query.to_lowercase();
        loop {
            if self.entries[index].text.to_lowercase().contains(&query) {
                self.search_index = Some(index);
                return HistorySearchResult::Found(self.entries[index].clone());
            }
            let next_index = match direction {
                HistorySearchDirection::Older => index.checked_sub(1),
                HistorySearchDirection::Newer => {
                    (index + 1 < self.entries.len()).then_some(index + 1)
                }
            };
            let Some(next_index) = next_index else {
                return if self.search_index.is_some() {
                    HistorySearchResult::AtBoundary
                } else {
                    HistorySearchResult::NotFound
                };
            };
            index = next_index;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 验证 `local_history_recalls_unicode_drafts_without_duplicate_submissions` 场景的触发条件与预期结果。
    #[test]
    fn local_history_recalls_unicode_drafts_without_duplicate_submissions() {
        let mut history = ChatComposerHistory::new();
        let unicode = HistoryEntry::new("你好 👩‍💻\n组合 e\u{301}");
        history.record_local_submission(unicode.clone());
        history.record_local_submission(unicode.clone());

        assert_eq!(history.entries(), std::slice::from_ref(&unicode));
        assert_eq!(history.navigate_up("当前草稿"), Some(unicode));
        assert_eq!(history.navigate_down(), Some(HistoryEntry::new("当前草稿")));
    }

    // 验证 `reverse_search_walks_local_history_and_reports_boundaries` 场景的触发条件与预期结果。
    #[test]
    fn reverse_search_walks_local_history_and_reports_boundaries() {
        let mut history = ChatComposerHistory::new();
        history.record_local_submission(HistoryEntry::new("first"));
        history.record_local_submission(HistoryEntry::new("second unicode 你好"));
        history.record_local_submission(HistoryEntry::new("third"));

        assert_eq!(
            history.search("unicode", HistorySearchDirection::Older, true),
            HistorySearchResult::Found(HistoryEntry::new("second unicode 你好"))
        );
        assert_eq!(
            history.search("unicode", HistorySearchDirection::Older, false),
            HistorySearchResult::AtBoundary
        );
    }
}
