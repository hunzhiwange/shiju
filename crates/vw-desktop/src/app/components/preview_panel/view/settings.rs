//! 设置面板视图模块
//!
//! 本模块提供编辑器设置面板的 UI 构建功能，用于创建和显示可交互的设置覆盖层。
//! 设置面板采用模态对话框形式呈现，支持多种编辑器配置选项。
//!
//! # 主要功能
//!
//! - **字体设置**：调整编辑器字号大小
//! - **行高设置**：控制文本行间距，支持自动行高调整
//! - **语言选择**：切换编辑器的显示语言
//! - **主题配置**：选择编辑器主题，支持跟随系统主题
//!
//! # 架构设计
//!
//! 该模块使用 iced GUI 框架构建声明式 UI，采用叠加层（overlay）模式：
//! - 底层保留原有内容
//! - 中层显示半透明遮罩
//! - 顶层居中显示设置面板

use crate::app::components::system_settings_common::{
    primary_action_btn_style, settings_checkbox_style, settings_close_button, settings_divider,
    settings_modal_card, settings_modal_overlay, settings_muted_text_style, settings_panel,
    settings_pick_list_menu_style, settings_pick_list_style, settings_value_badge,
};
use crate::app::message::editor::EditorMessage;
use crate::app::{App, Message};
use iced::widget::{button, checkbox, column, container, pick_list, row, slider, text};
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
        SettingsControl::FontSize(value) => EditorMessage::FontSizeChanged(value),
        SettingsControl::AutoLineHeight(enabled) => EditorMessage::ToggleAutoLineHeight(enabled),
        SettingsControl::LineHeight(value) => EditorMessage::LineHeightChanged(value),
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

/// 构建设置面板覆盖层
///
/// 创建一个包含编辑器设置选项的模态对话框覆盖层，该覆盖层叠加在基础内容之上。
/// 设置面板居中显示，并带有半透明背景遮罩，用户可以通过点击遮罩或关闭按钮关闭面板。
///
/// # 参数
///
/// * `app` - 应用程序状态引用，用于读取当前设置值（字号、行高、语言、主题等）
/// * `content_base` - 基础内容元素，将作为覆盖层的底层显示
///
/// # 返回值
///
/// 返回一个 `Element<Message>`，包含三层叠加的 UI 结构：
/// - 底层：原始内容（`content_base`）
/// - 中层：半透明黑色遮罩（透明度 0.5），点击可关闭设置面板
/// - 顶层：居中的设置面板容器
///
/// # UI 组件
///
/// 设置面板包含以下可交互组件：
///
/// 1. **标题**："编辑器设置"
/// 2. **字号滑块**：范围 10.0 - 30.0，实时显示当前值
/// 3. **自动行高复选框**：切换是否自动调整行高
/// 4. **行高滑块**：范围 10.0 - 50.0，实时显示当前值
/// 5. **语言选择器**：从所有支持的语言中选择显示语言
/// 6. **系统主题复选框**：切换是否跟随系统主题
/// 7. **主题选择器**：从所有可用主题中选择编辑器主题
/// 8. **关闭按钮**：关闭设置面板
///
/// # 样式
///
/// - 面板背景使用主题的弱背景色（`background.weak.color`）
/// - 边框使用主题的主色（`primary.base.color`），宽度 1.0，圆角 10.0
/// - 遮罩使用半透明黑色（RGBA: 0.0, 0.0, 0.0, 0.5）
///
/// # 消息处理
///
/// 所有 UI 交互都会生成 `Message::Editor` 消息，包含相应的 `EditorMessage` 变体：
/// - `FontSizeChanged` - 字号变更
/// - `ToggleAutoLineHeight` - 切换自动行高
/// - `LineHeightChanged` - 行高变更
/// - `LanguageChanged` - 语言变更
/// - `ToggleFollowSystemTheme` - 切换跟随系统主题
/// - `ThemeChanged` - 主题变更
/// - `ToggleSettings` - 切换设置面板显示状态
pub fn build_settings_overlay<'a>(
    app: &'a App,
    content_base: Element<'a, Message>,
) -> Element<'a, Message> {
    fn setting_row<'a>(
        label: &'static str,
        description: &'static str,
        control: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        container(
            row![
                column![
                    text(label).size(13),
                    text(description).size(11).style(settings_muted_text_style),
                ]
                .spacing(4)
                .width(Length::Fixed(168.0)),
                container(control.into()).width(Length::Fill),
            ]
            .spacing(18)
            .align_y(Alignment::Center),
        )
        .padding([14, 0])
        .width(Length::Fill)
        .into()
    }

    let close_message = control_message(SettingsControl::Close);
    let language_pick =
        pick_list(all_languages(), Some(DisplayLanguage(app.current_language)), |l| {
            control_message(SettingsControl::Language(l.0))
        })
        .padding([10, 14])
        .style(settings_pick_list_style)
        .menu_style(settings_pick_list_menu_style)
        .width(Length::Fixed(240.0));

    let theme_pick = pick_list(iced::Theme::ALL, Some(selected_editor_theme(app)), |theme| {
        control_message(SettingsControl::Theme(theme))
    })
    .padding([10, 14])
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fixed(240.0));

    let content = column![
        row![
            column![
                text("编辑器设置").size(22),
                text("统一调整字号、行高、语言与主题。").size(12).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fill),
            settings_close_button(close_message.clone()),
        ]
        .align_y(Alignment::Start),
        settings_panel(
            column![
                setting_row(
                    "字号",
                    "调整编辑器基础字体大小。",
                    row![
                        slider(10.0..=30.0, app.current_font_size, |value| {
                            control_message(SettingsControl::FontSize(value))
                        })
                        .width(Length::Fill),
                        settings_value_badge(font_size_badge_label(app.current_font_size)),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                ),
                settings_divider(),
                setting_row(
                    "自动行高",
                    "启用后根据字体大小自动匹配行距。",
                    checkbox(app.auto_adjust_line_height)
                        .label("启用")
                        .on_toggle(|enabled| {
                            control_message(SettingsControl::AutoLineHeight(enabled))
                        })
                        .style(settings_checkbox_style),
                ),
                settings_divider(),
                setting_row(
                    "行高",
                    "关闭自动行高后可手动微调阅读密度。",
                    row![
                        slider(10.0..=50.0, app.current_line_height, |value| {
                            control_message(SettingsControl::LineHeight(value))
                        })
                        .width(Length::Fill),
                        settings_value_badge(line_height_badge_label(app.current_line_height)),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                ),
                settings_divider(),
                setting_row("语言", "选择编辑器语法与显示语言。", language_pick),
                settings_divider(),
                setting_row(
                    "跟随系统主题",
                    "启用后编辑器自动同步桌面主题。",
                    checkbox(app.editor_follow_system_theme)
                        .label("启用")
                        .on_toggle(|enabled| {
                            control_message(SettingsControl::FollowSystemTheme(enabled))
                        })
                        .style(settings_checkbox_style),
                ),
                settings_divider(),
                setting_row("主题", "关闭跟随后可单独指定编辑器主题。", theme_pick),
            ]
            .spacing(0),
        ),
        container(
            button(text("完成").size(13))
                .on_press(close_message.clone())
                .padding([10, 18])
                .style(primary_action_btn_style),
        )
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(16);

    let card = settings_modal_card(content).width(Length::Fixed(620.0));

    settings_modal_overlay(Some(content_base), close_message, card)
}
