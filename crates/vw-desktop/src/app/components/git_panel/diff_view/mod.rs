//! Git 差异对比视图模块
//!
//! 本模块提供了用于显示 Git 差异对比的自定义 UI 组件。支持两种显示模式：
//! - **合并视图（Merge View）**：将旧内容和新内容显示在同一列中
//! - **分离视图（Split View）**：左侧显示旧内容，右侧显示新内容
//!
//! # 核心功能
//!
//! - 显示文件内容的行级差异对比（增加、删除、未更改）
//! - 支持行选择、拖拽选择和右键菜单功能
//! - 提供悬停高亮效果
//! - 显示行号和差异标记
//! - 支持语法高亮和字符级高亮
//! - 统计插入和删除的行数
//!
//! # 子模块
//!
//! - `comment_editor`：差异评论编辑器
//! - `context_menu`：差异右键菜单与包装
//! - `file_view`：文件视图渲染
//! - `header`：差异对比头部信息
//! - `markers`：差异标记和行号显示
//! - `selection`：行选择状态管理
//! - `styles`：差异行与分栏样式辅助
//! - `text_diff`：自定义文本 diff 渲染

use std::collections::HashSet;

use crate::app::App;

mod comment_editor;
mod context_menu;
mod file_view;
mod header;
mod history;
pub(super) use history::view_history_diff;
mod markers;
mod selection;
mod styles;
mod text_diff;

#[cfg(test)]
#[path = "comment_editor_tests.rs"]
mod comment_editor_tests;
#[cfg(test)]
#[path = "context_menu_tests.rs"]
mod context_menu_tests;
#[cfg(test)]
#[path = "header_tests.rs"]
mod header_tests;
#[cfg(test)]
#[path = "markers_tests.rs"]
mod markers_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "selection_tests.rs"]
mod selection_tests;
#[cfg(test)]
#[path = "styles_tests.rs"]
mod styles_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "text_diff_tests.rs"]
mod text_diff_tests;

pub use file_view::view_file;
pub use text_diff::view_custom_text_diff;

pub(crate) use comment_editor::diff_comment_editor;
pub(super) use context_menu::wrap_diff_row_with_context_menu;
pub(super) use styles::{
    diff_highlight_enabled, diff_line_number_with_background, diff_split_divider, diff_split_pane,
    diff_split_pane_with_background, diff_split_row, merge_diff_row,
    merge_diff_row_with_background, split_line_number_area,
};

pub(super) struct DiffRenderCtx<'a> {
    read_only: bool,
    history_file: Option<(usize, String)>,
    history_notice: Option<String>,
    selected_new_lines: HashSet<(&'a str, usize)>,
    selected_old_lines: HashSet<(&'a str, usize)>,
    selected_diff_lines: HashSet<(&'a str, usize, bool)>,
    selected_files: HashSet<&'a str>,
}

impl<'a> DiffRenderCtx<'a> {
    pub(super) fn new(app: &'a App) -> Self {
        Self {
            read_only: false,
            history_file: None,
            history_notice: None,
            selected_new_lines: app
                .staged_lines_selected
                .iter()
                .map(|(file, line)| (file.as_str(), *line))
                .collect(),
            selected_old_lines: app
                .staged_old_lines_selected
                .iter()
                .map(|(file, line)| (file.as_str(), *line))
                .collect(),
            selected_diff_lines: app
                .git_diff_selected_lines
                .iter()
                .map(|line| (line.file.as_str(), line.line, line.is_old))
                .collect(),
            selected_files: app.staged_files_selected.iter().map(String::as_str).collect(),
        }
    }

    fn history(index: usize, key: String) -> Self {
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

    fn is_file_staged(&self, file: &str) -> bool {
        self.selected_files.contains(file)
    }

    fn is_new_line_staged(&self, file: &str, line: usize) -> bool {
        self.is_file_staged(file) || self.selected_new_lines.contains(&(file, line))
    }

    fn is_old_line_staged(&self, file: &str, line: usize) -> bool {
        self.is_file_staged(file) || self.selected_old_lines.contains(&(file, line))
    }

    fn is_diff_line_selected(&self, file: &str, line: usize, is_old: bool) -> bool {
        self.selected_diff_lines.contains(&(file, line, is_old))
    }
}

/// 差异标记列的宽度（像素）
///
/// 用于显示行状态标记（增加/删除/无变化）的列宽度
const DIFF_MARKER_WIDTH: f32 = 8.0;
pub(super) const DIFF_LINE_NUMBER_WIDTH: f32 = 46.0;
const DIFF_SPLIT_DIVIDER_WIDTH: f32 = 8.0;

#[derive(Clone, Copy)]
pub(super) enum DiffSplitPaneTone {
    Neutral,
    Add,
    Delete,
    Empty,
}
