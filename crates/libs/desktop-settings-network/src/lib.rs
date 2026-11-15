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

pub mod message {
    pub mod gateway_health;
    #[cfg(test)]
    mod gateway_health_tests;
}
pub use message::gateway_health::check_servers;

pub mod app_init;
