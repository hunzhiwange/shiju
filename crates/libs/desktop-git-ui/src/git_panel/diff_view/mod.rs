use iced::widget::text_editor;
#[derive(Debug, Clone)]
pub struct GitDiffLineRange {
    /// 文件路径
    pub file: String,
    /// 起始行号
    pub start: usize,
    /// 结束行号
    pub end: usize,
    /// 是否为旧版本
    pub is_old: bool,
}
#[derive(Debug, Clone)]
pub struct GitDiffContextMenuState {
    /// 文件路径
    pub file: String,
    /// 行号
    pub line: usize,
    /// 是否为旧版本
    pub is_old: bool,
    /// 菜单锚点 X 坐标
    pub x: f32,
    /// 菜单锚点 Y 坐标
    pub y: f32,
}
#[derive(Debug, Clone)]
pub struct GitDiffCommentDraft {
    /// 评论的行范围
    pub range: GitDiffLineRange,
    /// 评论编辑器内容
    pub editor: text_editor::Content,
}
#[derive(Debug, Clone)]
pub enum DiffMessage {
    PreviewDiffFile(String),
    CopyDiffFile { file: String, deleted_content: Option<String> },
    RevertDiffFile(String),
    OpenDiffFileMenu(String),
    CloseDiffFileMenu,
    SelectAllFileLines(String),
    ClearAllFileLines(String),
    ToggleExpandFile(String),
    DiffDragSelectStart(String, usize, bool, String),
    DiffDragSelectHover(String, usize, bool),
    DiffHoverEnter(String, usize, bool),
    DiffHoverExit(String, usize, bool),
    ToggleStageLine(String, usize, bool),
    ToggleStageOldLine(String, usize, bool),
    ExpandContext(String, usize, ExpandDirection),

    DiffCommentEditorAction(text_editor::Action), DiffCommentCancel, DiffCommentSubmit,
    OpenDiffContextMenu { file: String, line: usize, is_old: bool, text: String, x: f32, y: f32 },
    OpenDiffCommentDraft, SelectDiffContextStageLines, ClearDiffContextStageLines,
    DiscardDiffSelection, CopyDiffSelection, InsertDiffSelectionToChat, CloseDiffContextMenu,
}
pub mod comment_editor;
pub mod context_menu;
#[cfg(test)]
mod comment_editor_tests;
#[cfg(test)]
mod context_menu_tests;

use std::collections::HashSet;
pub struct DiffRenderCtx<'a> {
    pub read_only: bool,
    pub history_file: Option<(usize, String)>,
    pub history_notice: Option<String>,
    pub selected_new_lines: HashSet<(&'a str, usize)>,
    pub selected_old_lines: HashSet<(&'a str, usize)>,
    pub selected_diff_lines: HashSet<(&'a str, usize, bool)>,
    pub selected_files: HashSet<&'a str>,
}

impl<'a> DiffRenderCtx<'a> {
    pub fn history(index: usize, key: String) -> Self {
        Self {
            read_only: true,
            history_file: Some((index, key)),
            history_notice: None,
            selected_new_lines: HashSet::new(),
            selected_old_lines: HashSet::new(),
            selected_diff_lines: HashSet::new(),
            selected_files: HashSet::new(),
        }
    }

    pub fn is_file_staged(&self, file: &str) -> bool {
        self.selected_files.contains(file)
    }

    pub fn is_new_line_staged(&self, file: &str, line: usize) -> bool {
        self.is_file_staged(file) || self.selected_new_lines.contains(&(file, line))
    }

    pub fn is_old_line_staged(&self, file: &str, line: usize) -> bool {
        self.is_file_staged(file) || self.selected_old_lines.contains(&(file, line))
    }

    pub fn is_diff_line_selected(&self, file: &str, line: usize, is_old: bool) -> bool {
        self.selected_diff_lines.contains(&(file, line, is_old))
    }
}

/// 差异标记列的宽度（像素）
///
/// 用于显示行状态标记（增加/删除/无变化）的列宽度
pub const DIFF_MARKER_WIDTH: f32 = 8.0;
pub const DIFF_LINE_NUMBER_WIDTH: f32 = 46.0;
pub const DIFF_SPLIT_DIVIDER_WIDTH: f32 = 8.0;

#[derive(Clone, Copy)]
pub enum DiffSplitPaneTone {
    Neutral,
    Add,
    Delete,
    Empty,
}

/// Borrowed inputs used only by diff row rendering. No application or effect access.
pub struct DiffViewState<'a> {
    pub merge_view: bool,
    pub desktop_locale: config_ui::DesktopLocale,
    pub expanded_hunks: &'a [(String, usize)],
    pub context_expansions: &'a std::collections::HashMap<(String, usize), (usize, usize)>,
    pub active_range: Option<&'a GitDiffLineRange>,
    pub hovered_line: Option<&'a (String, usize, bool)>,
    pub context_menu: Option<&'a GitDiffContextMenuState>,
}
pub fn wrap_diff_row_with_context_menu<'a>(state: &DiffViewState<'_>, file: &str, line: usize, old: bool, text: String, content: iced::Element<'a, DiffMessage>) -> iced::Element<'a, DiffMessage> {
    context_menu::wrap_diff_row_with_context_menu(state.desktop_locale, state.context_menu, file, line, old, text, content, |event| event)
}
pub mod markers;
pub mod styles;
pub mod selection;
pub mod file_view;
pub use styles::*;

#[derive(Debug, Clone, Copy)]
pub enum ExpandDirection { Up, Down, All }
#[cfg(test)]
mod test_support;

#[cfg(test)]
mod markers_tests;

#[cfg(test)]
mod styles_tests;

#[cfg(test)]
mod selection_tests;

pub mod header;
pub mod history;
#[cfg(test)]
mod header_tests;

pub mod text_diff;
#[cfg(test)]
mod text_diff_tests;
