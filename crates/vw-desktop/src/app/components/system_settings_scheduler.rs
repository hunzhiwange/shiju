//! 调度器设置视图组件
//!
//! 本模块提供调度器（Scheduler）配置的图形化设置界面，允许用户通过 UI 调整调度器的运行参数。
//!
//! ## 主要功能
//!
//! - 启用/禁用内置调度器
//! - 配置单次轮询最大任务数量
//! - 配置最大并发执行任务数量
//! - 提供配置帮助说明的模态对话框
//!
//! ## 配置影响范围
//!
//! 这些设置会通过 Gateway 写入共享数据库的 `scheduler` 字段，
//! 影响定时任务的拉取数量与并发执行能力。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_help_button, settings_muted_text_style,
    settings_page_intro, settings_panel, settings_section_card, settings_value_badge,
};
use crate::app::{App, Message, message};
use iced::widget::{checkbox, column, container, row, slider, text};
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
    Message::Settings(message::SettingsMessage::SchedulerEnabledToggled(value))
}

fn max_tasks_changed(value: f32) -> Message {
    Message::Settings(message::SettingsMessage::SchedulerMaxTasksChanged(value.round() as u32))
}

fn max_concurrent_changed(value: f32) -> Message {
    Message::Settings(message::SettingsMessage::SchedulerMaxConcurrentChanged(value.round() as u32))
}

/// 构建调度器设置视图
///
/// 该函数创建一个完整的调度器配置界面，包含开关选项、滑块控件和帮助模态框。
///
/// # 参数
///
/// - `app`: 应用程序状态引用，从中读取 `scheduler_settings` 获取当前配置
///
/// # 返回值
///
/// 返回一个 `Element`，包含调度器设置的完整 UI 组件
///
/// # UI 结构
///
/// 1. 标题行：包含标题文本和帮助按钮
/// 2. 启用开关：控制调度器是否运行
/// 3. 最大任务数滑块：配置单次轮询最多处理的任务数量
/// 4. 并发上限滑块：配置允许并行执行的任务数量
/// 5. 配置建议提示
/// 6. 可选的保存错误提示
/// 7. 可选的帮助模态对话框
pub fn view(app: &App) -> Element<'_, Message> {
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
        row![max_concurrent_slider, settings_value_badge(format!("{}", s.max_concurrent))]
            .spacing(16)
            .align_y(Alignment::Center),
    );

    let hint_row = row![
        container(text("")).width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
        text("建议 max_tasks=64、max_concurrent=4；高并发场景可按资源逐步上调。")
            .size(12)
            .style(settings_muted_text_style),
    ]
    .spacing(16)
    .align_y(Alignment::Center);

    let col = column![
        row![
            container(settings_page_intro("调度配置", "配置内置调度器的开关、吞吐和并发能力。"))
                .width(Length::Fill),
            help_btn
        ]
        .align_y(Alignment::Start),
        settings_section_card("执行能力", "控制调度器处理任务的吞吐和并发上限。"),
        settings_panel(column![enabled_row, max_tasks_row, max_concurrent_row].spacing(0)),
        hint_row,
    ]
    .spacing(16)
    .width(Length::Fill);

    col.into()
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.scheduler_settings;
    if !s.show_help_modal {
        return dialog;
    }

    let help_text = r#"调度配置说明

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

    crate::app::components::system_settings_common::with_settings_help_modal(
        app,
        dialog,
        "Scheduler 配置帮助",
        help_text,
        Message::Settings(message::SettingsMessage::SchedulerHelpClose),
    )
}
#[cfg(test)]
#[path = "system_settings_scheduler_tests.rs"]
mod system_settings_scheduler_tests;
