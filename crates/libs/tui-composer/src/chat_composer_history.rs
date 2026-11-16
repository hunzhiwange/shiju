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

use std::path::PathBuf;
use tui_contracts::event::V3Attachment;
use tui_contracts::presentation::MentionBinding;
use tui_input::input_types::TextElement;
use tui_input::textarea::TextAreaDraft;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryEntry {
    pub text: String,
    pub attachments: Vec<V3Attachment>,
    pub draft: Option<TextAreaDraft>,
    pub text_elements: Vec<TextElement>,
    pub local_image_paths: Vec<PathBuf>,
    pub remote_image_urls: Vec<String>,
    pub mention_bindings: Vec<MentionBinding>,
    pub pending_pastes: Vec<(String, String)>,
}

impl HistoryEntry {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            attachments: Vec::new(),
            draft: None,
            text_elements: Vec::new(),
            local_image_paths: Vec::new(),
            remote_image_urls: Vec::new(),
            mention_bindings: Vec::new(),
            pending_pastes: Vec::new(),
        }
    }

    pub fn with_draft_state(
        text: impl Into<String>,
        text_elements: Vec<TextElement>,
        pending_pastes: Vec<(String, String)>,
    ) -> Self {
        Self {
            text: text.into(),
            attachments: Vec::new(),
            draft: None,
            text_elements,
            local_image_paths: Vec::new(),
            remote_image_urls: Vec::new(),
            mention_bindings: Vec::new(),
            pending_pastes,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistorySearchDirection {
    Older,
    Newer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistorySearchResult {
    Found(HistoryEntry),
    AtBoundary,
    NotFound,
}

/// 编辑器在会话桥接前使用的内存式历史
/// In-memory shell-style history used by the editor before a session bridge is attached.
#[derive(Debug, Default)]
pub struct ChatComposerHistory {
    entries: Vec<HistoryEntry>,
    cursor: Option<usize>,
    draft_before_navigation: Option<HistoryEntry>,
    search_query: Option<String>,
    search_index: Option<usize>,
}

impl ChatComposerHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    pub fn record_local_submission(&mut self, entry: HistoryEntry) {
        self.reset_navigation();
        if (entry.text.is_empty() && entry.attachments.is_empty())
            || self.entries.last() == Some(&entry)
        {
            return;
        }
        self.entries.push(entry);
    }

    pub fn should_handle_navigation(&self, text: &str, cursor: usize) -> bool {
        cursor == 0 || cursor == text.len() || self.cursor.is_some()
    }

    pub fn navigate_up(&mut self, current_text: &str) -> Option<HistoryEntry> {
        self.navigate_up_with_draft(HistoryEntry::new(current_text))
    }

    pub fn navigate_up_with_draft(&mut self, draft: HistoryEntry) -> Option<HistoryEntry> {
        if self.entries.is_empty() {
            return None;
        }
        if self.cursor.is_none() {
            self.draft_before_navigation = Some(draft);
            self.cursor = Some(self.entries.len() - 1);
        } else if let Some(cursor) = self.cursor
            && cursor > 0
        {
            self.cursor = Some(cursor - 1);
        }
        self.cursor.and_then(|index| self.entries.get(index).cloned())
    }

    pub fn navigate_down(&mut self) -> Option<HistoryEntry> {
        let cursor = self.cursor?;
        if cursor + 1 < self.entries.len() {
            self.cursor = Some(cursor + 1);
            return self.entries.get(cursor + 1).cloned();
        }
        let draft = self.draft_before_navigation.take().unwrap_or_else(|| HistoryEntry::new(""));
        self.reset_navigation();
        Some(draft)
    }

    pub fn reset_navigation(&mut self) {
        self.cursor = None;
        self.draft_before_navigation = None;
        self.search_query = None;
        self.search_index = None;
    }

    pub fn search(
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
#[path = "chat_composer_history_tests.rs"]
mod tests;
