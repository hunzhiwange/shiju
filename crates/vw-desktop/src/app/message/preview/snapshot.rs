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
    desktop_editor::preview::snapshot::open(&mut app.navigation_tabs.preview_tabs, key.clone(), path, title, content, theme, app.current_font_size, app.current_line_height, app.desktop_locale);
    tab::update(app, PreviewMessage::Open(key))
}
