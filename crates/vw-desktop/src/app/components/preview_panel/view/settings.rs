//! 预览编辑器的局部设置弹窗，沿用编辑器设置消息与公共表单控件。

use crate::app::i18n::I18n;
use crate::app::message::editor::EditorMessage;
use crate::app::{App, Message};

use iced::Element;

/// 设置面板控件产生的局部交互。
#[cfg(test)]
pub(super) enum SettingsControl {
    /// 关闭设置面板。
    Close,
    /// 更新编辑器字号。
    FontSize(f32),
    /// 切换自动行高。
    AutoLineHeight(bool),
    /// 更新手动行高。
    LineHeight(f32),
    /// 更新编辑器语言。
    Language(iced_code_editor::i18n::Language),
    /// 切换跟随系统主题。
    FollowSystemTheme(bool),
    /// 更新手动主题。
    Theme(iced::Theme),
}

/// 将设置面板的局部交互映射为应用消息。
#[cfg(test)]
pub(super) fn control_message(control: SettingsControl) -> Message {
    let message = match control {
        SettingsControl::Close => EditorMessage::ToggleSettings,
        SettingsControl::FontSize(value) => EditorMessage::FontSizePreview(value),
        SettingsControl::AutoLineHeight(enabled) => EditorMessage::ToggleAutoLineHeight(enabled),
        SettingsControl::LineHeight(value) => EditorMessage::LineHeightPreview(value),
        SettingsControl::Language(language) => EditorMessage::LanguageChanged(language),
        SettingsControl::FollowSystemTheme(enabled) => {
            EditorMessage::ToggleFollowSystemTheme(enabled)
        }
        SettingsControl::Theme(theme) => EditorMessage::ThemeChanged(theme),
    };

    Message::Editor(message)
}

/// 返回设置面板当前应显示的编辑器主题。
#[cfg(test)]
pub(super) fn selected_editor_theme(app: &App) -> iced::Theme {
    if app.editor_follow_system_theme { app.app_theme.clone() } else { app.editor_theme.clone() }
}

/// 格式化字号徽标。
#[cfg(test)]
pub(super) fn font_size_badge_label(font_size: f32) -> String {
    format!("{font_size:.0}px")
}

/// 格式化行高徽标。
#[cfg(test)]
pub(super) fn line_height_badge_label(line_height: f32) -> String {
    format!("{line_height:.1}px")
}

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
