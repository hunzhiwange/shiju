//! 系统设置中 memory 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use desktop_ui_controls::number_input::NumberInput;
use desktop_ui_controls::system_settings_common::{SETTINGS_CONTROL_TEXT_SIZE, settings_checkbox_style, settings_field_label, settings_hover_help_label, settings_muted_text_style, settings_text_input};
use desktop_i18n::I18n;
use iced::widget::{checkbox, column, container, row, text};
use iced::{Alignment, Element, Length};
pub fn field_row_with_help<'a, Message: Clone + 'a>(
    locale: config_ui::DesktopLocale,
    theme: &iced::Theme,
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    if description.is_empty() {
        return aligned_field_row(column![settings_field_label(label)], control);
    }
    let heading = settings_hover_help_label(
        I18n::new(locale),
        theme,
        label,
        settings_field_label(label),
        description,
    )
    .wrap();
    aligned_field_row(column![heading], control)
}

pub(crate) fn aligned_field_row<'a, Message: Clone + 'a>(
    labels: iced::widget::Column<'a, Message>,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        row![
            labels.width(Length::FillPortion(2)),
            container(control.into()).align_right(Length::FillPortion(3)),
        ]
        .spacing(24)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

pub fn text_row_with_help<'a, Message: Clone + 'a>(
    locale: config_ui::DesktopLocale,
    theme: &iced::Theme,
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    field_row_with_help(
        locale, theme,
        label,
        description,
        settings_text_input(placeholder, value).on_input(on_input).width(Length::Fill),
    )
}

pub fn bool_row<'a, Message: Clone + 'a>(
    locale: config_ui::DesktopLocale,
    theme: &iced::Theme,
    label: &'static str,
    description: &'static str,
    checked: bool,
    checkbox_label: &'static str,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    field_row_with_help(
        locale, theme,
        label,
        description,
        checkbox(checked)
            .label(checkbox_label)
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .on_toggle(on_toggle)
            .style(settings_checkbox_style),
    )
}

pub(crate) fn parse_bounded_number(value: &str, min: f32, max: f32) -> f32 {
    value.parse::<f32>().unwrap_or(min).clamp(min, max)
}

pub fn number_row<'a, Message: Clone + 'a>(
    label: &'static str,
    value: String,
    suffix: &'static str,
    min: f32,
    max: f32,
    step: f32,
    precision: u8,
    none: Message,
    on_change: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    aligned_field_row(
        column![settings_field_label(label)],
        row![
            container(
                NumberInput::new(
                    parse_bounded_number(&value, min, max),
                    min,
                    max,
                    step,
                    precision,
                    step.max(0.01), none,
                    on_change,
                )
                .settings_style()
            )
            .width(110),
            text(suffix).size(13).style(settings_muted_text_style),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    )
}
