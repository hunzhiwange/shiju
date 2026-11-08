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
    settings_checkbox_style, settings_divider, settings_group_card, settings_group_heading,
    settings_help_button_with_locale as settings_help_button, settings_page_heading,
    settings_row_with_flexible_control, settings_select, settings_text_input,
};
use crate::app::{App, Message, message};
use iced::widget::{checkbox, column, container, row, text};
use iced::{Alignment, Element, Length};
use vw_config_types::proxy::ProxyScope;

const PROXY_SCOPE_OPTIONS: [&str; 3] = ["系统环境", "ShiJu", "指定服务"];

fn proxy_scope_label(scope: ProxyScope) -> &'static str {
    match scope {
        ProxyScope::Environment => "系统环境",
        ProxyScope::Services => "指定服务",
        ProxyScope::Vibewindow => "ShiJu",
    }
}

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_row_with_flexible_control(label, description, 360.0, control)
}

pub fn view(app: &App) -> Element<'_, Message> {
    column![
        settings_page_heading(
            crate::app::i18n::I18n::new(app.desktop_locale).__("代理配置"),
            crate::app::i18n::I18n::new(app.desktop_locale)
                .__("配置 HTTP、HTTPS、SOCKS5 代理以及生效范围。"),
        ),
        content(app),
    ]
    .spacing(16)
    .width(Length::Fill)
    .into()
}

pub(super) fn content(app: &App) -> Element<'_, Message> {
    let s = &app.proxy_settings;

    let help_btn = settings_help_button(
        app.desktop_locale,
        Message::Settings(message::SettingsMessage::ProxyHelpOpen),
    );

    let enabled_row = field_row(
        crate::app::i18n::I18n::new(app.desktop_locale).__("启用"),
        crate::app::i18n::I18n::new(app.desktop_locale).__("控制是否启用代理能力。"),
        checkbox(s.enabled)
            .label(crate::app::i18n::I18n::new(app.desktop_locale).__("启用代理"))
            .on_toggle(|v| Message::Settings(message::SettingsMessage::ProxyEnabledToggled(v)))
            .style(settings_checkbox_style),
    );

    let scope_pick = settings_select(
        PROXY_SCOPE_OPTIONS.map(|v| crate::app::i18n::I18n::new(app.desktop_locale).__(v)),
        Some(crate::app::i18n::I18n::new(app.desktop_locale).__(proxy_scope_label(s.scope))),
        move |v| {
            Message::Settings(message::SettingsMessage::ProxyScopeTextChanged(
                PROXY_SCOPE_OPTIONS
                    .into_iter()
                    .find(|source| crate::app::i18n::I18n::new(app.desktop_locale).__(source) == v)
                    .expect("selected scope belongs to supplied options")
                    .to_string(),
            ))
        },
    )
    .width(Length::Fill);

    let scope_row = field_row(
        crate::app::i18n::I18n::new(app.desktop_locale).__("生效范围"),
        crate::app::i18n::I18n::new(app.desktop_locale)
            .__("决定代理应用到环境、全局流量或指定服务。"),
        scope_pick,
    );

    let http_row = field_row(
        crate::app::i18n::I18n::new(app.desktop_locale).__("HTTP 代理"),
        crate::app::i18n::I18n::new(app.desktop_locale).__("HTTP 请求代理地址。"),
        settings_text_input("http://127.0.0.1:7890", &s.http_proxy)
            .on_input(|v| Message::Settings(message::SettingsMessage::ProxyHttpChanged(v)))
            .width(Length::Fill),
    );

    let https_row = field_row(
        crate::app::i18n::I18n::new(app.desktop_locale).__("HTTPS 代理"),
        crate::app::i18n::I18n::new(app.desktop_locale).__("HTTPS 请求代理地址。"),
        settings_text_input("http://127.0.0.1:7890", &s.https_proxy)
            .on_input(|v| Message::Settings(message::SettingsMessage::ProxyHttpsChanged(v)))
            .width(Length::Fill),
    );

    let all_row = field_row(
        crate::app::i18n::I18n::new(app.desktop_locale).__("ALL 代理"),
        crate::app::i18n::I18n::new(app.desktop_locale)
            .__("当未命中 HTTP/HTTPS 时使用的兜底代理。"),
        settings_text_input("socks5://127.0.0.1:7891", &s.all_proxy)
            .on_input(|v| Message::Settings(message::SettingsMessage::ProxyAllChanged(v)))
            .width(Length::Fill),
    );

    let no_proxy_row = field_row(
        crate::app::i18n::I18n::new(app.desktop_locale).__("排除列表"),
        crate::app::i18n::I18n::new(app.desktop_locale)
            .__("支持逗号或换行分隔，格式与 NO_PROXY 一致。"),
        settings_text_input("localhost, 127.0.0.1, *.internal", &s.no_proxy_input)
            .on_input(|v| Message::Settings(message::SettingsMessage::ProxyNoProxyChanged(v)))
            .width(Length::Fill),
    );

    let services_row = field_row(
        crate::app::i18n::I18n::new(app.desktop_locale).__("服务选择器"),
        crate::app::i18n::I18n::new(app.desktop_locale).__("仅在范围为指定服务时生效。"),
        settings_text_input("provider.openai, tool.http_request", &s.services_input)
            .on_input(|v| Message::Settings(message::SettingsMessage::ProxyServicesChanged(v)))
            .width(Length::Fill),
    );

    let col = column![
        row![
            container(
                text(crate::app::i18n::I18n::new(app.desktop_locale).__("代理设置")).size(16)
            )
            .width(Length::Fill),
            help_btn
        ]
        .align_y(Alignment::Center),
        settings_group_heading(
            crate::app::i18n::I18n::new(app.desktop_locale).__("基础行为"),
            crate::app::i18n::I18n::new(app.desktop_locale).__("控制代理开关与作用域选择。")
        ),
        settings_group_card(column![enabled_row, settings_divider(), scope_row].spacing(0)),
        settings_group_heading(
            crate::app::i18n::I18n::new(app.desktop_locale).__("代理地址"),
            crate::app::i18n::I18n::new(app.desktop_locale)
                .__("分别配置 HTTP、HTTPS 与兜底代理地址。")
        ),
        settings_group_card(
            column![http_row, settings_divider(), https_row, settings_divider(), all_row]
                .spacing(0),
        ),
        settings_group_heading(
            crate::app::i18n::I18n::new(app.desktop_locale).__("排除与路由"),
            crate::app::i18n::I18n::new(app.desktop_locale).__("配置 NO_PROXY 和指定服务选择器。")
        ),
        settings_group_card(column![no_proxy_row, settings_divider(), services_row].spacing(0),),
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
