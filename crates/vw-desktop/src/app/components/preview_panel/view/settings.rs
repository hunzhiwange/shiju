//! 预览编辑器的局部设置弹窗，沿用编辑器设置消息与公共表单控件。

use crate::app::components::system_settings_common::{
    primary_action_btn_style, settings_checkbox_style, settings_close_button, settings_divider,
    settings_group_card, settings_modal_header, settings_modal_layout, settings_modal_overlay,
    settings_muted_text_style, settings_row_with_flexible_control, settings_select,
    settings_value_badge,
};
use crate::app::components::system_settings_editor::metrics_slider;
use crate::app::i18n::I18n;
use crate::app::message::editor::{EditorMessage, metrics};
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, container, row, text};
use iced::{Alignment, Element, Length};

use super::super::tabs::{DisplayLanguage, all_languages};

/// 设置面板控件产生的局部交互。
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
pub(super) fn selected_editor_theme(app: &App) -> iced::Theme {
    if app.editor_follow_system_theme { app.app_theme.clone() } else { app.editor_theme.clone() }
}

/// 格式化字号徽标。
pub(super) fn font_size_badge_label(font_size: f32) -> String {
    format!("{font_size:.0} pt")
}

/// 格式化行高徽标。
pub(super) fn line_height_badge_label(line_height: f32) -> String {
    format!("{line_height:.1}")
}

pub fn build_settings_overlay<'a>(
    app: &'a App,
    content_base: Element<'a, Message>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let setting_row = |label, description, control: Element<'static, Message>| {
        settings_row_with_flexible_control(label, description, 240.0, control)
    };
    let displayed = metrics::displayed(app);
    let close_message = control_message(SettingsControl::Close);
    let language_pick = settings_select(
        all_languages().iter().copied(),
        Some(DisplayLanguage(app.current_language)),
        |language| control_message(SettingsControl::Language(language.0)),
    )
    .width(Length::Fill);
    let theme_pick = settings_select(
        iced::Theme::ALL.iter().cloned(),
        Some(selected_editor_theme(app)),
        |theme| control_message(SettingsControl::Theme(theme)),
    )
    .width(Length::Fill);

    let header = settings_modal_header(
        column![
            text(i18n.__("编辑器设置")).size(20),
            text(i18n.__("统一调整字号、行高、语言与主题。"))
                .size(13)
                .style(settings_muted_text_style),
        ]
        .spacing(4),
        settings_close_button(close_message.clone()),
    );

    let body = settings_group_card(column![
        setting_row(
            i18n.__("字号"),
            i18n.__("调整编辑器基础字体大小。"),
            row![
                metrics_slider(10.0..=30.0, displayed.font_size, |value| {
                    control_message(SettingsControl::FontSize(value))
                })
                .width(Length::Fill),
                settings_value_badge(font_size_badge_label(displayed.font_size)),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .into(),
        ),
        settings_divider(),
        setting_row(
            i18n.__("自动行高"),
            i18n.__("启用后根据字体大小自动匹配行距。"),
            checkbox(displayed.auto_line_height)
                .label(i18n.__("启用"))
                .on_toggle(|enabled| { control_message(SettingsControl::AutoLineHeight(enabled)) })
                .style(settings_checkbox_style)
                .into(),
        ),
        settings_divider(),
        setting_row(
            i18n.__("行高"),
            i18n.__("关闭自动行高后可手动微调阅读密度。"),
            row![
                metrics_slider(10.0..=50.0, displayed.line_height, |value| {
                    control_message(SettingsControl::LineHeight(value))
                })
                .width(Length::Fill),
                settings_value_badge(line_height_badge_label(displayed.line_height)),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .into(),
        ),
        settings_divider(),
        setting_row(i18n.__("语言"), i18n.__("选择编辑器语法与显示语言。"), language_pick.into()),
        settings_divider(),
        setting_row(
            i18n.__("跟随系统主题"),
            i18n.__("启用后编辑器自动同步桌面主题。"),
            checkbox(app.editor_follow_system_theme)
                .label(i18n.__("启用"))
                .on_toggle(|enabled| {
                    control_message(SettingsControl::FollowSystemTheme(enabled))
                })
                .style(settings_checkbox_style)
                .into(),
        ),
        settings_divider(),
        setting_row(
            i18n.__("主题"),
            i18n.__("关闭跟随后可单独指定编辑器主题。"),
            theme_pick.into(),
        ),
    ]);
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
