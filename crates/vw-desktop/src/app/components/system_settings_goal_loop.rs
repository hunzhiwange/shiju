//! 系统设置中 goal loop 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_divider, settings_muted_text_style,
    settings_page_intro, settings_panel, settings_section_card, settings_text_input_style,
};
use crate::app::message::settings::GoalLoopMessage;
use crate::app::{App, Message, message};
use iced::widget::{checkbox, column, container, row, text, text_input};
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
    let s = &app.goal_loop_settings;

    let enabled_row = field_row(
        "启用",
        "控制是否开启周期性的目标循环。",
        checkbox(s.enabled)
            .label("开启目标循环")
            .on_toggle(|value| {
                Message::Settings(message::SettingsMessage::GoalLoop(
                    GoalLoopMessage::EnabledToggled(value),
                ))
            })
            .style(settings_checkbox_style),
    );

    let interval_row = text_row(
        "间隔分钟",
        "两次目标循环之间的间隔分钟数。",
        "正整数，例如 10",
        &s.interval_minutes_input,
        |value| {
            Message::Settings(message::SettingsMessage::GoalLoop(
                GoalLoopMessage::IntervalMinutesChanged(value),
            ))
        },
    );

    let step_timeout_row = text_row(
        "步骤超时",
        "单步执行的最长等待时间。",
        "秒，例如 120",
        &s.step_timeout_secs_input,
        |value| {
            Message::Settings(message::SettingsMessage::GoalLoop(
                GoalLoopMessage::StepTimeoutSecsChanged(value),
            ))
        },
    );

    let max_steps_row = text_row(
        "单轮最大步数",
        "限制单次循环内的最大执行步数。",
        "正整数，例如 3",
        &s.max_steps_per_cycle_input,
        |value| {
            Message::Settings(message::SettingsMessage::GoalLoop(
                GoalLoopMessage::MaxStepsPerCycleChanged(value),
            ))
        },
    );

    let channel_row = text_row(
        "通道",
        "可选的结果投递通道。",
        "可选，例如 telegram",
        &s.channel_input,
        |value| {
            Message::Settings(message::SettingsMessage::GoalLoop(GoalLoopMessage::ChannelChanged(
                value,
            )))
        },
    );

    let target_row = text_row(
        "目标",
        "可选的通道目标，例如 chat_id。",
        "可选，例如 chat_id",
        &s.target_input,
        |value| {
            Message::Settings(message::SettingsMessage::GoalLoop(GoalLoopMessage::TargetChanged(
                value,
            )))
        },
    );

    let hint_row = row![
        container(text(" ")).width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
        text("channel 和 target 留空时仅执行目标循环，不额外投递事件。")
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let content = column![
        settings_page_intro("目标循环配置", "配置周期执行的间隔、步数上限和可选投递目标。"),
        settings_section_card("基础行为", "控制目标循环是否启用以及调度节奏。"),
        settings_panel(
            column![
                enabled_row,
                settings_divider(),
                interval_row,
                settings_divider(),
                step_timeout_row,
                settings_divider(),
                max_steps_row,
            ]
            .spacing(0),
        ),
        settings_section_card("投递目标", "可选配置外部通道与接收目标。"),
        settings_panel(column![channel_row, settings_divider(), target_row].spacing(0)),
        hint_row,
    ]
    .spacing(16)
    .width(Length::Fill);

    content.into()
}
#[cfg(test)]
#[path = "system_settings_goal_loop_tests.rs"]
mod system_settings_goal_loop_tests;
