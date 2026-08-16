//! 网络工具配置页，统一承载网页搜索、HTTP 请求、浏览器、代理与隧道配置。

use crate::app::components::system_settings::SystemTab;
use crate::app::components::system_settings_common::{
    settings_error_banner, settings_page_intro, settings_segment_button_style,
};
use crate::app::components::system_settings_search::SettingsTabDescriptor;
use crate::app::{App, Message, message};
use iced::widget::{button, column, row, text};
use iced::{Alignment, Element, Length, Theme};

pub(super) const NETWORK_TABS: &[SettingsTabDescriptor<SystemTab>] = &[
    SettingsTabDescriptor::new(
        SystemTab::WebSearch,
        "网页搜索",
        &[
            "web search",
            "DuckDuckGo",
            "Brave",
            "Brave 密钥",
            "Serper",
            "Google",
            "Bing",
            "API 密钥",
            "结果数量",
        ],
    ),
    SettingsTabDescriptor::new(
        SystemTab::HttpRequest,
        "网络请求",
        &[
            "http request",
            "allowlist",
            "允许域名",
            "白名单",
            "max body",
            "timeout",
            "User-Agent",
            "method",
            "headers",
        ],
    ),
    SettingsTabDescriptor::new(
        SystemTab::Browser,
        "浏览器",
        &[
            "browser",
            "computer use",
            "窗口行为",
            "运行后端",
            "webdriver",
            "sidecar",
            "viewport",
            "自动化",
        ],
    ),
    SettingsTabDescriptor::new(
        SystemTab::Proxy,
        "代理",
        &[
            "proxy",
            "HTTP 代理",
            "HTTPS 代理",
            "SOCKS5",
            "ALL_PROXY",
            "NO_PROXY",
            "生效范围",
            "服务选择器",
        ],
    ),
    SettingsTabDescriptor::new(
        SystemTab::Tunnel,
        "隧道",
        &[
            "tunnel",
            "public gateway",
            "Cloudflare",
            "Tailscale",
            "funnel",
            "tailnet",
            "ngrok",
            "公网",
            "自定义隧道",
        ],
    ),
];

/// 渲染网络配置及其内部标签。
pub fn view(app: &App) -> Element<'_, Message> {
    let active_tab = app.system_settings_tab;
    let tab_button = |tab, label| {
        let is_active = tab == active_tab;
        button(text(label).size(13))
            .on_press(Message::Settings(message::SettingsMessage::SystemTabSelected(tab)))
            .padding([8, 14])
            .style(move |theme: &Theme, status| {
                settings_segment_button_style(theme, status, is_active)
            })
    };

    let tabs = NETWORK_TABS
        .iter()
        .fold(row![].spacing(8).align_y(Alignment::Center), |row, descriptor| {
            row.push(tab_button(descriptor.tab, descriptor.label))
        });

    let content = match active_tab {
        SystemTab::HttpRequest => {
            crate::app::components::system_settings_http_request::content(app)
        }
        SystemTab::Browser => crate::app::components::system_settings_browser::content(app),
        SystemTab::Proxy => crate::app::components::system_settings_proxy::content(app),
        SystemTab::Tunnel => crate::app::components::system_settings_tunnel::content(app),
        _ => crate::app::components::system_settings_web_search::content(app),
    };

    let mut sections = column![
        settings_page_intro(
            "网络配置",
            "通过标签切换网页搜索、网络请求、浏览器、代理与隧道配置。",
        ),
        tabs,
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(error) = active_save_error(app) {
        sections = sections.push(settings_error_banner(error));
    }

    sections.push(content).into()
}

fn active_save_error(app: &App) -> Option<&str> {
    match app.system_settings_tab {
        SystemTab::WebSearch => app.web_search_settings.save_error.as_deref(),
        SystemTab::HttpRequest => app.http_request_settings.save_error.as_deref(),
        SystemTab::Browser => app.browser_settings.save_error.as_deref(),
        SystemTab::Proxy => app.proxy_settings.save_error.as_deref(),
        SystemTab::Tunnel => app.tunnel_settings.save_error.as_deref(),
        _ => None,
    }
}

#[cfg(test)]
#[path = "system_settings_network_tests.rs"]
mod system_settings_network_tests;
