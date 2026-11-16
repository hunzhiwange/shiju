//! 评论草稿的桌面适配。
use crate::page::Message;
use crate::page::ViewContext;
pub(crate) fn diff_comment_editor<'a>(app: &ViewContext<'a>) -> Option<iced::Element<'a, Message>> {
    crate::git_panel::diff_view::comment_editor::diff_comment_editor(
        app.desktop_locale,
        app.git.git_diff_comment_draft.as_ref(),
    )
    .map(|view| view.map(super::map_diff_message))
}
