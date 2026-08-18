//! 调度器设置视图组件
//!
//! 本模块提供调度器（Scheduler）配置的图形化设置界面，允许用户通过 UI 调整调度器的运行参数。
//!
//! ## 主要功能
//!
//! - 启用/禁用内置调度器
//! - 配置单次轮询最大任务数量
//! - 配置最大并发执行任务数量
//! - 提供配置帮助说明的面板内卡片
//!
//! ## 配置影响范围
//!
//! 这些设置会通过 Gateway 写入共享数据库的 `scheduler` 字段，
//! 影响定时任务的拉取数量与并发执行能力。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, icon_btn, settings_checkbox_style, settings_close_button,
    settings_control_with_hint, settings_help_button, settings_help_copy_state,
    settings_muted_text_style, settings_page_intro, settings_panel, settings_section_card,
    settings_section_content, settings_value_badge,
};
use crate::app::{App, Message, message};
use iced::widget::{Space, checkbox, column, container, row, slider, text};
use iced::{Alignment, Element, Length};

const SCHEDULER_HELP: &str = r#"调度配置说明

一、作用
- scheduler 用于控制内置调度器的开关与吞吐上限。
- 该配置会影响定时任务的拉取数量与并发执行能力。

二、字段含义
1) enabled
- 是否启用调度器主循环。

2) max_tasks
- 单次轮询最多处理的任务数量上限。

3) max_concurrent
- 单次轮询内允许并行执行的任务数量上限。

三、示例
{
  "scheduler": {
    "enabled": true,
    "max_tasks": 64,
    "max_concurrent": 4
  }
}

四、建议
- 若任务执行时间较长，优先增大 max_tasks，不要一次性把 max_concurrent 调太高。
- 出现资源竞争（CPU/IO）时，适当下调 max_concurrent。
"#;

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
    Message::Settings(message::SettingsMessage::SchedulerEnabledToggled(value))
}

fn max_tasks_changed(value: f32) -> Message {
    Message::Settings(message::SettingsMessage::SchedulerMaxTasksChanged(value.round() as u32))
}

fn max_concurrent_changed(value: f32) -> Message {
    Message::Settings(message::SettingsMessage::SchedulerMaxConcurrentChanged(value.round() as u32))
}

fn inline_help_panel(app: &App) -> Option<Element<'static, Message>> {
    app.scheduler_settings.show_help_modal.then(|| {
        let close_message = Message::Settings(message::SettingsMessage::SchedulerHelpClose);
        let close_btn = settings_close_button(close_message);
        let (copy_icon, copy_tip) =
            settings_help_copy_state(app.last_copied_code_hash, SCHEDULER_HELP);
        let copy_btn =
            icon_btn(copy_icon, copy_tip, Some(Message::CopyCode(SCHEDULER_HELP.to_string())));

        settings_panel(
            column![
                row![
                    text("Scheduler 配置帮助").size(16),
                    Space::new().width(Length::Fill),
                    copy_btn,
                    close_btn
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                text(SCHEDULER_HELP).size(13).style(settings_muted_text_style),
            ]
            .spacing(12)
            .width(Length::Fill),
        )
        .into()
    })
}

/// 构建可嵌入到定时任务配置页的调度器设置内容。
pub(crate) fn embedded_view(app: &App) -> Element<'_, Message> {
    let s = &app.scheduler_settings;
    let help_btn =
        settings_help_button(Message::Settings(message::SettingsMessage::SchedulerHelpOpen));

    let enabled_row = field_row(
        "启用",
        "控制是否启用内置调度器主循环。",
        checkbox(s.enabled)
            .label("开启内置调度器")
            .on_toggle(enabled_toggled)
            .style(settings_checkbox_style),
    );

    let max_tasks_slider =
        slider(1.0..=10_000.0, s.max_tasks as f32, max_tasks_changed).width(Length::Fixed(280.0));

    let max_tasks_row = field_row(
        "最大任务",
        "单次轮询最多处理的任务数量。",
        row![max_tasks_slider, settings_value_badge(format!("{}", s.max_tasks))]
            .spacing(16)
            .align_y(Alignment::Center),
    );

    let max_concurrent_slider =
        slider(1.0..=100.0, s.max_concurrent as f32, max_concurrent_changed)
            .width(Length::Fixed(280.0));

    let max_concurrent_row = field_row(
        "并发上限",
        "允许并行执行的任务数量上限。",
        settings_control_with_hint(
            row![max_concurrent_slider, settings_value_badge(format!("{}", s.max_concurrent))]
                .spacing(16)
                .align_y(Alignment::Center),
            "建议 max_tasks=64、max_concurrent=4；高并发场景可按资源逐步上调。",
        ),
    );

    let mut col = column![
        row![
            text("内置调度器").size(16),
            Space::new().width(Length::Fill),
            help_btn
        ]
        .align_y(Alignment::Start),
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(help_panel) = inline_help_panel(app) {
        col = col.push(help_panel);
    }

    col = col
        .push(settings_section_card("执行能力", "控制调度器处理任务的吞吐和并发上限。"))
        .push(settings_section_content(
            column![enabled_row, max_tasks_row, max_concurrent_row].spacing(0),
        ));

    col.into()
}

/// 构建独立调度器设置视图。
///
/// 保留该入口供组件测试和嵌入前的调用方使用；系统设置导航会将内容展示在定时任务配置中。
pub fn view(app: &App) -> Element<'_, Message> {
    column![
        settings_page_intro("调度配置", "配置内置调度器的开关、吞吐和并发能力。"),
        embedded_view(app),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
#[path = "system_settings_scheduler_tests.rs"]
mod system_settings_scheduler_tests;
