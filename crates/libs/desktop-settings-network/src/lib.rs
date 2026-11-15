//! 浏览器与网关设置领域。
pub mod state;
pub mod messages;
pub mod system_settings_browser;
pub mod system_settings_gateway;
pub mod system_settings_gateway_client;
pub mod help;

#[derive(Debug, Clone)]
pub enum Message {
    None,
    Browser(messages::BrowserMessage),
    Gateway(messages::GatewayMessage),
    GatewayClient(messages::GatewayClientMessage),
    BrowserHelp,
    GatewayHealthTick,
    CopyCode(String),
}

mod health;
pub use health::server_health_key;
