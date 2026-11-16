//! 网络工具配置页，统一承载网页搜索、HTTP 请求、浏览器、代理与隧道配置。

use crate::NetworkTab as SystemTab;
use desktop_ui_controls::system_settings_common::*;
use crate::help::{settings_hover_help_label, settings_page_heading_with_help, settings_group_heading_with_help, settings_flexible_row_with_help, with_settings_help_modal, with_settings_help_modal_mapped, HelpRenderer};
use crate::NetworkTabDescriptor as SettingsTabDescriptor;
use desktop_i18n::I18n;
use desktop_i18n::settings_models::connection_text::text as connection_text;
use crate::Message;
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, overlay, renderer};
use iced::widget::{Column, column};
use iced::{Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};

pub const NETWORK_TABS: &[SettingsTabDescriptor] = &[
    SettingsTabDescriptor::new(
        SystemTab::WebSearch,
        "网页搜索",
        &[
            "web search",
            "網頁搜尋",
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
            "http requests",
            "網路請求",
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
            "瀏覽器",
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
            "代理",
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
            "隧道",
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
pub fn view<'a>(app: View<'a>) -> Element<'a, Message> {
    let active_tab = app.system_settings_tab;
    let tabs = settings_tab_row(NETWORK_TABS.iter().map(|descriptor| {
        settings_tab_button(
            connection_text(app.desktop_locale, descriptor.label),
            descriptor.tab == active_tab,
            Some(Message::NetworkTabSelected(descriptor.tab)),
        )
        .into()
    }))
    .wrap();

    let content = match active_tab {
        SystemTab::HttpRequest => {
            crate::system_settings_http_request::content(crate::system_settings_http_request::View { http_request_settings: app.http_request_settings, desktop_locale: app.desktop_locale, app_theme: app.app_theme, render_help: app.render_help })
        }
        SystemTab::Browser => crate::system_settings_browser::content(crate::system_settings_browser::View { browser_settings: app.browser_settings, desktop_locale: app.desktop_locale, app_theme: app.app_theme, render_help: app.render_help }),
        SystemTab::Proxy => crate::system_settings_proxy::content(crate::system_settings_proxy::View { proxy_settings: app.proxy_settings, desktop_locale: app.desktop_locale, app_theme: app.app_theme, render_help: app.render_help, last_copied_code_hash: app.last_copied_code_hash }),
        SystemTab::Tunnel => crate::system_settings_tunnel::content(crate::system_settings_tunnel::View { tunnel_settings: app.tunnel_settings, desktop_locale: app.desktop_locale, app_theme: app.app_theme, render_help: app.render_help }),
        _ => crate::system_settings_web_search::content(crate::system_settings_web_search::View { web_search_settings: app.web_search_settings, desktop_locale: app.desktop_locale, app_theme: app.app_theme, render_help: app.render_help }),
    };

    let mut sections = column![
        settings_page_heading_with_help(
        app.render_help,
            I18n::new(app.desktop_locale),
            app.app_theme,
            connection_text(app.desktop_locale, "网络配置"),
            connection_text(
                app.desktop_locale,
                "通过标签切换网页搜索、网络请求、浏览器、代理与隧道配置。"
            ),
        ),
        tabs,
    ]
    .spacing(16)
    .width(Length::Fill);

    if let Some(error) = active_save_error(&app) {
        sections = sections.push(settings_error_banner(
            if app.system_settings_tab == SystemTab::Browser {
                app.browser_settings
                    .save_error_translation
                    .as_ref()
                    .map(|message| {
                        message
                            .render(desktop_i18n::I18n::new(app.desktop_locale))
                            .expect("valid browser error template")
                    })
                    .unwrap_or_else(|| error.to_string())
            } else {
                desktop_i18n::I18n::new(app.desktop_locale).__(error).to_string()
            },
        ));
    }

    Element::new(NetworkTabView { tab: active_tab, content: sections.push(content) })
}

// Settings search can switch tabs without a click to dismiss the current help card.
struct NetworkTabView<'a> {
    tab: SystemTab,
    content: Column<'a, Message>,
}

impl Widget<Message, Theme, Renderer> for NetworkTabView<'_> {
    fn tag(&self) -> tree::Tag {
        struct NetworkTabTag;
        tree::Tag::of::<NetworkTabTag>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(self.tab)
    }

    fn children(&self) -> Vec<Tree> {
        self.content.children()
    }

    fn diff(&self, tree: &mut Tree) {
        let tab = tree.state.downcast_mut::<SystemTab>();
        if *tab == self.tab {
            self.content.diff(tree);
        } else {
            *tab = self.tab;
            tree.children = self.content.children();
        }
    }

    fn size(&self) -> Size<Length> {
        self.content.size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.layout(tree, renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content.operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, Theme, Renderer>> {
        self.content.overlay(tree, layout, renderer, viewport, translation)
    }
}

fn active_save_error<'a>(app: &View<'a>) -> Option<&'a str> {
    match app.system_settings_tab {
        SystemTab::WebSearch => app.web_search_settings.save_error.as_deref(),
        SystemTab::HttpRequest => app.http_request_settings.save_error.as_deref(),
        SystemTab::Browser => app.browser_settings.save_error.as_deref(),
        SystemTab::Proxy => app.proxy_settings.save_error.as_deref(),
        SystemTab::Tunnel => app.tunnel_settings.save_error.as_deref(),

    }
}


#[derive(Clone, Copy)]
pub struct View<'a> {
    pub system_settings_tab: crate::NetworkTab,
    pub web_search_settings: &'a crate::app_init::state::WebSearchSettingsState,
    pub http_request_settings: &'a crate::app_init::state::HttpRequestSettingsState,
    pub browser_settings: &'a crate::state::BrowserSettingsState,
    pub proxy_settings: &'a crate::app_init::state::ProxySettingsState,
    pub tunnel_settings: &'a crate::app_init::state::TunnelSettingsState,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: &'a Theme,
    pub render_help: crate::help::HelpRenderer,
    pub last_copied_code_hash: Option<u64>,
}

#[cfg(test)]
#[path = "system_settings_network_tests.rs"]
mod tests;
