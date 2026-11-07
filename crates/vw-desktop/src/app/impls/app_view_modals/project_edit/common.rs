//! 渲染应用视图中的模态窗口。
//! 本模块只描述模态内容和交互消息，不直接承担持久化策略。

use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, rounded_action_btn_style,
    settings_field_label, settings_tab_button, settings_text_input_style,
};
use crate::app::state::ProjectEditTab;
use iced::widget::{button, row, text, text_input};
use iced::{Alignment, Color, Element, Length};

use super::super::Message;

/// 模块内可见函数，执行 parse_hex_color 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn parse_hex_color(input: &str) -> Option<Color> {
    let hex = input.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }

    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::from_rgb8(red, green, blue))
}

/// 模块内可见函数，执行 format_hex_color 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn format_hex_color(color: Color) -> String {
    let red = (color.r * 255.0).round() as u8;
    let green = (color.g * 255.0).round() as u8;
    let blue = (color.b * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", red, green, blue)
}

/// 模块内可见函数，执行 icon_image_handle 对应的应用流程。
/// 返回值表达处理结果；失败通过错误值、日志或任务消息显式传递。
pub(super) fn icon_image_handle(icon: &str) -> Option<iced::widget::image::Handle> {
    let raw = icon.trim();
    if raw.is_empty() {
        return None;
    }

    let path = raw.strip_prefix("file://").unwrap_or(raw);
    let path =
        path.strip_prefix("//").map(|rest| format!("/{rest}")).unwrap_or_else(|| path.to_string());
    let path = std::path::Path::new(&path);
    if path.exists() { Some(iced::widget::image::Handle::from_path(path)) } else { None }
}

pub(super) fn tab_button(
    label: &'static str,
    tab: ProjectEditTab,
    selected: bool,
) -> Element<'static, Message> {
    settings_tab_button(
        label,
        selected,
        Some(Message::Project(
            crate::app::message::project::ProjectMessage::ProjectEditTabSelected(tab),
        )),
    )
    .into()
}

pub(super) fn numeric_row<'a>(
    label: &'a str,
    placeholder: &'a str,
    input: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
    decrease: Message,
    increase: Message,
) -> Element<'a, Message> {
    row![
        settings_field_label(label).width(Length::Fill),
        button(text("−").size(14))
            .on_press(decrease)
            .padding([9, 12])
            .style(rounded_action_btn_style),
        text_input(placeholder, input)
            .on_input(on_input)
            .padding(SETTINGS_CONTROL_PADDING)
            .size(SETTINGS_CONTROL_TEXT_SIZE)
            .width(Length::Fixed(80.0))
            .style(settings_text_input_style),
        button(text("+").size(14))
            .on_press(increase)
            .padding([9, 12])
            .style(rounded_action_btn_style),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

#[cfg(test)]
#[path = "common_tests.rs"]
mod common_tests;
