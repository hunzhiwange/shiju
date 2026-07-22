//! 系统设置中 coordination 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_divider, settings_help_button,
    settings_muted_text_style, settings_page_intro, settings_panel, settings_section_card,
    settings_text_input_style, settings_value_badge, with_settings_help_modal,
};
use crate::app::{App, Message, message};
use iced::widget::{checkbox, column, container, row, slider, text, text_input};
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

fn bool_row<'a>(
    label: &'static str,
    description: &'static str,
    checked: bool,
    checkbox_label: &'static str,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        checkbox(checked).label(checkbox_label).on_toggle(on_toggle).style(settings_checkbox_style),
    )
}

fn text_row<'a>(
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        text_input(placeholder, value)
            .on_input(on_input)
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    )
}

fn slider_row<'a>(
    label: &'static str,
    description: &'static str,
    slider: impl Into<Element<'a, Message>>,
    value: impl ToString,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        row![slider.into(), settings_value_badge(value)].spacing(12).align_y(Alignment::Center),
    )
}

fn enabled_toggled(value: bool) -> Message {
    Message::Settings(message::SettingsMessage::CoordinationEnabledToggled(value))
}

fn lead_agent_changed(value: String) -> Message {
    Message::Settings(message::SettingsMessage::CoordinationLeadAgentChanged(value))
}

fn max_inbox_messages_per_agent_changed(value: f32) -> Message {
    Message::Settings(message::SettingsMessage::CoordinationMaxInboxMessagesPerAgentChanged(
        value.round() as u32,
    ))
}

fn max_dead_letters_changed(value: f32) -> Message {
    Message::Settings(message::SettingsMessage::CoordinationMaxDeadLettersChanged(
        value.round() as u32
    ))
}

fn max_context_entries_changed(value: f32) -> Message {
    Message::Settings(message::SettingsMessage::CoordinationMaxContextEntriesChanged(
        value.round() as u32
    ))
}

fn max_seen_message_ids_changed(value: f32) -> Message {
    Message::Settings(message::SettingsMessage::CoordinationMaxSeenMessageIdsChanged(
        value.round() as u32
    ))
}

/// 构建或处理 `view` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.coordination_settings;

    let help_btn =
        settings_help_button(Message::Settings(message::SettingsMessage::CoordinationHelpOpen));

    let enabled_row = bool_row(
        "启用",
        "开启多代理 delegate coordination 总线。",
        s.enabled,
        "开启 Delegate Coordination",
        enabled_toggled,
    );

    let lead_agent_row = text_row(
        "主协调 Agent",
        "逻辑主协调 agent 标识，通常保持为 delegate-lead。",
        "delegate-lead",
        &s.lead_agent_input,
        lead_agent_changed,
    );

    let inbox_slider = slider(
        1.0..=10_000.0,
        s.max_inbox_messages_per_agent as f32,
        max_inbox_messages_per_agent_changed,
    )
    .width(Length::Fill);

    let inbox_row = slider_row(
        "每 Agent 收件箱",
        "每个注册 agent 保留的收件箱消息上限。",
        inbox_slider,
        s.max_inbox_messages_per_agent,
    );

    let dead_letters_slider =
        slider(1.0..=10_000.0, s.max_dead_letters as f32, max_dead_letters_changed)
            .width(Length::Fill);

    let dead_letters_row = slider_row(
        "死信上限",
        "未送达或失败消息在死信队列中的保留上限。",
        dead_letters_slider,
        s.max_dead_letters,
    );

    let context_entries_slider =
        slider(1.0..=20_000.0, s.max_context_entries as f32, max_context_entries_changed)
            .width(Length::Fill);

    let context_entries_row = slider_row(
        "上下文条目上限",
        "共享上下文键值与 patch 条目的保留容量。",
        context_entries_slider,
        s.max_context_entries,
    );

    let seen_ids_slider =
        slider(1.0..=100_000.0, s.max_seen_message_ids as f32, max_seen_message_ids_changed)
            .width(Length::Fill);

    let seen_ids_row = slider_row(
        "去重窗口上限",
        "已处理消息 ID 的去重窗口大小。",
        seen_ids_slider,
        s.max_seen_message_ids,
    );

    let col = column![
        row![
            settings_page_intro("协调配置", "配置 delegate 协调总线的容量、留存与去重窗口。"),
            container(text(" ")).width(Length::Fill),
            help_btn,
        ]
        .align_y(Alignment::Start),
        settings_section_card("运行开关", "决定是否启用协调总线以及主协调 agent 标识。"),
        settings_panel(column![enabled_row, settings_divider(), lead_agent_row].spacing(0)),
        settings_section_card("容量控制", "限制收件箱、死信、上下文与消息去重窗口。"),
        settings_panel(
            column![
                inbox_row,
                settings_divider(),
                dead_letters_row,
                settings_divider(),
                context_entries_row,
                settings_divider(),
                seen_ids_row,
            ]
            .spacing(0)
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    col.into()
}

/// 构建或处理 `view_overlays` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.coordination_settings;
    if !s.show_help_modal {
        return dialog;
    }

    let help_text = r#"协调配置说明

一、作用
- coordination 用于配置 delegate 任务的协调消息总线运行时。
- 这些参数控制消息留存上限、上下文补丁容量和去重窗口大小。

二、字段含义
1) enabled
- 是否启用协调总线能力。

2) lead_agent
- 逻辑主协调 agent 标识，通常为 delegate-lead。

3) max_inbox_messages_per_agent
- 每个注册 agent 保留的收件箱消息上限。

4) max_dead_letters
- 死信队列保留条目上限。

5) max_context_entries
- 共享上下文键值（ContextPatch）保留上限。

6) max_seen_message_ids
- 已处理消息 ID 去重窗口大小上限。

三、示例
{
  "coordination": {
    "enabled": true,
    "lead_agent": "delegate-lead",
    "max_inbox_messages_per_agent": 256,
    "max_dead_letters": 256,
    "max_context_entries": 512,
    "max_seen_message_ids": 4096
  }
}
"#;

    with_settings_help_modal(
        app,
        dialog,
        "Coordination 配置帮助",
        help_text,
        Message::Settings(message::SettingsMessage::CoordinationHelpClose),
    )
}
#[cfg(test)]
#[path = "system_settings_coordination_tests.rs"]
mod system_settings_coordination_tests;
