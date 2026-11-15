//! 预览标签菜单的桌面消息映射。
use crate::app::{Message, message::PreviewMessage};
use desktop_editor::components::preview_panel::tabs;

pub fn build_preview_tab_menu<'a>(i18n: desktop_i18n::I18n, path: &str) -> iced::Element<'a, Message> {
    tabs::build_preview_tab_menu(i18n, path).map(|action| Message::Preview(match action {
        tabs::Message::Close(path) => PreviewMessage::Close(path),
        tabs::Message::TabMenuCloseLeft(path) => PreviewMessage::TabMenuCloseLeft(path),
        tabs::Message::TabMenuCloseRight(path) => PreviewMessage::TabMenuCloseRight(path),
        tabs::Message::TabMenuCloseAll => PreviewMessage::TabMenuCloseAll,
    }))
}
