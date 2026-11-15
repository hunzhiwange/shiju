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
#[path = "header_tests.rs"]
mod header_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
mod tests;

pub use file_view::view_file;
pub use text_diff::view_custom_text_diff;

pub(crate) use comment_editor::diff_comment_editor;
pub(super) use context_menu::wrap_diff_row_with_context_menu;
pub(super) use styles::{
    diff_highlight_enabled, diff_split_divider, diff_split_pane,
    diff_split_pane_with_background, diff_split_row, merge_diff_row,
    split_line_number_area,
};

pub(super) use desktop_git_ui::git_panel::diff_view::{DiffRenderCtx, DiffSplitPaneTone, DIFF_LINE_NUMBER_WIDTH};
    pub(crate) fn diff_render_ctx(app: &App) -> DiffRenderCtx<'_> {
        DiffRenderCtx {
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


pub(crate) fn diff_view_state(app: &App) -> desktop_git_ui::git_panel::diff_view::DiffViewState<'_> {
    desktop_git_ui::git_panel::diff_view::DiffViewState {
        merge_view: app.merge_view, desktop_locale: app.desktop_locale,
        expanded_hunks: &app.expanded_hunks, context_expansions: &app.context_expansions,
        active_range: app.git_diff_drag_range.as_ref().or(app.git_diff_selected_range.as_ref()).or_else(|| app.git_diff_comment_draft.as_ref().map(|draft| &draft.range)),
        hovered_line: app.git_diff_hovered_line.as_ref(), context_menu: app.git_diff_context_menu.as_ref(),
    }
}

fn map_diff_message(event: desktop_git_ui::git_panel::diff_view::DiffMessage) -> crate::app::Message {
    use desktop_git_ui::git_panel::diff_view::DiffMessage as Local;
    use crate::app::{Message, message::GitMessage};
    Message::Git(match event {
        Local::PreviewDiffFile(file) => GitMessage::PreviewDiffFile(file),
        Local::CopyDiffFile { file, deleted_content } => GitMessage::CopyDiffFile { file, deleted_content },
        Local::RevertDiffFile(file) => GitMessage::RevertDiffFile(file),
        Local::OpenDiffFileMenu(file) => GitMessage::OpenDiffFileMenu(file),
        Local::CloseDiffFileMenu => GitMessage::CloseDiffFileMenu,
        Local::SelectAllFileLines(file) => GitMessage::SelectAllFileLines(file),
        Local::ClearAllFileLines(file) => GitMessage::ClearAllFileLines(file),
        Local::ToggleExpandFile(file) => GitMessage::ToggleExpandFile(file),
        Local::DiffDragSelectStart(file, line, old, text) => GitMessage::DiffDragSelectStart(file, line, old, text),
        Local::DiffDragSelectHover(file, line, old) => GitMessage::DiffDragSelectHover(file, line, old),
        Local::DiffHoverEnter(file, line, old) => GitMessage::DiffHoverEnter(file, line, old),
        Local::DiffHoverExit(file, line, old) => GitMessage::DiffHoverExit(file, line, old),
        Local::ToggleStageLine(file, line, selected) => GitMessage::ToggleStageLine(file, line, selected),
        Local::ToggleStageOldLine(file, line, selected) => GitMessage::ToggleStageOldLine(file, line, selected),
        Local::ExpandContext(file, gap, direction) => GitMessage::ExpandContext(file, gap, match direction {
            desktop_git_ui::git_panel::diff_view::ExpandDirection::Up => crate::app::message::git::ExpandDirection::Up,
            desktop_git_ui::git_panel::diff_view::ExpandDirection::Down => crate::app::message::git::ExpandDirection::Down,
            desktop_git_ui::git_panel::diff_view::ExpandDirection::All => crate::app::message::git::ExpandDirection::All,
        }),
        Local::DiffCommentCancel => GitMessage::DiffCommentCancel,
        Local::DiffCommentSubmit => GitMessage::DiffCommentSubmit,
        Local::OpenDiffCommentDraft => GitMessage::OpenDiffCommentDraft,
        Local::SelectDiffContextStageLines => GitMessage::SelectDiffContextStageLines,
        Local::ClearDiffContextStageLines => GitMessage::ClearDiffContextStageLines,
        Local::DiscardDiffSelection => GitMessage::DiscardDiffSelection,
        Local::CopyDiffSelection => GitMessage::CopyDiffSelection,
        Local::InsertDiffSelectionToChat => GitMessage::InsertDiffSelectionToChat,
        Local::CloseDiffContextMenu => GitMessage::CloseDiffContextMenu,
        Local::DiffCommentEditorAction(action) => GitMessage::DiffCommentEditorAction(action),
        Local::OpenDiffContextMenu {file, line, is_old, text, x, y} => GitMessage::OpenDiffContextMenu {file, line, is_old, text, x, y},
    })
}

#[cfg(test)]
use desktop_git_ui::git_panel::diff_view::{DIFF_MARKER_WIDTH, DIFF_SPLIT_DIVIDER_WIDTH};
