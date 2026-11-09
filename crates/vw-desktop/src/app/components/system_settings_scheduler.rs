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
    icon_btn, settings_checkbox_style, settings_close_button, settings_divider,
    settings_flexible_row_with_help, settings_group_card, settings_group_heading_with_help,
    settings_help_button_with_locale as settings_help_button, settings_help_copy_state,
    settings_modal_header, settings_muted_text_style, settings_page_heading_with_help,
    settings_panel, settings_value_badge,
};
use crate::app::i18n::I18n;
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
    app: &App,
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    settings_flexible_row_with_help(
        i18n,
        &app.app_theme,
        i18n.__(label),
        i18n.__(description),
        320.0,
        control,
    )
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
        let (copy_icon, copy_tip) = settings_help_copy_state(
            app.last_copied_code_hash,
            crate::app::i18n::I18n::new(app.desktop_locale).__(SCHEDULER_HELP),
        );
        let copy_btn = icon_btn(
            copy_icon,
            crate::app::i18n::I18n::new(app.desktop_locale).__(copy_tip),
            Some(Message::CopyCode(
                crate::app::i18n::I18n::new(app.desktop_locale).__(SCHEDULER_HELP).to_string(),
            )),
        );

        settings_panel(
            column![
                settings_modal_header(
                    text(crate::app::i18n::I18n::new(app.desktop_locale).__("Scheduler 配置帮助"))
                        .size(16),
                    row![copy_btn, close_btn].spacing(10).align_y(Alignment::Center),
                ),
                text(crate::app::i18n::I18n::new(app.desktop_locale).__(SCHEDULER_HELP))
                    .size(13)
                    .style(settings_muted_text_style),
            ]
            .spacing(12)
            .width(Length::Fill),
        )
        .into()
    })
}

/// 构建可嵌入到定时任务配置页的调度器设置内容。
pub(crate) fn embedded_view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let s = &app.scheduler_settings;
    let help_btn = settings_help_button(
        app.desktop_locale,
        Message::Settings(message::SettingsMessage::SchedulerHelpOpen),
    );

    let enabled_row = field_row(
        app,
        "启用",
        "控制是否启用内置调度器主循环。",
        checkbox(s.enabled)
            .label(i18n.__("开启内置调度器"))
            .on_toggle(enabled_toggled)
            .style(settings_checkbox_style),
    );

    let max_tasks_slider =
        slider(1.0..=10_000.0, s.max_tasks as f32, max_tasks_changed).width(Length::Fill);

    let max_tasks_row = field_row(
        app,
        "最大任务",
        "单次轮询最多处理的任务数量。",
        row![max_tasks_slider, container(settings_value_badge(s.max_tasks)).align_right(72.0)]
            .spacing(12)
            .align_y(Alignment::Center),
    );

    let max_concurrent_slider =
        slider(1.0..=100.0, s.max_concurrent as f32, max_concurrent_changed).width(Length::Fill);

    let max_concurrent_row = settings_flexible_row_with_help(
        i18n,
        &app.app_theme,
        i18n.__("并发上限"),
        format!(
            "{}\n{}",
            i18n.__("允许并行执行的任务数量上限。"),
            i18n.__("建议 max_tasks=64、max_concurrent=4；高并发场景可按资源逐步上调。"),
        ),
        320.0,
        row![
            max_concurrent_slider,
            container(settings_value_badge(s.max_concurrent)).align_right(72.0)
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    );

    let mut col = column![
        row![text(i18n.__("内置调度器")).size(16), Space::new().width(Length::Fill), help_btn]
            .align_y(Alignment::Start),
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(help_panel) = inline_help_panel(app) {
        col = col.push(help_panel);
    }

    col = col
        .push(settings_group_heading_with_help(
            i18n,
            &app.app_theme,
            i18n.__("执行能力"),
            i18n.__("控制调度器处理任务的吞吐和并发上限。"),
        ))
        .push(settings_group_card(column![
            enabled_row,
            settings_divider(),
            max_tasks_row,
            settings_divider(),
            max_concurrent_row
        ]));

    col.into()
}

/// 构建独立调度器设置视图。
///
/// 保留该入口供组件测试和嵌入前的调用方使用；系统设置导航会将内容展示在定时任务配置中。
pub fn view(app: &App) -> Element<'_, Message> {
    let i18n = I18n::new(app.desktop_locale);
    column![
        settings_page_heading_with_help(
            i18n,
            &app.app_theme,
            i18n.__("调度配置"),
            i18n.__("配置内置调度器的开关、吞吐和并发能力。")
        ),
        embedded_view(app),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
#[path = "system_settings_scheduler_tests.rs"]
mod system_settings_scheduler_tests;
