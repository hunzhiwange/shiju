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

    crate::app::components::system_settings_common::with_settings_help_modal(
        app,
        dialog,
        "Reliability 配置帮助",
        RELIABILITY_HELP_TEXT,
        Message::Settings(message::SettingsMessage::ReliabilityHelpClose),
    )
}
#[cfg(test)]
#[path = "system_settings_reliability_tests.rs"]
mod system_settings_reliability_tests;
