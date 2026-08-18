//! 系统设置 - 代理配置视图模块
//!
//! 本模块提供代理配置设置的用户界面组件，用于配置 VibeWindow 的网络代理。
//!
//! # 功能特性
//!
//! - 支持启用/禁用代理配置
//! - 配置 HTTP、HTTPS、ALL 三种代理地址
//! - 设置代理排除列表（NO_PROXY）
//! - 选择代理生效范围（environment/vibewindow/services）
//! - 为特定服务选择器配置代理
//! - 显示帮助模态窗口
//!
//! # 配置作用域
//!
//! - `environment`: 仅使用系统环境变量
//! - `vibewindow`: 所有 VibeWindow 管理的 HTTP 流量
//! - `services`: 仅对指定服务选择器生效

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_divider, settings_help_button,
    settings_muted_text_style, settings_page_intro, settings_pick_list_menu_style,
    settings_pick_list_style, settings_section_card, settings_section_content,
    settings_text_input_style,
};
use crate::app::{App, Message, message};
use iced::widget::{checkbox, column, container, pick_list, row, text, text_input};
use iced::{Alignment, Element, Length};
use vw_config_types::proxy::ProxyScope;

const PROXY_SCOPE_OPTIONS: [&str; 3] = ["系统环境", "VibeWindow", "指定服务"];

fn proxy_scope_label(scope: ProxyScope) -> &'static str {
    match scope {
        ProxyScope::Environment => "系统环境",
        ProxyScope::Services => "指定服务",
        ProxyScope::Vibewindow => "VibeWindow",
    }
}

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

pub fn view(app: &App) -> Element<'_, Message> {
    column![
        settings_page_intro(
            "代理配置",
            "配置 HTTP、HTTPS、SOCKS5 代理以及生效范围。",
        ),
        content(app),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

pub(super) fn content(app: &App) -> Element<'_, Message> {
    let s = &app.proxy_settings;

    let help_btn = settings_help_button(Message::Settings(message::SettingsMessage::ProxyHelpOpen));

    let enabled_row = field_row(
        "启用",
        "控制是否启用代理能力。",
        checkbox(s.enabled)
            .label("启用代理")
            .on_toggle(|v| Message::Settings(message::SettingsMessage::ProxyEnabledToggled(v)))
            .style(settings_checkbox_style),
    );

    let scope_pick = pick_list(PROXY_SCOPE_OPTIONS, Some(proxy_scope_label(s.scope)), |v| {
        Message::Settings(message::SettingsMessage::ProxyScopeTextChanged(v.to_string()))
    })
    .padding([10, 14])
    .text_size(13)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fixed(280.0));

    let scope_row = field_row("生效范围", "决定代理应用到环境、全局流量或指定服务。", scope_pick);

    let http_row = field_row(
        "HTTP 代理",
        "HTTP 请求代理地址。",
        text_input("http://127.0.0.1:7890", &s.http_proxy)
            .on_input(|v| Message::Settings(message::SettingsMessage::ProxyHttpChanged(v)))
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    );

    let https_row = field_row(
        "HTTPS 代理",
        "HTTPS 请求代理地址。",
        text_input("http://127.0.0.1:7890", &s.https_proxy)
            .on_input(|v| Message::Settings(message::SettingsMessage::ProxyHttpsChanged(v)))
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    );

    let all_row = field_row(
        "ALL 代理",
        "当未命中 HTTP/HTTPS 时使用的兜底代理。",
        text_input("socks5://127.0.0.1:7891", &s.all_proxy)
            .on_input(|v| Message::Settings(message::SettingsMessage::ProxyAllChanged(v)))
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    );

    let no_proxy_row = field_row(
        "排除列表",
        "支持逗号或换行分隔，格式与 NO_PROXY 一致。",
        text_input("localhost, 127.0.0.1, *.internal", &s.no_proxy_input)
            .on_input(|v| Message::Settings(message::SettingsMessage::ProxyNoProxyChanged(v)))
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    );

    let services_row = field_row(
        "服务选择器",
        "仅在范围为指定服务时生效。",
        text_input("provider.openai, tool.http_request", &s.services_input)
            .on_input(|v| Message::Settings(message::SettingsMessage::ProxyServicesChanged(v)))
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    );

    let col = column![
        row![container(text("代理设置").size(16)).width(Length::Fill), help_btn]
            .align_y(Alignment::Center),
        settings_section_card("基础行为", "控制代理开关与作用域选择。"),
        settings_section_content(column![enabled_row, settings_divider(), scope_row].spacing(0)),
        settings_section_card("代理地址", "分别配置 HTTP、HTTPS 与兜底代理地址。"),
        settings_section_content(
            column![http_row, settings_divider(), https_row, settings_divider(), all_row]
                .spacing(0),
        ),
        settings_section_card("排除与路由", "配置 NO_PROXY 和指定服务选择器。"),
        settings_section_content(
            column![no_proxy_row, settings_divider(), services_row].spacing(0),
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    col.into()
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.proxy_settings;
    if !s.show_help_modal {
        return dialog;
    }

    let help_text = r#"代理配置说明

一、作用
- proxy 用于配置 VibeWindow 的 HTTP/HTTPS/SOCKS5 代理。
- scope 决定代理生效范围。

二、字段含义
1) enabled
- 是否启用代理。

2) http_proxy / https_proxy / all_proxy
- 分别对应 HTTP、HTTPS 或兜底代理地址。
- 支持 http/https/socks5/socks5h。

3) no_proxy
- 代理排除列表，与 NO_PROXY 格式一致。

4) scope
- environment：仅使用系统环境变量。
- vibewindow：所有 VibeWindow 管理的 HTTP 流量。
- services：仅对指定服务选择器生效。

5) services
- scope=services 时使用。
- 例如：provider.openai、tool.http_request、channel.telegram。

三、示例
{
  "proxy": {
    "enabled": false,
    "http_proxy": null,
    "https_proxy": null,
    "all_proxy": null,
    "no_proxy": [],
    "scope": "vibewindow",
    "services": []
  }
}
"#;

    crate::app::components::system_settings_common::with_settings_help_modal(
        app,
        dialog,
        "Proxy 配置帮助",
        help_text,
        Message::Settings(message::SettingsMessage::ProxyHelpClose),
    )
}
#[cfg(test)]
#[path = "system_settings_proxy_tests.rs"]
mod system_settings_proxy_tests;
