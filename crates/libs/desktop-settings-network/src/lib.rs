//! 网络工具、浏览器与网关设置领域。
pub mod state;
pub mod messages;
pub mod system_settings_browser;
pub mod system_settings_gateway;
pub mod system_settings_gateway_client;
pub mod help;

#[derive(Debug, Clone)]
pub enum Message {
    NetworkTabSelected(NetworkTab),
    None,
    HttpRequest(messages::HttpRequestMessage),
    WebSearch(messages::WebSearchMessage),
    Tunnel(messages::TunnelMessage),
    ProxyEnabledToggled(bool),
    ProxyScopeTextChanged(String),
    ProxyHttpChanged(String),
    ProxyHttpsChanged(String),
    ProxyAllChanged(String),
    ProxyNoProxyChanged(String),
    ProxyServicesChanged(String),
    ProxyHelpOpen,
    ProxyHelpClose,

    Browser(messages::BrowserMessage),
    Gateway(messages::GatewayMessage),
    GatewayClient(messages::GatewayClientMessage),
    BrowserHelp,
    GatewayHealthTick,
    CopyCode(String),
}

mod health;
pub use health::server_health_key;

pub mod message {
    pub mod gateway;
    pub mod gateway_client;
    pub mod http_request;
    pub mod proxy;
    pub mod tunnel;
    pub mod web_search;
    fn parse_comma_or_newline_list(input: &str) -> Vec<String> {
        input.split([',', '\n']).map(str::trim).filter(|value| !value.is_empty()).map(ToString::to_string).collect()
    }
    pub mod browser;
    pub mod gateway_health;
    #[cfg(test)]
    mod gateway_health_tests;
}
pub use message::gateway_health::check_servers;

pub mod app_init;

pub mod system_settings_http_request;
pub mod system_settings_web_search;
pub mod system_settings_proxy;
pub mod system_settings_tunnel;

pub mod system_settings_network;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkTab {
    WebSearch,
    HttpRequest,
    Browser,
    Proxy,
    Tunnel,
}

pub struct NetworkTabDescriptor {
    pub tab: NetworkTab,
    pub label: &'static str,
    pub keywords: &'static [&'static str],
}

impl NetworkTabDescriptor {
    pub const fn new(tab: NetworkTab, label: &'static str, keywords: &'static [&'static str]) -> Self {
        Self { tab, label, keywords }
    }
}

pub mod system_settings_search;

pub mod save_feedback;

#[cfg(not(target_arch = "wasm32"))]
pub mod startup;

pub mod bootstrap;
