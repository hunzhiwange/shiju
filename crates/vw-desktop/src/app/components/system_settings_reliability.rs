//! 系统设置 - 可靠性配置视图模块
//!
//! 本模块提供可靠性配置的用户界面，允许用户配置以下参数：
//! - Provider 重试策略：控制 AI 模型提供商请求失败时的重试行为
//! - Provider 退避时间：控制重试之间的基础等待时间
//! - 频道退避策略：控制通信频道（如 Telegram、Discord）重连时的退避时间
//! - 调度器配置：控制定时任务的轮询间隔和重试行为
//!
//! # 配置存储位置
//! 配置将通过 Gateway 保存到共享数据库的 `reliability` 字段中
//!
//! # 主要功能
//! - 提供直观的滑块控件调整各项参数
//! - 实时显示当前配置值
//! - 提供详细的帮助文档模态框说明各参数含义

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_help_button, settings_muted_text_style, settings_page_intro,
    settings_panel, settings_section_card, settings_value_badge,
};
use crate::app::{App, Message, message};
use iced::widget::{column, container, row, slider, text};
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

fn provider_retries_changed(v: f32) -> Message {
    Message::Settings(message::SettingsMessage::ReliabilityProviderRetriesChanged(v.round() as u32))
}

fn provider_backoff_ms_changed(v: f32) -> Message {
    Message::Settings(message::SettingsMessage::ReliabilityProviderBackoffMsChanged(
        v.round() as u64
    ))
}

fn channel_initial_backoff_secs_changed(v: f32) -> Message {
    Message::Settings(message::SettingsMessage::ReliabilityChannelInitialBackoffSecsChanged(
        v.round() as u64,
    ))
}

fn channel_max_backoff_secs_changed(v: f32) -> Message {
    Message::Settings(message::SettingsMessage::ReliabilityChannelMaxBackoffSecsChanged(
        v.round() as u64
    ))
}

fn scheduler_poll_secs_changed(v: f32) -> Message {
    Message::Settings(message::SettingsMessage::ReliabilitySchedulerPollSecsChanged(
        v.round() as u64
    ))
}

fn scheduler_retries_changed(v: f32) -> Message {
    Message::Settings(
        message::SettingsMessage::ReliabilitySchedulerRetriesChanged(v.round() as u32),
    )
}

/// 渲染可靠性配置设置视图
///
/// 创建包含多个配置项的表单界面，用户可以通过滑块调整各项可靠性参数。
/// 每个配置项都包含标签、滑块控件和当前值显示。
///
/// # 参数
/// - `app`: 应用状态引用，包含当前的可靠性配置数据
///
/// # 返回值
/// 返回一个 Iced Element，表示完整的可靠性配置界面
///
/// # 界面组成
/// 1. 标题和副标题区域
/// 2. Provider 重试次数配置
/// 3. Provider 退避时间配置
/// 4. 频道初始退避时间配置
/// 5. 频道最大退避时间配置
/// 6. 调度轮询间隔配置
/// 7. 调度重试次数配置
/// 8. 错误提示（如果有保存错误）
/// 9. 帮助模态框（可选显示）
pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.reliability_settings;
    let help_btn =
        settings_help_button(Message::Settings(message::SettingsMessage::ReliabilityHelpOpen));

    let provider_retries_slider =
        slider(0.0..=20.0, s.provider_retries as f32, provider_retries_changed)
            .width(Length::Fixed(280.0));

    let provider_retries_row = field_row(
        "Provider 重试",
        "请求失败时的最大重试次数。",
        row![provider_retries_slider, settings_value_badge(format!("{}", s.provider_retries)),]
            .spacing(16)
            .align_y(Alignment::Center),
    );

    let provider_backoff_slider =
        slider(0.0..=60_000.0, s.provider_backoff_ms as f32, provider_backoff_ms_changed)
            .width(Length::Fixed(280.0));

    let provider_backoff_row = field_row(
        "Provider 退避",
        "重试之间的基础退避时间。",
        row![
            provider_backoff_slider,
            settings_value_badge(format!("{} ms", s.provider_backoff_ms)),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    );

    let channel_initial_slider = slider(
        1.0..=3600.0,
        s.channel_initial_backoff_secs as f32,
        channel_initial_backoff_secs_changed,
    )
    .width(Length::Fixed(280.0));

    let channel_initial_row = field_row(
        "频道初始退避",
        "频道或守护进程重连时的初始退避秒数。",
        row![
            channel_initial_slider,
            settings_value_badge(format!("{} s", s.channel_initial_backoff_secs)),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    );

    let channel_max_slider = slider(
        s.channel_initial_backoff_secs as f32..=3600.0,
        s.channel_max_backoff_secs as f32,
        channel_max_backoff_secs_changed,
    )
    .width(Length::Fixed(280.0));

    let channel_max_row =
        field_row(
            "频道最大退避",
            "退避算法允许增长到的最大等待时间。",
            row![
                channel_max_slider,
                settings_value_badge(format!("{} s", s.channel_max_backoff_secs)),
            ]
            .spacing(16)
            .align_y(Alignment::Center),
        );

    let scheduler_poll_slider =
        slider(1.0..=3600.0, s.scheduler_poll_secs as f32, scheduler_poll_secs_changed)
            .width(Length::Fixed(280.0));

    let scheduler_poll_row = field_row(
        "调度轮询间隔",
        "调度器检查任务的轮询频率。",
        row![scheduler_poll_slider, settings_value_badge(format!("{} s", s.scheduler_poll_secs)),]
            .spacing(16)
            .align_y(Alignment::Center),
    );

    let scheduler_retries_slider =
        slider(0.0..=20.0, s.scheduler_retries as f32, scheduler_retries_changed)
            .width(Length::Fixed(280.0));

    let scheduler_retries_row = field_row(
        "调度重试次数",
        "任务执行失败时允许的重试次数。",
        row![scheduler_retries_slider, settings_value_badge(format!("{}", s.scheduler_retries)),]
            .spacing(16)
            .align_y(Alignment::Center),
    );

    let col = column![
        row![
            container(settings_page_intro(
                "可靠性配置",
                "统一配置 provider 重试、频道重连退避和调度重试策略。"
            ))
            .width(Length::Fill),
            help_btn
        ]
        .align_y(Alignment::Start),
        settings_section_card("Provider", "控制模型请求失败时的重试次数和退避时间。"),
        settings_panel(column![provider_retries_row, provider_backoff_row].spacing(0)),
        settings_section_card("频道与调度", "控制频道重连与调度器重试节奏。"),
        settings_panel(
            column![
                channel_initial_row,
                channel_max_row,
                scheduler_poll_row,
                scheduler_retries_row,
            ]
            .spacing(0),
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    col.into()
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.reliability_settings;
    if !s.show_help_modal {
        return dialog;
    }

    let help_text = r#"可靠性配置说明

一、作用
- reliability 控制 provider 重试、退避、频道重连退避、调度重试与轮询间隔。
- 这些参数影响故障恢复速度与系统稳定性。

二、字段含义
1) provider_retries
- provider 请求失败时的重试次数。

2) provider_backoff_ms
- provider 重试的基础退避毫秒。

3) channel_initial_backoff_secs
- 频道/守护重连的初始退避秒数。

4) channel_max_backoff_secs
- 频道/守护重连的最大退避秒数。

5) scheduler_poll_secs
- 调度器轮询间隔秒数。

6) scheduler_retries
- 调度任务执行失败时的重试次数。

三、示例
{
  "reliability": {
    "provider_retries": 2,
    "provider_backoff_ms": 500,
    "fallback_providers": [],
    "fallback_api_keys": {},
    "api_keys": [],
    "model_fallbacks": {},
    "channel_initial_backoff_secs": 2,
    "channel_max_backoff_secs": 60,
    "scheduler_poll_secs": 15,
    "scheduler_retries": 2
  }
}
"#;

    crate::app::components::system_settings_common::with_settings_help_modal(
        app,
        dialog,
        "Reliability 配置帮助",
        help_text,
        Message::Settings(message::SettingsMessage::ReliabilityHelpClose),
    )
}
#[cfg(test)]
#[path = "system_settings_reliability_tests.rs"]
mod system_settings_reliability_tests;
