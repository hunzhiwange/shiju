//! 右键菜单消息与桌面事件的适配。
use crate::app::{App, Message};
pub(in crate::app::components::git_panel) fn wrap_diff_row_with_context_menu<'a>(app: &App, file: &str, line: usize, is_old: bool, text: String, content: iced::Element<'a, Message>) -> iced::Element<'a, Message> {
    // Existing rows still carry desktop messages until their own slice is migrated.
    desktop_git_ui::git_panel::diff_view::context_menu::wrap_diff_row_with_context_menu(app.desktop_locale, app.git_diff_context_menu.as_ref(), file, line, is_old, text, content, super::map_diff_message)
}
