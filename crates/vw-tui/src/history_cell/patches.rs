//! 补丁历史只展示外层提供的有序文件差异，不读取或应用文件。
//! Patch history renders ordered file diffs supplied by the outer layer without file access.

use ratatui::text::Line;

use super::HistoryCell;
use crate::diff_render::create_diff_summary;
use crate::vibe::event::V3FileDiff;

#[derive(Debug)]
pub(crate) struct PatchHistoryCell {
    diff: V3FileDiff,
}

impl HistoryCell for PatchHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        create_diff_summary(&self.diff.files, width as usize)
    }
}

impl PatchHistoryCell {
    pub(crate) fn new(diff: V3FileDiff) -> Self {
        Self { diff }
    }
}
