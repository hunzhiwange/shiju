//! Git 消息处理模块
//!
//! 本模块负责处理所有与 Git 相关的 UI 消息和操作，包括：
//! - 变更文件的刷新与展示
//! - Diff 视图的交互（选择、高亮、滚动、评论）
//! - 文件暂存与提交
//! - 代码丢弃与恢复
//! - 自定义 Diff 对比
//! - 复制模式与聊天集成
//!
//! 该模块是 VibeWindow 应用中 Git 面板功能的核心消息处理器。

use crate::app::{
    App, DiffTheme, Message,
    state::{ConventionalCommitType, GitWorktreeOption},
};
use iced::Task;
use iced::widget::text_editor;

mod diff;
mod modal;
mod refresh;
mod shared;
mod stage_commit;

/// Diff 上下文展开方向
///
/// 用于控制 Diff 视图中折叠上下文的展开方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpandDirection {
    /// 向下展开更多上下文行
    Up,
    /// 向上展开更多上下文行
    Down,
    /// 展开所有上下文
    All,
}

/// Git 操作消息枚举
///
/// 定义了所有与 Git 相关的 UI 交互消息，涵盖文件变更、Diff 视图、
/// 暂存区、提交、过滤等多个功能领域。
#[derive(Debug, Clone)]
pub enum GitMessage {
    /// 刷新 Git 面板所需的全部数据
    RefreshGitPanelData,

    /// 从网关刷新 Git 面板可切换的 worktree 列表
    RefreshWorktreeOptions,

    /// Git 面板 worktree 列表已就绪
    WorktreeOptionsReady {
        project_path: String,
        result: Result<Vec<GitWorktreeOption>, String>,
    },

    /// 切换 Git 面板当前操作的 worktree
    SelectGitWorktree(GitWorktreeOption),

    /// 打开或关闭 Git 面板 worktree 菜单
    ToggleGitWorktreeMenu(bool),

    /// 刷新变更文件列表
    ///
    /// 触发异步获取当前仓库的变更文件列表。
    RefreshChangedFiles,

    /// 变更文件列表已就绪
    ///
    /// 异步获取变更文件完成后的回调，携带文件路径列表。
    ChangedFilesReady {
        repo_path: Option<String>,
        files: Vec<String>,
    },

    /// 刷新 diff 文件元数据缓存
    RefreshDiffFileMetas,

    /// Diff 文件元数据已就绪
    DiffFileMetasReady {
        repo_path: Option<String>,
        metas: Vec<crate::app::components::git_panel::DiffFileMeta>,
    },

    LoadDiffContent(String),

    DiffContentReady {
        repo_path: Option<String>,
        file: String,
        old_content: String,
        new_content: String,
    },

    /// 切换 Diff 高亮显示
    ToggleDiffHighlight(bool),

    /// 切换 Git Diff 全屏状态
    ToggleFullscreen,

    /// 切换 Git Diff 半全屏状态
    ToggleHalfFullscreen,

    /// 切换单行选择状态
    ///
    /// 参数：文件路径、行号、是否为旧版本、行文本内容
    ToggleDiffLineSelection(String, usize, bool, String),

    /// 开始拖拽选择
    ///
    /// 用户开始拖拽选择多行时的初始事件。
    DiffDragSelectStart(String, usize, bool, String),

    /// 拖拽选择悬停
    ///
    /// 拖拽过程中鼠标悬停到的行。
    DiffDragSelectHover(String, usize, bool),

    /// 结束拖拽选择
    DiffDragSelectEnd,

    /// 打开 Diff 行右键菜单
    OpenDiffContextMenu {
        file: String,
        line: usize,
        is_old: bool,
        text: String,
        x: f32,
        y: f32,
    },

    /// 关闭 Diff 行右键菜单
    CloseDiffContextMenu,

    /// 打开文件级操作菜单
    OpenDiffFileMenu(String),

    /// 关闭文件级操作菜单
    CloseDiffFileMenu,

    /// 预览当前 Diff 文件
    PreviewDiffFile(String),

    /// 复制当前 Diff 文件内容
    CopyDiffFile {
        file: String,
        deleted_content: Option<String>,
    },

    /// 回滚当前 Diff 文件
    RevertDiffFile(String),

    /// 将当前选区作为评论草稿打开
    OpenDiffCommentDraft,

    /// 批量选择右键所在选区的行级暂存
    SelectDiffContextStageLines,

    /// 批量取消右键所在选区的行级暂存
    ClearDiffContextStageLines,

    /// 丢弃当前选区的更改
    DiscardDiffSelection,

    /// 将当前选区直接添加到会话输入
    InsertDiffSelectionToChat,

    /// 鼠标进入 Diff 行
    DiffHoverEnter(String, usize, bool),

    /// 鼠标离开 Diff 行
    DiffHoverExit(String, usize, bool),

    /// Diff 评论编辑器操作
    DiffCommentEditorAction(text_editor::Action),

    /// 取消 Diff 评论
    DiffCommentCancel,

    /// 提交 Diff 评论
    DiffCommentSubmit,

    /// 复制选中的 Diff 内容
    CopyDiffSelection,

    /// 打开 Diff 复制模式
    ///
    /// 打开指定文件的完整 Diff 内容到复制模态框。
    OpenDiffCopyMode(String),

    /// 使用指定文本打开复制模态框
    OpenCopyModalWithText(String),

    /// 从文件路径打开复制模态框
    OpenCopyModalFromPath(String),

    /// 关闭复制模态框
    CloseCopyModal,

    /// 切换复制模态框语法高亮
    ToggleCopyModalColored(bool),

    /// 复制模态框编辑器操作
    CopyModalEditorAction(text_editor::Action),

    /// 复制模态框代码编辑器事件
    CopyModalCodeEditorEvent(iced_code_editor::Message),

    /// 复制当前复制模态框内容
    CopyModalCopyCurrent,

    /// 将当前复制模态框内容插入聊天
    InsertCopyModalToChatCurrent,

    /// 将指定文本插入聊天
    InsertCopyModalToChat(String),

    /// 打开自定义 Diff 模态框
    OpenCustomDiffModal,

    /// 关闭自定义 Diff 模态框
    CloseCustomDiffModal,

    /// 自定义 Diff 标题变更
    CustomDiffTitleChanged(String),

    /// 自定义 Diff 前置内容编辑器操作
    CustomDiffBeforeEditorAction(text_editor::Action),

    /// 自定义 Diff 后置内容编辑器操作
    CustomDiffAfterEditorAction(text_editor::Action),

    /// 交换自定义 Diff 前后内容
    CustomDiffSwap,

    /// 打开自定义 Diff 结果
    ///
    /// 参数：标题、前置内容、后置内容
    OpenCustomDiffResult {
        title: String,
        before: String,
        after: String,
    },

    /// 打开聊天文本 Diff 视图
    ///
    /// 用于在聊天中展示代码变更对比。
    OpenChatTextDiff {
        title: String,
        file: String,
        before: String,
        after: String,
    },

    /// 关闭聊天文本 Diff 视图
    CloseChatTextDiff,

    /// 插入 Diff 选择评论到聊天
    InsertDiffSelectionComment,

    /// 确认丢弃文件
    ///
    /// 显示丢弃文件的确认对话框。
    ConfirmDiscardFile(String),

    /// 取消丢弃文件
    CancelDiscardFile,

    /// 执行丢弃文件
    DiscardFile(String),

    /// 丢弃指定 Hunk
    DiscardHunk(String, usize),

    /// 恢复删除行
    RevertLineDelete(String, usize),

    /// 恢复保留行
    RevertLineRestore(String, usize, usize),

    /// 切换 Hunk 展开/折叠
    ToggleExpandHunk(String, usize),

    /// 切换文件展开/折叠
    ToggleExpandFile(String),

    /// 聚焦到指定文件
    FocusFile(String),

    DiffScrollChanged {
        offset_y: f32,
        viewport_h: f32,
    },

    /// 选择 Diff 主题
    DiffThemeSelected(DiffTheme),

    /// 展开上下文
    ///
    /// 展开指定文件和间隙索引的上下文内容。
    ExpandContext(String, usize, ExpandDirection),

    /// 提交消息变更
    CommitMessageChanged(String),

    /// 提交类型选择
    CommitTypeSelected(ConventionalCommitType),

    /// 提交作用域变更
    CommitScopeChanged(String),

    /// 提交描述变更
    CommitDescriptionChanged(String),

    /// 提交描述编辑器操作
    CommitDescriptionEditorAction(text_editor::Action),

    /// 打开约定式提交帮助弹窗
    CommitHelpOpen,

    /// 关闭约定式提交帮助弹窗
    CommitHelpClose,

    /// 打开过滤选项帮助弹窗
    FilterHelpOpen,

    /// 关闭过滤选项帮助弹窗
    FilterHelpClose,

    /// 切换过滤选项面板
    ToggleFilterOptions(bool),

    /// 过滤查询变更
    FilterQueryChanged(String),

    /// 切换"已暂存"过滤
    FilterToggleIncluded(bool),

    /// 切换"未暂存"过滤
    FilterToggleExcluded(bool),

    /// 切换"新增文件"过滤
    FilterToggleNew(bool),

    /// 切换"修改文件"过滤
    FilterToggleModified(bool),

    /// 切换"删除文件"过滤
    FilterToggleDeleted(bool),

    /// 清除所有过滤器
    ClearFilters,

    /// 切换文件暂存状态
    ToggleStageFile(String, bool),

    /// 切换 Hunk 暂存状态
    ToggleStageHunk(String, usize, bool),

    /// 切换新版本行暂存状态
    ToggleStageLine(String, usize, bool),

    /// 切换旧版本行暂存状态
    ToggleStageOldLine(String, usize, bool),

    /// 选中文件中的所有变更行
    SelectAllFileLines(String),

    /// 选中当前列表中的所有变更行
    SelectAllVisibleFileLines(Vec<String>),

    /// 反选当前列表中的所有变更行
    InvertVisibleFileLines(Vec<String>),

    /// 取消选中文件中的所有变更行
    ClearAllFileLines(String),

    /// 鼠标进入文件标题头
    HoverFileHeaderEnter(String),

    /// 鼠标离开文件标题头
    HoverFileHeaderExit(String),

    /// 鼠标进入 Git 面板标题头
    HoverGitPanelHeaderEnter,

    /// 鼠标离开 Git 面板标题头
    HoverGitPanelHeaderExit,

    /// 执行提交
    ///
    /// 将所有已暂存的文件、Hunk、行进行 Git 提交。
    CommitSelected,

    /// 暂存当前工作树中的全部变更
    StageAll,

    /// 暂存全部变更已完成
    StageAllFinished(Result<(), String>),

    /// 暂存并提交当前工作树中的全部变更
    CommitAll,

    /// 选择提交已完成
    CommitSelectedFinished(Result<(), String>),
}

#[derive(Debug, Clone)]
struct SelectedCommitRequest {
    message: String,
    stage_all: bool,
    selected_files: Vec<String>,
    selected_hunks: Vec<(String, usize)>,
    selected_lines: Vec<(String, usize)>,
    selected_old_lines: Vec<(String, usize)>,
}

/// 处理 Git 消息并更新应用状态
///
/// 这是 Git 消息处理的核心函数，根据接收到的消息类型分发到对应职责模块。
pub fn update(app: &mut App, message: GitMessage) -> Task<Message> {
    match message {
        message @ (GitMessage::RefreshGitPanelData
        | GitMessage::RefreshWorktreeOptions
        | GitMessage::WorktreeOptionsReady { .. }
        | GitMessage::SelectGitWorktree(_)
        | GitMessage::ToggleGitWorktreeMenu(_)
        | GitMessage::RefreshChangedFiles
        | GitMessage::ChangedFilesReady { .. }
        | GitMessage::RefreshDiffFileMetas
        | GitMessage::DiffFileMetasReady { .. }
        | GitMessage::LoadDiffContent(_)
        | GitMessage::DiffContentReady { .. }) => refresh::update(app, message),
        message @ (GitMessage::ToggleDiffHighlight(_)
        | GitMessage::ToggleFullscreen
        | GitMessage::ToggleHalfFullscreen
        | GitMessage::ToggleDiffLineSelection(_, _, _, _)
        | GitMessage::DiffDragSelectStart(_, _, _, _)
        | GitMessage::DiffDragSelectHover(_, _, _)
        | GitMessage::DiffDragSelectEnd
        | GitMessage::OpenDiffContextMenu { .. }
        | GitMessage::CloseDiffContextMenu
        | GitMessage::OpenDiffFileMenu(_)
        | GitMessage::CloseDiffFileMenu
        | GitMessage::PreviewDiffFile(_)
        | GitMessage::CopyDiffFile { .. }
        | GitMessage::RevertDiffFile(_)
        | GitMessage::OpenDiffCommentDraft
        | GitMessage::SelectDiffContextStageLines
        | GitMessage::ClearDiffContextStageLines
        | GitMessage::DiscardDiffSelection
        | GitMessage::InsertDiffSelectionToChat
        | GitMessage::DiffHoverEnter(_, _, _)
        | GitMessage::DiffHoverExit(_, _, _)
        | GitMessage::DiffCommentEditorAction(_)
        | GitMessage::DiffCommentCancel
        | GitMessage::DiffCommentSubmit
        | GitMessage::CopyDiffSelection
        | GitMessage::InsertDiffSelectionComment
        | GitMessage::ConfirmDiscardFile(_)
        | GitMessage::CancelDiscardFile
        | GitMessage::DiscardFile(_)
        | GitMessage::DiscardHunk(_, _)
        | GitMessage::RevertLineDelete(_, _)
        | GitMessage::RevertLineRestore(_, _, _)
        | GitMessage::ToggleExpandHunk(_, _)
        | GitMessage::ToggleExpandFile(_)
        | GitMessage::FocusFile(_)
        | GitMessage::DiffScrollChanged { .. }
        | GitMessage::DiffThemeSelected(_)
        | GitMessage::ExpandContext(_, _, _)) => diff::update(app, message),
        message @ (GitMessage::OpenDiffCopyMode(_)
        | GitMessage::OpenCopyModalWithText(_)
        | GitMessage::OpenCopyModalFromPath(_)
        | GitMessage::CloseCopyModal
        | GitMessage::ToggleCopyModalColored(_)
        | GitMessage::CopyModalEditorAction(_)
        | GitMessage::CopyModalCodeEditorEvent(_)
        | GitMessage::CopyModalCopyCurrent
        | GitMessage::InsertCopyModalToChatCurrent
        | GitMessage::InsertCopyModalToChat(_)
        | GitMessage::OpenCustomDiffModal
        | GitMessage::CloseCustomDiffModal
        | GitMessage::CustomDiffTitleChanged(_)
        | GitMessage::CustomDiffBeforeEditorAction(_)
        | GitMessage::CustomDiffAfterEditorAction(_)
        | GitMessage::CustomDiffSwap
        | GitMessage::OpenCustomDiffResult { .. }
        | GitMessage::OpenChatTextDiff { .. }
        | GitMessage::CloseChatTextDiff) => modal::update(app, message),
        message @ (GitMessage::CommitMessageChanged(_)
        | GitMessage::CommitTypeSelected(_)
        | GitMessage::CommitScopeChanged(_)
        | GitMessage::CommitDescriptionChanged(_)
        | GitMessage::CommitDescriptionEditorAction(_)
        | GitMessage::CommitHelpOpen
        | GitMessage::CommitHelpClose
        | GitMessage::FilterHelpOpen
        | GitMessage::FilterHelpClose
        | GitMessage::ToggleFilterOptions(_)
        | GitMessage::FilterQueryChanged(_)
        | GitMessage::FilterToggleIncluded(_)
        | GitMessage::FilterToggleExcluded(_)
        | GitMessage::FilterToggleNew(_)
        | GitMessage::FilterToggleModified(_)
        | GitMessage::FilterToggleDeleted(_)
        | GitMessage::ClearFilters
        | GitMessage::ToggleStageFile(_, _)
        | GitMessage::ToggleStageHunk(_, _, _)
        | GitMessage::ToggleStageLine(_, _, _)
        | GitMessage::ToggleStageOldLine(_, _, _)
        | GitMessage::SelectAllFileLines(_)
        | GitMessage::SelectAllVisibleFileLines(_)
        | GitMessage::InvertVisibleFileLines(_)
        | GitMessage::ClearAllFileLines(_)
        | GitMessage::HoverFileHeaderEnter(_)
        | GitMessage::HoverFileHeaderExit(_)
        | GitMessage::HoverGitPanelHeaderEnter
        | GitMessage::HoverGitPanelHeaderExit
        | GitMessage::CommitSelected
        | GitMessage::StageAll
        | GitMessage::StageAllFinished(_)
        | GitMessage::CommitAll
        | GitMessage::CommitSelectedFinished(_)) => stage_commit::update(app, message),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "diff_tests.rs"]
mod diff_tests;

#[cfg(test)]
#[path = "modal_tests.rs"]
mod modal_tests;

#[cfg(test)]
#[path = "refresh_tests.rs"]
mod refresh_tests;

#[cfg(test)]
#[path = "shared_tests.rs"]
mod shared_tests;

#[cfg(test)]
#[path = "stage_commit_tests.rs"]
mod stage_commit_tests;
