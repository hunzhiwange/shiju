//! 复用普通预览编辑器打开只读内容，标签身份与工作区文件分离。
use super::{PreviewMessage, tab};
use crate::app::{App, Message};
use iced::Task;

pub(super) fn open(
    app: &mut App,
    key: String,
    path: String,
    title: String,
    content: String,
) -> Task<Message> {
    let theme = app.effective_editor_theme();
    desktop_editor::preview::snapshot::open(&mut app.navigation_tabs.preview_tabs, key.clone(), path, title, content, theme, app.preview_state.current_font_size, app.preview_state.current_line_height, app.general_settings.desktop_locale);
    tab::update(app, PreviewMessage::Open(key))
}
