use iced::widget::text_editor;
use super::types::*;
#[derive(Debug, Clone)]
pub enum UiMessage<External> {
    CommitDescriptionEditorAction(text_editor::Action),
    CommitHelpOpen,
    CommitMessageChanged(String),
    CommitScopeChanged(String),
    CommitSelected,
    CommitTypeSelected(ConventionalCommitType),
    HoverGitPanelHeaderEnter,
    HoverGitPanelHeaderExit,
    InvertVisibleFileLines(Vec<String>),
    LoadMoreHistory,
    RefreshRepository,
    RunRemote(super::repository_ops::RemoteAction),
    SelectAllVisibleFileLines(Vec<String>),
    SelectGitWorktree(GitWorktreeOption),
    SelectHistoryCommit(String),
    ShowHistory(bool),
    ToggleFilterOptions(bool),
    ToggleFullscreen,
    ToggleGitWorktreeMenu(bool),
    ToggleHalfFullscreen,
    OpenTerminal,
    ToggleSummary,
    External(External),
}
