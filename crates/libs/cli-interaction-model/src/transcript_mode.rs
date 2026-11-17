use std::path::PathBuf;

/// 退出阅读模式时需要恢复的最小滚动状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranscriptScrollRestore {
    pub top_message: usize,
    pub follow_tail: bool,
}

/// 阅读模式交给 app 边界执行的显式副作用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptEffectIntent {
    Dumped { path: PathBuf },
    OpenExternalEditor { path: PathBuf },
}

/// Ctrl+O transcript 阅读模式的独立状态。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TranscriptModeState {
    pub active: bool,
    pub lines: Vec<String>,
    pub top_line: usize,
    pub viewport_rows: usize,
    pub show_all: bool,
    pub search_bar_open: bool,
    pub search_draft: String,
    pub search_query: String,
    pub search_matches: Vec<usize>,
    pub selected_match: Option<usize>,
    pub search_restore_top: usize,
    pub restore: Option<TranscriptScrollRestore>,
}

impl TranscriptModeState {
    #[cfg(test)]
    pub(crate) fn visible_lines(&self) -> &[String] {
        let start = self.top_line.min(self.lines.len());
        let end = start.saturating_add(self.viewport_rows.max(1)).min(self.lines.len());
        &self.lines[start..end]
    }

    #[cfg(test)]
    pub(crate) fn max_top_line(&self) -> usize {
        self.lines.len().saturating_sub(self.viewport_rows.max(1))
    }

    #[cfg(test)]
    pub(crate) fn clamp_top_line(&mut self) {
        self.top_line = self.top_line.min(self.max_top_line());
    }
}
