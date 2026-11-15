//! 评论草稿的桌面适配。
use crate::app::{App, Message};
pub(crate) fn diff_comment_editor(app: &App) -> Option<iced::Element<'_, Message>> {
    desktop_git_ui::git_panel::diff_view::comment_editor::diff_comment_editor(app.desktop_locale, app.git_diff_comment_draft.as_ref()).map(|view| view.map(super::map_diff_message))
}
