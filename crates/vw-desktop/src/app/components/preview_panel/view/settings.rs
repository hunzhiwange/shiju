//! 预览编辑器的局部设置弹窗，沿用编辑器设置消息与公共表单控件。

use crate::app::components::system_settings_common::{
    primary_action_btn_style, settings_close_button, settings_hover_help_label,
    settings_modal_header, settings_modal_layout, settings_modal_overlay,
};
use crate::app::i18n::I18n;
use crate::app::message::editor::EditorMessage;
use crate::app::{App, Message};
use iced::widget::{button, container, text};
use iced::{Element, Length};

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

pub fn build_settings_overlay<'a>(
    app: &'a App,
    content_base: Element<'a, Message>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let close_message = Message::Editor(EditorMessage::ToggleSettings);
    let title = i18n.__("编辑器设置");
    let header = settings_modal_header(
        settings_hover_help_label(
            i18n,
            &app.app_theme,
            title,
            text(title).size(20),
            i18n.__("统一调整字号、行高、语言与主题。"),
        )
        .wrap(),
        settings_close_button(close_message.clone()),
    );

    let body = crate::app::components::editor_settings_fields::view(app, 240.0);
    let footer = container(
        button(text(i18n.__("完成")).size(14))
            .on_press(close_message.clone())
            .padding([8, 18])
            .height(38)
            .style(primary_action_btn_style),
    )
    .align_right(Length::Fill);

    let card = settings_modal_layout(header, body, Some(footer.into()))
        .width(Length::Fill)
        .max_width(620.0)
        .max_height(660.0);

    let centered = container(card).center_x(Length::Fill).center_y(Length::Fill).padding(12);
    settings_modal_overlay(Some(content_base), close_message, centered)
}

#[cfg(test)]
#[path = "settings_ui_refresh_40_tests.rs"]
mod settings_ui_refresh_40_tests;
