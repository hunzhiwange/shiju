//! 任务看板面板样式集合，统一维护按钮、弹层和容器的视觉规则。

use iced::widget::{button, container, text, text_editor, text_input};
use iced::{Border, Theme};


use desktop_ui_controls::system_settings_common::{
    settings_muted_text_style, settings_panel_style,
    settings_text_editor_style,
    settings_text_input_style,
};

/// 构建或更新 input label 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn input_label<'a, Message: 'a>(label: &'a str) -> iced::Element<'a, Message> {
    text(label).size(12).style(settings_muted_text_style).into()
}

/// 构建或更新 input style 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    settings_text_input_style(theme, status)
}

/// 构建或更新 editor style 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn editor_style(theme: &Theme, status: text_editor::Status) -> text_editor::Style {
    settings_text_editor_style(theme, status)
}

pub fn selector_trigger_style(
    theme: &Theme,
    status: button::Status,
    open: bool,
) -> button::Style {
    crate::selector_styles::selector_trigger_style(theme, status, open)
}

pub fn selector_option_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    crate::selector_styles::selector_option_style(theme, status, selected)
}

pub fn popover_style(theme: &Theme) -> container::Style {
    crate::selector_styles::popover_style(theme)
}

/// 构建或更新 tooltip dark style 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn tooltip_dark_style(_theme: &Theme) -> container::Style {
    crate::selector_styles::tooltip_dark_style(_theme)
}

/// 构建或更新 square icon button style 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn square_icon_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    enabled: bool,
) -> iced::widget::button::Style {
    crate::selector_styles::square_icon_button_style(theme, status, enabled)
}







/// 构建或更新 panel container style 相关行为。
///
/// 参数由调用方提供，返回值直接交给上层视图或状态处理；本函数不执行额外错误恢复。
pub fn panel_container_style(theme: &Theme) -> container::Style {
    let mut style = settings_panel_style(theme);
    style.border.radius = Border { radius: 22.0.into(), ..style.border }.radius;
    style
}

#[cfg(test)]
#[path = "styles_tests.rs"]
mod styles_tests;
