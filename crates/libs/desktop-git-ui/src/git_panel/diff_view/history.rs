//! 历史差异快照参数与只读交互边界。
use super::DiffRenderCtx;
use crate::{DiffTheme, git_panel::utils::FileStatus};
use iced::{Color, Element};
use vw_api_git::{GitFileStatus, GitHistoryFileDto};

/// 尚由桌面组合的文件卡片所需的只读参数。
pub struct HistoryDiff {
    pub context: DiffRenderCtx<'static>,
    pub status: FileStatus,
    pub theme: DiffTheme,
    pub colors: [Color; 5],
}

/// 文件卡片事件在历史视图中的有限投影。
pub enum HistoryEvent<Message> {
    ToggleFile, OpenMenu, CloseMenu, PreviewFile, CopyFile,
    Browse(Message), Blocked,
}
pub enum HistoryAction<Message> {
    SelectFile(usize), OpenMenu(usize), CloseMenu, PreviewFile(usize), CopyFile(usize),
    Browse(Message), None,
}
pub fn history_event<Message>(index: usize, event: HistoryEvent<Message>) -> HistoryAction<Message> {
    match event {
        HistoryEvent::ToggleFile => HistoryAction::SelectFile(index),
        HistoryEvent::OpenMenu => HistoryAction::OpenMenu(index),
        HistoryEvent::CloseMenu => HistoryAction::CloseMenu,
        HistoryEvent::PreviewFile => HistoryAction::PreviewFile(index),
        HistoryEvent::CopyFile => HistoryAction::CopyFile(index),
        HistoryEvent::Browse(event) => HistoryAction::Browse(event),
        HistoryEvent::Blocked => HistoryAction::None,
    }
}

/// 组合历史快照与文件卡片；卡片适配器仅负责尚未迁移的桌面头部。
pub fn view_history_diff<Message: 'static>(
    app_theme: &iced::Theme, selected_commit: Option<&str>, index: usize,
    file: &GitHistoryFileDto,
    render_file: impl FnOnce(&HistoryDiff) -> Element<'static, HistoryEvent<Message>>,
) -> Element<'static, HistoryAction<Message>> {
    let theme = if super::styles::is_dark_theme(app_theme) {
        DiffTheme::Monokai
    } else {
        DiffTheme::GitHub
    };
    let (background, _, add_line, add_word, del_line, del_word, _, _) =
        crate::git_panel::utils::get_diff_colors(theme);
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
        selected_commit.unwrap_or_default(),
        file.path
    );
    let mut context = super::DiffRenderCtx::history(index, key);
    context.history_notice = file.notice.clone();
    let diff = HistoryDiff { context, status, theme, colors: [background, add_line, add_word, del_line, del_word] };
    render_file(&diff).map(move |event| history_event(index, event))
}
#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
