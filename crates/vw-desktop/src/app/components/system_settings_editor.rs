//! 系统设置编辑器组件
//!
//! 本模块提供编辑器相关设置的用户界面组件，用于配置编辑器的显示和行为参数。
//! 主要功能包括：
//! - 主题选择：支持跟随系统主题或自定义编辑器主题
//! - 字体大小调整：提供滑块控制编辑器字体大小
//! - 行高配置：支持手动调整行高或自动根据字体大小调整
//!
//! # 示例
//!
//! ```ignore
//! use crate::app::components::system_settings_editor;
//! let settings_view = system_settings_editor::view(&app);
//! ```

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_divider, settings_muted_text_style,
    settings_page_intro, settings_panel, settings_pick_list_menu_style, settings_pick_list_style,
    settings_section_card, settings_value_badge,
};
use crate::app::{App, Message, message};
use iced::widget::{checkbox, column, container, pick_list, row, slider, text};
use iced::{Alignment, Element, Length};

fn field_row<'a>(
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
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

/// 渲染系统设置编辑器视图
///
/// 构建并返回编辑器设置界面的 UI 元素，包含主题、字体大小和行高等配置选项。
///
/// # 参数
///
/// - `app` - 应用程序状态引用，包含当前编辑器配置信息
///
/// # 返回值
///
/// 返回一个 `Element` 类型的 UI 组件，包含了所有编辑器设置的交互控件
///
/// # 示例
///
/// ```ignore
/// let view = view(&app);
/// // 返回包含主题选择器、字体大小滑块、行高滑块等控件的界面
/// ```
///
/// # UI 组件
///
/// 该视图包含以下设置项：
/// 1. 跟随系统主题 - 复选框，控制是否使用系统主题
/// 2. 编辑器主题 - 下拉列表，选择编辑器配色主题
/// 3. 字体大小 - 滑块控件，调整编辑器字体大小（10-30px）
/// 4. 行高 - 滑块控件，调整编辑器行高（10-60px）
/// 5. 自动行高 - 复选框，控制是否根据字体大小自动调整行高
pub(crate) fn content(app: &App) -> Element<'_, Message> {
    let editor_theme = if app.editor_follow_system_theme {
        app.app_theme.clone()
    } else {
        app.editor_theme.clone()
    };

    let theme_pick = pick_list(iced::Theme::ALL, Some(editor_theme), |t| {
        Message::Editor(message::EditorMessage::ThemeChanged(t))
    })
    .padding([10, 14])
    .text_size(13)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fixed(280.0));

    let follow_system_theme = field_row(
        "跟随系统主题",
        "启用后沿用应用当前主题。",
        checkbox(app.editor_follow_system_theme)
            .label("启用")
            .on_toggle(|v| Message::Editor(message::EditorMessage::ToggleFollowSystemTheme(v)))
            .style(settings_checkbox_style),
    );

    let editor_theme_row =
        field_row("编辑器主题", "关闭跟随系统主题时使用的编辑器配色。", theme_pick);

    let size_slider = row![
        slider(10.0..=30.0, app.current_font_size, |v| {
            Message::Editor(message::EditorMessage::FontSizeChanged(v))
        })
        .width(Length::Fixed(220.0)),
        settings_value_badge(format!("{:.0}px", app.current_font_size))
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let size_row = field_row("字体大小", "调整编辑器主文本字号。", size_slider);

    let line_height_slider = row![
        slider(10.0..=60.0, app.current_line_height, |v| {
            Message::Editor(message::EditorMessage::LineHeightChanged(v))
        })
        .width(Length::Fixed(220.0)),
        settings_value_badge(format!("{:.1}px", app.current_line_height))
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let line_height_row = field_row("行高", "调整每一行的垂直高度。", line_height_slider);

    let auto_line_height = field_row(
        "自动行高",
        "根据字体大小自动推导更合适的行高。",
        checkbox(app.auto_adjust_line_height)
            .label("根据字体大小自动调整")
            .on_toggle(|v| Message::Editor(message::EditorMessage::ToggleAutoLineHeight(v)))
            .style(settings_checkbox_style),
    );

    column![
        settings_section_card("主题与显示", "统一编辑器主题来源及排版参数。"),
        settings_panel(
            column![
                follow_system_theme,
                settings_divider(),
                editor_theme_row,
                settings_divider(),
                size_row,
                settings_divider(),
                line_height_row,
                settings_divider(),
                auto_line_height
            ]
            .spacing(0)
        )
    ]
    .spacing(16)
    .into()
}

pub fn view(app: &App) -> Element<'_, Message> {
    column![
        settings_page_intro("编辑器配置", "配置主题、字号与行高等编辑体验参数。"),
        content(app),
    ]
    .spacing(16)
    .into()
}
#[cfg(test)]
#[path = "system_settings_editor_tests.rs"]
mod system_settings_editor_tests;
