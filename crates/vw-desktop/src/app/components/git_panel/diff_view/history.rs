//! 历史文件复用完整变更卡片，只允许浏览提交快照。
use super::super::utils::FileStatus;
use crate::app::{App, DiffTheme, Message, message::GitMessage};
use iced::Element;
use vw_gateway_client::vw_api_types::git::{GitFileStatus, GitHistoryFileDto};

pub(crate) fn view_history_diff(
    app: &App,
    index: usize,
    file: &GitHistoryFileDto,
) -> Element<'static, Message> {
    let theme = if super::super::is_dark_theme(&app.app_theme) {
        DiffTheme::Monokai
    } else {
        DiffTheme::GitHub
    };
    let (background, _, add_line, add_word, del_line, del_word, _, _) =
        super::super::get_diff_colors(theme);
    let status = match file.status {
        GitFileStatus::Added | GitFileStatus::Untracked => FileStatus::Added,
        GitFileStatus::Deleted => FileStatus::Deleted,
        GitFileStatus::Renamed => FileStatus::Renamed,
        GitFileStatus::Modified | GitFileStatus::Copied | GitFileStatus::TypeChanged => {
            FileStatus::Modified
        }
    };
    let key = format!(
        "history/{}/{}",
        app.git_repository.selected.as_deref().unwrap_or_default(),
        file.path
    );
    let mut context = super::DiffRenderCtx::history(index, key);
    context.history_notice = file.notice.clone();
    super::file_view::view_file(
        app,
        &context,
        &file.path,
        file.old_content.as_deref().zip(file.new_content.as_deref()),
        false,
        status,
        file.insertions,
        file.deletions,
        theme,
        background,
        add_line,
        add_word,
        del_line,
        del_word,
        matches!(status, FileStatus::Modified | FileStatus::Renamed),
    )
    .map(move |message| match message {
        Message::Git(GitMessage::ToggleExpandFile(_)) => {
            Message::Git(GitMessage::SelectHistoryFile(index))
        }
        Message::Git(GitMessage::OpenDiffFileMenu(_)) => {
            Message::Git(GitMessage::OpenHistoryFileMenu(index))
        }
        Message::Git(GitMessage::CloseDiffFileMenu) => {
            Message::Git(GitMessage::CloseHistoryFileMenu)
        }
        Message::Git(GitMessage::PreviewDiffFile(_)) => {
            Message::Git(GitMessage::PreviewHistoryFile(index))
        }
        Message::Git(GitMessage::CopyDiffFile { .. }) => {
            Message::Git(GitMessage::CopyHistoryFile(index))
        }
        other => history_event(other),
    })
}

fn history_event(message: Message) -> Message {
    match message {
        event @ Message::Git(
            GitMessage::ExpandContext(..)
            | GitMessage::ToggleExpandHunk(..)
            | GitMessage::DiffHoverEnter(..)
            | GitMessage::DiffHoverExit(..),
        ) => event,
        _ => Message::None,
    }
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
