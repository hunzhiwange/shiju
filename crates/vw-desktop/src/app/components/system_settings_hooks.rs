//! 钩子配置设置界面组件
//!
//! 本模块提供 钩子配置的图形化设置入口，用于管理运行时 hooks 总开关、
//! 当前内置钩子以及未来预留的自定义钩子扩展区域。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_muted_text_style, settings_page_intro,
    settings_panel, settings_section_card,
};
use crate::app::message::settings::HooksMessage;
use crate::app::{App, Message, message};
use iced::widget::{checkbox, column, container, row, text};
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

fn enabled_toggled(value: bool) -> Message {
    Message::Settings(message::SettingsMessage::Hooks(HooksMessage::EnabledToggled(value)))
}

fn command_logger_toggled(value: bool) -> Message {
    Message::Settings(message::SettingsMessage::Hooks(HooksMessage::CommandLoggerToggled(value)))
}

pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.hooks_settings;

    let enabled_row = field_row(
        "启用",
        "控制是否开启运行时 hooks 执行。",
        checkbox(s.enabled)
            .label("开启运行时 hooks 执行")
            .on_toggle(enabled_toggled)
            .style(settings_checkbox_style),
    );

    let enabled_hint = row![
        container(text("")).width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
        text("关闭总开关后将暂停 hooks 执行，但会保留内置钩子的具体启用状态。")
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let builtin_header = settings_section_card(
        "内置钩子",
        "当前提供审计类内置 hooks。后续新增内置项时会继续在这里扩展。",
    );

    let command_logger_row = field_row(
        "command_logger",
        "记录命令调用信息，便于审计与问题回溯。",
        checkbox(s.command_logger).on_toggle(command_logger_toggled).style(settings_checkbox_style),
    );

    let custom_placeholder = settings_section_card(
        "自定义钩子（预留）",
        "此区域为未来扩展预留。后续会在这里加入自定义 hook 注册、启停和参数配置能力。",
    );

    let content = column![
        settings_page_intro("钩子配置", "配置运行时 hooks 总开关与内置钩子的启用状态。"),
        settings_section_card("基础行为", "关闭总开关后将暂停 hooks 执行。"),
        settings_panel(column![enabled_row].spacing(0)),
        enabled_hint,
        builtin_header,
        settings_panel(column![command_logger_row].spacing(0)),
        custom_placeholder,
    ]
    .spacing(16)
    .width(Length::Fill);

    container(content).width(Length::Fill).into()
}
#[cfg(test)]
#[path = "system_settings_hooks_tests.rs"]
mod system_settings_hooks_tests;
