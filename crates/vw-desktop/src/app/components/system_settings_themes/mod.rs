//! 系统设置「主题」页：画廊列表与编辑器。

pub mod editor;
pub mod gallery;
pub mod preview;

#[cfg(test)]
#[path = "gallery_tests.rs"]
mod gallery_tests;

use crate::app::components::system_settings_common::settings_modal_overlay;
use crate::app::message::settings::{SettingsMessage, ThemesMessage};
use crate::app::{App, Message};
use iced::Element;
use iced::widget::container;

/// 渲染主题画廊。
pub fn view(app: &App) -> Element<'_, Message> {
    gallery::view(app)
}

/// 从设置外壳挂载编辑弹窗，使长表单获得窗口约束。
pub(super) fn editor_overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let cancel = Message::Settings(SettingsMessage::Themes(ThemesMessage::CancelEdit));
    settings_modal_overlay(Some(base), cancel, container(editor::view(app)).padding(12))
}

fn inputs(app: &App) -> desktop_settings_appearance::system_settings_themes::View<'_, Message> {
    desktop_settings_appearance::system_settings_themes::View {
        desktop_locale: app.desktop_locale, app_theme: &app.app_theme,
        active_custom_theme_id: &app.active_custom_theme_id,
        themes_settings: &app.themes_settings,
        map: |message| Message::Settings(SettingsMessage::Themes(message)),
        none: Message::None,
        help: |title, body, color, i18n| super::chat_panel::message_view::selectable_titled_text(title, body, color, i18n),
    }
}
