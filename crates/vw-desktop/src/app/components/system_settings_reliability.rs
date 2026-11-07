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
    settings_divider, settings_group_card, settings_group_heading,
    settings_help_button_with_locale, settings_modal_header, settings_page_heading, settings_row,
    settings_value_badge, with_settings_help_modal,
};
use crate::app::i18n::I18n;
use crate::app::{App, Message, message};
use iced::widget::{column, container, row, slider};
use iced::{Alignment, Element, Length};
use std::ops::RangeInclusive;

const RELIABILITY_HELP_TEXT: &str = r#"可靠性配置说明

1) provider_retries
- 影响桌面聊天、Gateway、Channel、Task Plan 和 ACP 发起的模型请求。
- 首次模型请求失败后，仅遇到可重试的瞬时错误时额外重试；总尝试次数为该值加 1。
- 重试次数越大，短暂故障下成功机会越高，但最终失败前可能等待更久；设为 0 表示不额外重试。
- Provider 设置保存后对新发起的模型请求生效，已在执行或等待中的请求不变。

2) provider_backoff_ms
- 控制上述模型请求首次重试前的等待毫秒数，后续重试采用指数退避，并受最大等待上限约束。
- 数值越大可减轻故障时的请求压力，但恢复响应更慢；数值越小重试更快，但可能更快再次触发限流。

3) channel_initial_backoff_secs
- 聊天通道监听断开后，控制首次重新连接前等待的秒数。
- 数值越大可减少频繁重连，数值越小恢复更快；必须不大于频道最大退避。
- 频道退避设置在聊天通道监听器下次启动或重启时生效。

4) channel_max_backoff_secs
- 限制聊天通道连续重连失败时，后续重连等待增长到的最大秒数。
- 数值越大可降低长期故障时的重连压力，数值越小会更频繁尝试恢复。

5) scheduler_poll_secs
- 控制调度器执行 cron 扫描、发现到期 cron 任务的间隔秒数。
- 数值越小扫描越频繁、任务发现更及时但轮询开销更高；数值越大开销更低但触发延迟可能更长。
- 调度设置在 cron 调度器下次启动或重启时生效。

6) scheduler_retries
- cron 任务执行失败后，控制额外重试次数。
- 数值越大可提高瞬时失败后的成功机会，但会增加失败任务的总执行时间；设为 0 表示不额外重试。

默认配置示例
{
  "reliability": {
    "provider_retries": 3,
    "provider_backoff_ms": 500,
    "channel_initial_backoff_secs": 1,
    "channel_max_backoff_secs": 60,
    "scheduler_poll_secs": 60,
    "scheduler_retries": 3
  }
}
"#;

fn slider_row<'a>(
    label: &'a str,
    description: &'a str,
    range: RangeInclusive<f32>,
    value: f32,
    display_value: impl ToString,
    on_change: fn(f32) -> Message,
) -> Element<'a, Message> {
    settings_row(
        label,
        description,
        280.0,
        row![
            slider(range, value, on_change).width(Length::Fill),
            // Keep the rail steady as the number of digits changes during a drag.
            container(settings_value_badge(display_value)).align_right(88.0),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
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

/// 渲染模型请求、频道重连和调度任务的重试设置。
pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.reliability_settings;
    let i18n = I18n::new(app.desktop_locale);
    let help = settings_help_button_with_locale(
        app.desktop_locale,
        Message::Settings(message::SettingsMessage::ReliabilityHelpOpen),
    );

    let provider_retries = slider_row(
        i18n.__("Provider 重试"),
        i18n.__("请求失败时的最大重试次数。"),
        0.0..=20.0,
        s.provider_retries as f32,
        s.provider_retries,
        provider_retries_changed,
    );
    let provider_backoff = slider_row(
        i18n.__("Provider 退避"),
        i18n.__("重试之间的基础退避时间。"),
        0.0..=60_000.0,
        s.provider_backoff_ms as f32,
        format!("{} ms", s.provider_backoff_ms),
        provider_backoff_ms_changed,
    );
    let channel_initial = slider_row(
        i18n.__("频道初始退避"),
        i18n.__("频道或守护进程重连时的初始退避秒数。"),
        1.0..=3600.0,
        s.channel_initial_backoff_secs as f32,
        format!("{} s", s.channel_initial_backoff_secs),
        channel_initial_backoff_secs_changed,
    );
    let channel_max = slider_row(
        i18n.__("频道最大退避"),
        i18n.__("退避算法允许增长到的最大等待时间。"),
        s.channel_initial_backoff_secs as f32..=3600.0,
        s.channel_max_backoff_secs as f32,
        format!("{} s", s.channel_max_backoff_secs),
        channel_max_backoff_secs_changed,
    );
    let scheduler_poll = slider_row(
        i18n.__("调度轮询间隔"),
        i18n.__("调度器检查任务的轮询频率。"),
        1.0..=3600.0,
        s.scheduler_poll_secs as f32,
        format!("{} s", s.scheduler_poll_secs),
        scheduler_poll_secs_changed,
    );
    let scheduler_retries = slider_row(
        i18n.__("调度重试次数"),
        i18n.__("任务执行失败时允许的重试次数。"),
        0.0..=20.0,
        s.scheduler_retries as f32,
        s.scheduler_retries,
        scheduler_retries_changed,
    );

    column![
        settings_modal_header(
            settings_page_heading(
                i18n.__("可靠性配置"),
                i18n.__("统一配置 provider 重试、频道重连退避和调度重试策略。"),
            ),
            help,
        ),
        settings_group_heading("Provider", i18n.__("控制模型请求失败时的重试次数和退避时间。")),
        settings_group_card(column![provider_retries, settings_divider(), provider_backoff]),
        settings_group_heading(i18n.__("频道与调度"), i18n.__("控制频道重连与调度器重试节奏。")),
        settings_group_card(column![
            channel_initial,
            settings_divider(),
            channel_max,
            settings_divider(),
            scheduler_poll,
            settings_divider(),
            scheduler_retries,
        ]),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.reliability_settings;
    if !s.show_help_modal {
        return dialog;
    }

    with_settings_help_modal(
        app,
        dialog,
        I18n::new(app.desktop_locale).__("Reliability 配置帮助"),
        RELIABILITY_HELP_TEXT,
        Message::Settings(message::SettingsMessage::ReliabilityHelpClose),
    )
}
#[cfg(test)]
#[path = "system_settings_reliability_tests.rs"]
mod system_settings_reliability_tests;

#[cfg(test)]
#[path = "system_settings_reliability_ui_refresh_21_tests.rs"]
mod ui_refresh_21_tests;
