//! Network page regressions using only domain-owned state and renderers.
#![cfg(not(target_arch = "wasm32"))]
use crate::Message;
use crate::Message as SettingsMessage;
use crate::messages::*;
use crate::state::*;
use desktop_ui_controls::settings_navigation::SystemTab;
use iced::Element;

struct NetworkFixture {
    browser_settings: BrowserSettingsState,
    gateway_settings: GatewaySettingsState,
    gateway_client_settings: GatewayClientSettingsState,
    http_request_settings: crate::app_init::state::HttpRequestSettingsState,
    web_search_settings: crate::app_init::state::WebSearchSettingsState,
    proxy_settings: crate::app_init::state::ProxySettingsState,
    tunnel_settings: crate::app_init::state::TunnelSettingsState,
    desktop_locale: config_ui::DesktopLocale,
    app_theme: iced::Theme,
    system_settings_tab: SystemTab,
    show_system_settings: bool,
    last_copied_code_hash: Option<u64>,
}
use NetworkFixture as App;
fn help<'a>(
    title: &'a str,
    body: std::borrow::Cow<'a, str>,
    color: Option<iced::Color>,
    i18n: desktop_i18n::I18n,
) -> Element<'a, Message> {
    desktop_chat_view::message_view::markdown_selection::selectable_titled_text(
        title, body, color, i18n,
    )
}
impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        use desktop_ui_controls::text_editor_context_menu::TextContextEffect;
        match effect {
            TextContextEffect::Copy(text) => Self::CopyCode(text),
            TextContextEffect::OpenUrl(url) => Self::OpenUrl(url),
            TextContextEffect::Append(_) => Self::None,
        }
    }
    fn selection(_: String, _: usize, _: usize, _: Option<String>) -> Self {
        Self::None
    }
}
macro_rules! page {
    ($module:ident, $domain:ident, $field:ident, $($extra:ident),*) => {
        mod $module {
            use super::*;
            pub fn inputs(app: &NetworkFixture) -> crate::$domain::View<'_> {
                crate::$domain::View { $field: &app.$field, desktop_locale: app.desktop_locale, app_theme: &app.app_theme, render_help: help, $($extra: app.$extra,)* }
            }
            pub fn view(app: &NetworkFixture) -> Element<'_, Message> { crate::$domain::view(inputs(app)) }
        }
    }
}
page!(browser, system_settings_browser, browser_settings,);
page!(http_request, system_settings_http_request, http_request_settings,);
page!(web_search, system_settings_web_search, web_search_settings,);
page!(tunnel, system_settings_tunnel, tunnel_settings,);
mod gateway_client {
    use super::*;
    fn inputs(app: &NetworkFixture) -> crate::system_settings_gateway_client::View<'_> {
        crate::system_settings_gateway_client::View {
            gateway_client_settings: &app.gateway_client_settings,
            desktop_locale: app.desktop_locale,
            app_theme: &app.app_theme,
            render_help: help,
            last_copied_code_hash: app.last_copied_code_hash,
        }
    }
    pub fn view(app: &NetworkFixture) -> Element<'_, Message> {
        crate::system_settings_gateway_client::view(inputs(app))
    }
    pub fn view_overlays<'a>(
        app: &'a NetworkFixture,
        base: Element<'a, Message>,
    ) -> Element<'a, Message> {
        match crate::system_settings_gateway_client::overlay(inputs(app)) {
            Some(overlay) => iced::widget::stack![base, overlay].into(),
            None => base,
        }
    }
}
mod proxy {
    use super::*;
    fn inputs(app: &NetworkFixture) -> crate::system_settings_proxy::View<'_> {
        crate::system_settings_proxy::View {
            proxy_settings: &app.proxy_settings,
            desktop_locale: app.desktop_locale,
            app_theme: &app.app_theme,
            render_help: help,
            last_copied_code_hash: app.last_copied_code_hash,
        }
    }
    pub fn view(app: &NetworkFixture) -> Element<'_, Message> {
        crate::system_settings_proxy::view(inputs(app))
    }
    pub fn view_overlays<'a>(
        app: &'a NetworkFixture,
        base: Element<'a, Message>,
    ) -> Element<'a, Message> {
        crate::system_settings_proxy::view_overlays(inputs(app), base, |m| m)
    }
}
mod gateway {
    use super::*;
    pub use crate::system_settings_gateway::{service_command_copy_button, service_display_label};
    pub use desktop_i18n::settings_models::connection_text::text as connection_text;
    fn inputs(app: &NetworkFixture) -> crate::system_settings_gateway::View<'_> {
        crate::system_settings_gateway::View {
            gateway_settings: &app.gateway_settings,
            desktop_locale: app.desktop_locale,
            app_theme: &app.app_theme,
            render_help: help,
            last_copied_code_hash: app.last_copied_code_hash,
        }
    }
    pub fn view(app: &NetworkFixture) -> Element<'_, Message> {
        crate::system_settings_gateway::view(inputs(app))
    }
    pub fn skey_calendar_popover(app: &NetworkFixture) -> Element<'_, Message> {
        crate::system_settings_gateway::skey_calendar_popover(inputs(app))
    }
    pub fn skey_date_picker(app: &NetworkFixture) -> Element<'_, Message> {
        crate::system_settings_gateway::skey_date_picker(inputs(app))
    }
    pub fn view_overlays<'a>(
        app: &'a NetworkFixture,
        base: Element<'a, Message>,
    ) -> Element<'a, Message> {
        match crate::system_settings_gateway::overlay(inputs(app)) {
            Some(overlay) => iced::widget::stack![base, overlay].into(),
            None => base,
        }
    }
}
mod network {
    use super::*;
    pub fn view(app: &NetworkFixture) -> Element<'_, Message> {
        let tab = match app.system_settings_tab {
            SystemTab::Browser => crate::NetworkTab::Browser,
            SystemTab::Proxy => crate::NetworkTab::Proxy,
            SystemTab::Tunnel => crate::NetworkTab::Tunnel,
            SystemTab::HttpRequest => crate::NetworkTab::HttpRequest,
            _ => crate::NetworkTab::WebSearch,
        };
        crate::system_settings_network::view(crate::system_settings_network::View {
            system_settings_tab: tab,
            browser_settings: &app.browser_settings,
            http_request_settings: &app.http_request_settings,
            web_search_settings: &app.web_search_settings,
            proxy_settings: &app.proxy_settings,
            tunnel_settings: &app.tunnel_settings,
            desktop_locale: app.desktop_locale,
            app_theme: &app.app_theme,
            render_help: help,
            last_copied_code_hash: app.last_copied_code_hash,
        })
    }
}
fn settings_content(app: &NetworkFixture, tab: SystemTab) -> Element<'_, Message> {
    match tab {
        SystemTab::Gateway => gateway::view(app),
        SystemTab::GatewayClient => gateway_client::view(app),
        _ => network::view(app),
    }
}
mod system_settings {
    pub use super::SystemTab;
    pub fn view(app: &super::NetworkFixture) -> iced::Element<'_, super::Message> {
        super::settings_content(app, app.system_settings_tab)
    }
}
fn with_help_modal<'a>(
    app: &NetworkFixture,
    base: Element<'a, Message>,
    tab: SystemTab,
    open: Option<SystemTab>,
) -> Element<'a, Message> {
    desktop_settings_appearance::help::with_help_modal(
        app.desktop_locale,
        app.last_copied_code_hash,
        Message::CopyCode,
        Message::None,
        base,
        tab,
        open,
    )
}
fn update(app: &mut NetworkFixture, message: Message) {
    match message {
        Message::Browser(m) => {
            crate::message::browser::update(&mut app.browser_settings, m);
        }
        Message::HttpRequest(_) => {
            crate::message::http_request::update(&mut app.http_request_settings, message);
        }
        Message::WebSearch(_) => {
            crate::message::web_search::update(&mut app.web_search_settings, message);
        }
        Message::Gateway(_) => {
            crate::message::gateway::update(&mut app.gateway_settings, message);
        }
        Message::NetworkTabSelected(tab) => {
            app.system_settings_tab = match tab {
                crate::NetworkTab::Browser => SystemTab::Browser,
                crate::NetworkTab::Proxy => SystemTab::Proxy,
                crate::NetworkTab::Tunnel => SystemTab::Tunnel,
                crate::NetworkTab::HttpRequest => SystemTab::HttpRequest,
                crate::NetworkTab::WebSearch => SystemTab::WebSearch,
            };
        }
        _ => {}
    }
}
mod browser_ui_refresh_26;
mod gateway_client_ui_refresh_24;
mod gateway_ui_refresh_23;
mod http_request_ui_refresh_25;
mod proxy_tests;

mod gateway_client_update;

impl Default for NetworkFixture {
    fn default() -> Self {
        Self {
            browser_settings: Default::default(),
            gateway_settings: Default::default(),
            gateway_client_settings: Default::default(),
            http_request_settings: Default::default(),
            web_search_settings: Default::default(),
            proxy_settings: Default::default(),
            tunnel_settings: Default::default(),
            desktop_locale: Default::default(),
            app_theme: iced::Theme::Dark,
            system_settings_tab: SystemTab::Browser,
            show_system_settings: Default::default(),
            last_copied_code_hash: Default::default(),
        }
    }
}

mod tunnel_tests;
