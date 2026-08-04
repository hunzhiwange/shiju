use std::path::PathBuf;

/// 退出阅读模式时需要恢复的最小滚动状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TranscriptScrollRestore {
    pub(crate) top_message: usize,
    pub(crate) follow_tail: bool,
}

/// 阅读模式交给 app 边界执行的显式副作用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TranscriptEffectIntent {
    Dumped { path: PathBuf },
    OpenExternalEditor { path: PathBuf },
}

/// Ctrl+O transcript 阅读模式的独立状态。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TranscriptModeState {
    pub(crate) active: bool,
    pub(crate) lines: Vec<String>,
    pub(crate) top_line: usize,
    pub(crate) viewport_rows: usize,
    pub(crate) show_all: bool,
    pub(crate) search_bar_open: bool,
    pub(crate) search_draft: String,
    pub(crate) search_query: String,
    pub(crate) search_matches: Vec<usize>,
    pub(crate) selected_match: Option<usize>,
    pub(crate) search_restore_top: usize,
    pub(crate) restore: Option<TranscriptScrollRestore>,
}

impl TranscriptModeState {
    pub(crate) fn visible_lines(&self) -> &[String] {
        let start = self.top_line.min(self.lines.len());
        let end = start.saturating_add(self.viewport_rows.max(1)).min(self.lines.len());
        &self.lines[start..end]
    }

    pub(crate) fn max_top_line(&self) -> usize {
        self.lines.len().saturating_sub(self.viewport_rows.max(1))
    }

    pub(crate) fn clamp_top_line(&mut self) {
        self.top_line = self.top_line.min(self.max_top_line());
    }
}
