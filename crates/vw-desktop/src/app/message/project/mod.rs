//! 汇总项目消息处理子模块，并分发项目、会话和配置相关消息。
//!
//! 注释聚焦模块职责、消息边界和失败处理方式，帮助维护者在不改变逻辑的前提下理解代码。

use crate::app::components::color_picker::ColorFormat;
use crate::app::{
    App, Message,
    state::{FindInFolderMatch, ProjectEditTab},
};
use iced::Task;
use iced::widget::text_editor;
use vw_shared::message::types as agent_message;

mod basic;
mod file_tree;
mod find;
pub(crate) mod helpers;
mod session;

#[cfg(target_arch = "wasm32")]
pub(crate) use helpers::refresh_file_index;
pub(crate) use helpers::{prepare_session_ui_chunks_task, prepare_session_ui_task};
pub(crate) use session::loaded_chat_from_gateway_messages;

#[derive(Debug, Clone)]
/// LoadedProjectInfo 保存该流程中跨函数传递的结构化数据。
///
/// 使用具名字段保留领域含义，避免在消息链路中传递松散的动态数据。
pub struct LoadedProjectInfo {
    pub project_path: String,
    pub info: vw_shared::project::Info,
    pub current_branch: Option<String>,
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
/// FileTreeAction 表示该流程中可枚举的状态或用户动作。
///
/// 变体与界面事件或后台任务结果保持对应，便于在消息分发时显式匹配。
pub enum FileTreeAction {
    Open,
    RevealInFinder,
    OpenInTerminal,
    AddToChat,
    FindInFolder,
    Cut,
    Copy,
    Paste,
    CopyPath,
    CopyRelativePath,
    Rename,
    Delete,
}

#[derive(Debug, Clone)]
/// ProjectMessage 表示该流程中可枚举的状态或用户动作。
///
/// 变体与界面事件或后台任务结果保持对应，便于在消息分发时显式匹配。
pub enum ProjectMessage {
    OpenFolderPressed,
    OpenProjectPressed,
    OpenRecentPressed(String),
    SwitchWorkSpacePressed(String),
    OpenProjectSessionPressed(String, String),
    RecentHovered(Option<String>),
    RecentOverlayClosed,
    ProjectToolsMenuToggled(String),
    ProjectToolsMenuClosed,
    ProjectEditOpened(String),
    ProjectEditTabSelected(ProjectEditTab),
    ProjectEditNameChanged(String),
    ProjectEditIconChanged(String),
    ProjectEditIconHovered(bool),
    ProjectEditIconPickFile,
    ProjectEditIconFilePicked(Option<String>),
    ProjectEditIconColorChanged(String),
    ProjectEditIconColorPresetSelected(String),
    ProjectEditIconColorPickerToggled,
    ProjectEditIconColorPickerClosed,
    ProjectEditIconColorFormatChanged(ColorFormat),
    ProjectEditStartScriptChanged(String),
    ProjectEditStartScriptEditorAction(text_editor::Action),
    ProjectEditWorktreeToggled(bool),
    ProjectEditTaskBoardAutoRefreshToggled(bool),
    ProjectEditSessionAutoRefreshToggled(bool),
    ProjectEditMaxConcurrentInputChanged(String),
    ProjectEditSessionRefreshIntervalSecondsInputChanged(String),
    ProjectEditTaskBoardRefreshIntervalSecondsInputChanged(String),
    ProjectEditTaskBoardSchedulerTickIntervalSecondsInputChanged(String),
    ProjectEditTaskBoardAutoPromoteTickIntervalSecondsInputChanged(String),
    ProjectEditFailedRetryMinutesInputChanged(String),
    ProjectEditFailedRetryMaxAttemptsInputChanged(String),
    ProjectEditRunningTimeoutMinutesInputChanged(String),
    ProjectEditPrSubmittedStallTimeoutSecondsInputChanged(String),
    ProjectEditMaxConcurrentChanged(u32),
    ProjectEditSessionRefreshIntervalSecondsChanged(u64),
    ProjectEditTaskBoardRefreshIntervalSecondsChanged(u64),
    ProjectEditTaskBoardSchedulerTickIntervalSecondsChanged(u64),
    ProjectEditTaskBoardAutoPromoteTickIntervalSecondsChanged(u64),
    ProjectEditFailedRetryMinutesChanged(u32),
    ProjectEditFailedRetryMaxAttemptsChanged(u32),
    ProjectEditRunningTimeoutMinutesChanged(u32),
    ProjectEditPrSubmittedStallTimeoutSecondsChanged(u32),
    ProjectEditRecycleWorktreeOnTaskFinishToggled(bool),
    ProjectEditWorkflowIdChanged(String),
    ProjectEditSaved,
    ProjectEditRuntimeSaved(Result<(), String>),
    ProjectEditCanceled,
    RecentRevealPressed(String),
    RecentRemovePressed(String),
    ProjectPathChanged(String),
    AttachmentFilesPick,
    AttachmentFilesPicked(Option<Vec<String>>),
    RemoveAttachedFile(String),
    FileUrlChanged(String),
    AddFilePressed,
    FileIndexLoaded(crate::app::FileIndexLoadResult),
    FileIndexReady(Vec<String>),
    ToggleTreeDir(String),
    OpenFilePressed,
    FileManagerShowChanges(bool),
    FileManagerRefreshChanges,
    FileManagerRefreshFileTree,
    FileManagerRefreshAnimationTick,
    OpenChangedFile(String),
    FileTreeRightClicked(String, String, f32, f32),
    FileTreeMenuClose,
    FileTreeAction(FileTreeAction),
    FileTreeRenameChanged(String),
    FileTreeRenameSave,
    FileTreeRenameCompleted {
        old_path: String,
        result: Result<String, String>,
    },
    FileTreeRenameCancel,
    FileTreeFindScopeChanged(String, String),
    FileTreeFindQueryEditorAction(String, text_editor::Action),
    FileTreeFindReplaceEditorAction(String, text_editor::Action),
    FileTreeFindCaseSensitiveToggled(String, bool),
    FileTreeFindWholeWordToggled(String, bool),
    FileTreeFindRegexToggled(String, bool),
    FileTreeFindCaseSensitiveShortcut,
    FileTreeFindWholeWordShortcut,
    FileTreeFindRegexShortcut,
    FileTreeFindResultsPageChanged(String, usize),
    FileTreeFindRun(String),
    FileTreeFindRefreshActive,
    FileTreeFindInProject,
    FileTreeReplaceInProject,
    FileTreeFindCompleted {
        tab_id: String,
        title: String,
        scope_path: String,
        query: String,
        replace_text: String,
        case_sensitive: bool,
        whole_word: bool,
        use_regex: bool,
        matches: Vec<FindInFolderMatch>,
        error: Option<String>,
        limit_reached: bool,
    },
    FileTreeAddToChat {
        path: String,
        line: usize,
        column: usize,
    },
    FileTreeFindMatchSelected {
        path: String,
        line: usize,
        column: usize,
    },
    FileTreeFindTabSelected(String),
    FileTreeFindTabClosed(String),
    FileTreeDragStart(String, Option<(usize, usize)>),
    FileTreeDragEnd,
    FileTreePasteCompleted {
        clear_clipboard: bool,
        result: Result<(), String>,
    },
    FileTreeDeleteCompleted(Result<(), String>),
    SessionRightClicked(String, f32, f32),
    SessionMenuClose,
    SessionTreeToggled(String),
    SessionArchivePressed(String),
    SessionDeletePressed(String),
    SessionRenamePressed(String),
    SessionTitleClicked(String),
    SessionCopyPressed(String),
    SessionRenameChanged(String),
    SessionRenameSave,
    SessionRenameCancel,
    SessionCopied(vw_shared::session::info::Info),
    SessionCreated(vw_shared::session::info::Info),
    SessionsLoaded(Result<Vec<vw_shared::session::info::Info>, String>),
    SessionBootstrapLoaded {
        result: Result<Vec<vw_shared::session::info::Info>, String>,
        previews: std::collections::HashMap<String, String>,
        archived_session_ids: std::collections::HashSet<String>,
    },
    ProjectSessionsLoaded(String, Result<Vec<vw_shared::session::info::Info>, String>),
    ProjectSessionListScrollChanged {
        project_path: String,
        has_vertical_scrollbar: bool,
    },
    ProjectLoadMoreSessions(String),
    ProjectCreateSession(String),
    SessionCreateFailed(String),
    ProjectCreateSessionPicked {
        project_path: String,
        directory: String,
    },
    ProjectCreateSessionPickerLoaded {
        project_path: String,
        options: Result<Vec<(String, String)>, String>,
    },
    ProjectCreateSessionWorktreeNameChanged(String),
    ProjectCreateSessionDeleteWorktree(String),
    ProjectCreateSessionDeleteWorktreeConfirmed,
    ProjectCreateSessionDeleteWorktreeCancel,
    ProjectCreateSessionDeleteWorktreeForceConfirmed,
    ProjectCreateSessionDeleteWorktreeResult {
        project_path: String,
        directory: String,
        result: Result<(), String>,
    },
    ProjectCreateSessionResetWorktree(String),
    ProjectCreateSessionResetWorktreeConfirmed,
    ProjectCreateSessionResetWorktreeCancel,
    ProjectCreateSessionResetWorktreeResult {
        project_path: String,
        directory: String,
        result: Result<(), String>,
    },
    ProjectCreateSessionPickerClose,
    ProjectCreateSessionWorktree(String),
    ProjectLoadSessions(String),
    ProjectSessionsRefreshTick,
    RecentProjectsLoaded(Result<Vec<crate::app::RecentProjectMeta>, String>),
    ProjectInfoLoaded(Result<LoadedProjectInfo, String>),
    ProjectBranchesLoaded {
        project_path: String,
        selected_branch: Option<String>,
        branches: Vec<String>,
    },
    SessionMessagesLoaded(
        Result<(String, Vec<agent_message::WithParts>, crate::app::models::TokenUsage), String>,
    ),
    SessionUiPrepared {
        session_id: String,
        phase: crate::app::session::PreparedChatUiPhase,
    },
    StartDeferredTasks {
        project_path: String,
    },
}

/// update 处理当前模块对应的消息或状态转换。
///
/// 参数由调用方提供应用状态、用户输入或后台任务结果；返回值会交给上层消息循环继续处理。
/// 失败会被转换为界面可展示的状态或消息，避免在处理链路中静默丢失。
pub fn update(app: &mut App, message: ProjectMessage) -> Task<Message> {
    if let Some(task) = basic::handle(app, message.clone()) {
        return task;
    }
    if let Some(task) = find::handle(app, message.clone()) {
        return task;
    }
    if let Some(task) = file_tree::handle(app, message.clone()) {
        return task;
    }
    if let Some(task) = session::handle(app, message) {
        return task;
    }
    Task::none()
}
