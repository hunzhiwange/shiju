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

use crate::app::{App, Message};
use iced::Task;

mod diff;
mod modal;
mod refresh;
mod repository;
mod shared;
pub(crate) use shared::git_context_path_for_app;
mod stage_commit;

pub use desktop_git_ui::message_git::messages::{ExpandDirection, GitMessage};

use desktop_git_ui::message_git::shared::SelectedCommitRequest;

/// 处理 Git 消息并更新应用状态
///
/// 这是 Git 消息处理的核心函数，根据接收到的消息类型分发到对应职责模块。
pub fn update(app: &mut App, message: GitMessage) -> Task<Message> {
    match message {
        message @ (GitMessage::RemoteAnimationTick
        | GitMessage::ShowHistory(_)
        | GitMessage::RefreshRepository
        | GitMessage::LoadMoreHistory
        | GitMessage::RepositoryReady { .. }
        | GitMessage::SelectHistoryCommit(_)
        | GitMessage::HistoryDetailReady { .. }
        | GitMessage::OpenHistoryFileMenu(_)
        | GitMessage::CloseHistoryFileMenu
        | GitMessage::PreviewHistoryFile(_)
        | GitMessage::CopyHistoryFile(_)
        | GitMessage::SelectHistoryFile(_)
        | GitMessage::RunRemote(_)
        | GitMessage::RemoteFinished { .. }) => repository::update(app, message),
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod repository_tests;
