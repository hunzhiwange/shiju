//! Agents IPC 设置页面的视图组件
//!
//! 本模块提供跨进程 Agent IPC（Inter-Process Communication）配置的 UI 视图。
//! 用户可以通过此界面配置共享数据库中的 `agents_ipc` 字段，相关读写统一经过 Gateway，
//! 实现同一主机上多个 VibeWindow 进程间的发现与消息交换。
//!
//! # 功能概述
//!
//! - **启用/禁用开关**：控制是否开启跨进程 Agent IPC
//! - **数据库路径配置**：设置共享 SQLite 文件路径，多个进程需指向同一文件
//! - **离线阈值设置**：定义 Agent 心跳超时判定时间
//! - **帮助弹窗**：提供详细的配置说明文档
//!
//! # 相关配置字段
//!
//! - `enabled`: 是否启用进程间通信
//! - `db_path`: 共享 SQLite 文件路径
//! - `staleness_secs`: 离线判定阈值（秒）

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_help_button, settings_muted_text_style,
    settings_page_intro, settings_panel, settings_section_card, settings_text_input_style,
    settings_value_badge, with_settings_help_modal,
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

fn enabled_toggled(value: bool) -> Message {
    Message::Settings(message::SettingsMessage::AgentsIpcEnabledToggled(value))
}

fn db_path_changed(value: String) -> Message {
    Message::Settings(message::SettingsMessage::AgentsIpcDbPathChanged(value))
}

fn staleness_secs_changed(value: f32) -> Message {
    Message::Settings(message::SettingsMessage::AgentsIpcStalenessSecsChanged(value.round() as u64))
}

/// 构建 Agents IPC 设置页面的 UI 视图
///
/// 该函数渲染完整的 Agents IPC 配置界面，包括：
/// - 标题和帮助按钮
/// - 启用开关（checkbox）
/// - 数据库路径输入框
/// - 离线阈值滑块
/// - 保存错误提示（如有）
/// - 帮助弹窗（可展开）
///
/// # 参数
///
/// - `app`: 应用状态引用，包含 `agents_ipc_settings` 配置数据
///
/// # 返回值
///
/// 返回一个 Iced `Element`，可直接渲染为 UI 组件
///
/// # UI 结构
///
/// ```text
/// ┌─────────────────────────────────────────────────┐
/// │ 代理通信配置                         [?]     │
/// ├─────────────────────────────────────────────────┤
/// │ 启用         [✓] 开启跨进程 Agent IPC            │
/// │ 数据库路径   [________________________]          │
/// │ 离线阈值     [━━━━━●━━━━━━] 300 s               │
/// └─────────────────────────────────────────────────┘
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.agents_ipc_settings;
    let help_btn =
        settings_help_button(Message::Settings(message::SettingsMessage::AgentsIpcHelpOpen));

    let enabled_row = field_row(
        "启用",
        "控制是否启用跨进程 Agent IPC。",
        checkbox(s.enabled)
            .label("开启跨进程 Agent IPC")
            .on_toggle(enabled_toggled)
            .style(settings_checkbox_style),
    );

    let db_path_row = field_row(
        "数据库路径",
        "多个进程需指向同一 SQLite 文件。",
        text_input(vw_config_types::paths::AGENTS_IPC_DB_PATH, &s.db_path_input)
            .on_input(db_path_changed)
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    );

    let staleness_slider = slider(1.0..=86_400.0, s.staleness_secs as f32, staleness_secs_changed)
        .width(Length::Fixed(280.0));

    let staleness_row = field_row(
        "离线阈值",
        "最近心跳超过该秒数的 Agent 会被视为离线。",
        row![staleness_slider, settings_value_badge(format!("{} s", s.staleness_secs))]
            .spacing(16)
            .align_y(Alignment::Center),
    );

    let col = column![
        row![
            container(settings_page_intro(
                "代理通信配置",
                "配置同一主机上多个 VibeWindow 进程之间的 IPC 发现与通信。"
            ))
            .width(Length::Fill),
            help_btn
        ]
        .align_y(Alignment::Start),
        settings_section_card("基础行为", "控制 IPC 开关、共享数据库路径和离线阈值。"),
        settings_panel(column![enabled_row, db_path_row, staleness_row].spacing(0)),
    ]
    .spacing(16)
    .width(Length::Fill);

    col.into()
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.agents_ipc_settings;
    if !s.show_help_modal {
        return dialog;
    }

    #[cfg(debug_assertions)]
    let help_text = r#"代理通信配置说明

一、作用
- agents_ipc 用于同一主机上多个 VibeWindow 进程间的发现与消息交换。
- 启用后会注册 agents_list / agents_send / agents_inbox 等工具。

二、字段含义
1) enabled
- 是否启用进程间通信。

2) db_path
- 共享 SQLite 文件路径。多个进程需指向同一个文件。

3) staleness_secs
- 最近心跳超过该秒数的 Agent 会被视为离线。

三、示例
{
    "agents_ipc": {
        "enabled": true,
        "db_path": "~/.vibewindowdev/agents.db",
        "staleness_secs": 300
    }
}
"#;

    #[cfg(not(debug_assertions))]
    let help_text = r#"代理通信配置说明

一、作用
- agents_ipc 用于同一主机上多个 VibeWindow 进程间的发现与消息交换。
- 启用后会注册 agents_list / agents_send / agents_inbox 等工具。

二、字段含义
1) enabled
- 是否启用进程间通信。

2) db_path
- 共享 SQLite 文件路径。多个进程需指向同一个文件。

3) staleness_secs
- 最近心跳超过该秒数的 Agent 会被视为离线。

三、示例
{
    "agents_ipc": {
        "enabled": true,
        "db_path": "~/.vibewindow/agents.db",
        "staleness_secs": 300
    }
}
"#;

    with_settings_help_modal(
        app,
        dialog,
        "Agents IPC 配置帮助",
        help_text,
        Message::Settings(message::SettingsMessage::AgentsIpcHelpClose),
    )
}
#[cfg(test)]
#[path = "system_settings_agents_ipc_tests.rs"]
mod system_settings_agents_ipc_tests;
