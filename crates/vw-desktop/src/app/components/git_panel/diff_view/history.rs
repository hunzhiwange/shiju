//! 桌面文件卡片与历史领域视图的消息适配。
use crate::app::{App, Message, message::GitMessage};
use iced::Element;
use vw_api_git::GitHistoryFileDto;
use desktop_git_ui::git_panel::diff_view::history::{self, HistoryAction, HistoryEvent};

pub(crate) fn view_history_diff(app: &App, index: usize, file: &GitHistoryFileDto) -> Element<'static, Message> {
    history::view_history_diff(&app.app_theme, app.git_repository.selected.as_deref(), index, file, |diff| {
        let [background, add_line, add_word, del_line, del_word] = diff.colors;
        super::file_view::view_file(
            app, &diff.context, &file.path,
            file.old_content.as_deref().zip(file.new_content.as_deref()), false,
            diff.status, file.insertions, file.deletions, diff.theme,
            background, add_line, add_word, del_line, del_word,
            matches!(diff.status, super::super::utils::FileStatus::Modified | super::super::utils::FileStatus::Renamed),
        ).map(project_event)
    }).map(|action| match action {
        HistoryAction::SelectFile(index) => Message::Git(GitMessage::SelectHistoryFile(index)),
        HistoryAction::OpenMenu(index) => Message::Git(GitMessage::OpenHistoryFileMenu(index)),
        HistoryAction::CloseMenu => Message::Git(GitMessage::CloseHistoryFileMenu),
        HistoryAction::PreviewFile(index) => Message::Git(GitMessage::PreviewHistoryFile(index)),
        HistoryAction::CopyFile(index) => Message::Git(GitMessage::CopyHistoryFile(index)),
        HistoryAction::Browse(message) => message,
        HistoryAction::None => Message::None,
    })
}
fn project_event(message: Message) -> HistoryEvent<Message> {
    match message {
        Message::Git(GitMessage::ToggleExpandFile(_)) => HistoryEvent::ToggleFile,
        Message::Git(GitMessage::OpenDiffFileMenu(_)) => HistoryEvent::OpenMenu,
        Message::Git(GitMessage::CloseDiffFileMenu) => HistoryEvent::CloseMenu,
        Message::Git(GitMessage::PreviewDiffFile(_)) => HistoryEvent::PreviewFile,
        Message::Git(GitMessage::CopyDiffFile { .. }) => HistoryEvent::CopyFile,
        event @ Message::Git(GitMessage::ExpandContext(..) | GitMessage::ToggleExpandHunk(..)
            | GitMessage::DiffHoverEnter(..) | GitMessage::DiffHoverExit(..)) => HistoryEvent::Browse(event),
        _ => HistoryEvent::Blocked,
    }
}
#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
