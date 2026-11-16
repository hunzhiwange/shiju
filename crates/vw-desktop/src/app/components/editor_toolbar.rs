//! 编辑器工具栏的桌面消息装配。
use crate::app::{App, Message, Screen};
use crate::app::message::editor::EditorMessage;
use desktop_i18n::I18n;
use iced::Element;

pub fn view<'a>(app: &App, on_save: Option<Message>, is_dirty: bool) -> Element<'a, Message> {
    desktop_editor::components::editor_toolbar::view(
        matches!(app.navigation_tabs.screen, Screen::Project), I18n::new(app.general_settings.desktop_locale), on_save, is_dirty,
        Message::Editor(EditorMessage::Undo), Message::Editor(EditorMessage::Redo),
    )
}
