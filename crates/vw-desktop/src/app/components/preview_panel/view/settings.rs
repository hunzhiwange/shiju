//! 预览编辑器的局部设置弹窗，沿用编辑器设置消息与公共表单控件。

use crate::app::i18n::I18n;
use crate::app::message::editor::EditorMessage;
use crate::app::{App, Message};

use iced::Element;

pub fn build_settings_overlay<'a>(app: &'a App, content_base: Element<'a, Message>) -> Element<'a, Message> {
    desktop_editor::components::preview_panel::view::settings::build_settings_overlay(
        I18n::new(app.desktop_locale),
        crate::app::components::chat_panel::message_view::selectable_titled_text(
            I18n::new(app.desktop_locale).__("编辑器设置"),
            I18n::new(app.desktop_locale).__("统一调整字号、行高、语言与主题。"),
            Some(app.app_theme.palette().text), I18n::new(app.desktop_locale),
        ),
        crate::app::components::editor_settings_fields::view(app, 240.0),
        Message::Editor(EditorMessage::ToggleSettings), content_base,
    )
}

#[cfg(test)]
#[path = "settings_ui_refresh_40_tests.rs"]
mod settings_ui_refresh_40_tests;
