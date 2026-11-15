use crate::app::{Message, message};
use desktop_git_ui::git_panel::messages::UiMessage;
pub(super) fn map(value: UiMessage<Message>) -> Message {
 match value {
        UiMessage::CommitDescriptionEditorAction(value) => Message::Git(message::GitMessage::CommitDescriptionEditorAction(value)),
        UiMessage::CommitHelpOpen => Message::Git(message::GitMessage::CommitHelpOpen),
        UiMessage::CommitMessageChanged(value) => Message::Git(message::GitMessage::CommitMessageChanged(value)),
        UiMessage::CommitScopeChanged(value) => Message::Git(message::GitMessage::CommitScopeChanged(value)),
        UiMessage::CommitSelected => Message::Git(message::GitMessage::CommitSelected),
        UiMessage::CommitTypeSelected(value) => Message::Git(message::GitMessage::CommitTypeSelected(value)),
        UiMessage::HoverGitPanelHeaderEnter => Message::Git(message::GitMessage::HoverGitPanelHeaderEnter),
        UiMessage::HoverGitPanelHeaderExit => Message::Git(message::GitMessage::HoverGitPanelHeaderExit),
        UiMessage::InvertVisibleFileLines(value) => Message::Git(message::GitMessage::InvertVisibleFileLines(value)),
        UiMessage::LoadMoreHistory => Message::Git(message::GitMessage::LoadMoreHistory),
        UiMessage::RefreshRepository => Message::Git(message::GitMessage::RefreshRepository),
        UiMessage::RunRemote(value) => Message::Git(message::GitMessage::RunRemote(value)),
        UiMessage::SelectAllVisibleFileLines(value) => Message::Git(message::GitMessage::SelectAllVisibleFileLines(value)),
        UiMessage::SelectGitWorktree(value) => Message::Git(message::GitMessage::SelectGitWorktree(value)),
        UiMessage::SelectHistoryCommit(value) => Message::Git(message::GitMessage::SelectHistoryCommit(value)),
        UiMessage::ShowHistory(value) => Message::Git(message::GitMessage::ShowHistory(value)),
        UiMessage::ToggleFilterOptions(value) => Message::Git(message::GitMessage::ToggleFilterOptions(value)),
        UiMessage::ToggleFullscreen => Message::Git(message::GitMessage::ToggleFullscreen),
        UiMessage::ToggleGitWorktreeMenu(value) => Message::Git(message::GitMessage::ToggleGitWorktreeMenu(value)),
        UiMessage::ToggleHalfFullscreen => Message::Git(message::GitMessage::ToggleHalfFullscreen),
 UiMessage::OpenTerminal => Message::View(message::ViewMessage::OpenTerminalPressed),
 UiMessage::ToggleSummary => Message::View(message::ViewMessage::ToggleGitDiffSummary),
 UiMessage::External(value) => value,
 }
}
