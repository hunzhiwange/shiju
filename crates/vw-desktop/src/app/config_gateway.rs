//! 组装桌面端网关配置、主代理覆盖项与客户端连接任务。

use crate::app::{Message, message::SettingsMessage};
use iced::Task;
use {gateway_endpoint::{GatewayEndpoint}, {vw_gateway_client::GatewayClient}};

use super::system_settings::load_gateway_client_bootstrap_config;

pub(in crate::app) use desktop_config::access::apply_main_agent_overrides;

#[cfg(not(target_arch = "wasm32"))]
pub(super) use desktop_config::config_gateway::project_config_override_path;

#[cfg(all(test, not(target_arch = "wasm32")))]
use desktop_config::config_gateway::load_vibewindow_root_json;

pub(super) use desktop_config::access::set_config_value_at_path;

#[cfg(not(target_arch = "wasm32"))]
pub(super) use desktop_config::config_gateway::load_config_value_at_path;

pub use desktop_config::config_gateway::server_config_unreachable_error;

#[cfg(target_arch = "wasm32")]
pub fn spawn_gateway_task(
    tag: &'static str,
    future: impl std::future::Future<Output = Result<(), String>> + 'static,
) -> Task<Message> {
    Task::perform(async move { future.await }, move |result| {
        Message::Settings(SettingsMessage::AgentConfigSaved { tag, result })
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_gateway_task(
    tag: &'static str,
    future: impl std::future::Future<Output = Result<(), String>> + Send + 'static,
) -> Task<Message> {
    Task::perform(future, move |result| {
        Message::Settings(SettingsMessage::AgentConfigSaved { tag, result })
    })
}

/// 从当前活动服务配置构造带认证与租户范围的网关端点。
pub fn gateway_client_endpoint() -> GatewayEndpoint {
    gateway_client_endpoint_from_config(&load_gateway_client_bootstrap_config())
}

pub(super) use desktop_config::config_gateway::gateway_client_endpoint_from_config;

/// 创建当前桌面配置对应的网关客户端。
pub fn gateway_client() -> Result<GatewayClient, String> {
    GatewayClient::new(gateway_client_endpoint())
}

/// 同步加载静态工具标识；连接失败时返回空列表并过滤动态 MCP 工具。
pub fn load_tools_list_via_gateway() -> Vec<String> {
    config_access().load_tools_list_via_gateway()
}

pub(super) use desktop_config::access::run_gateway_call;

#[cfg(test)]
#[path = "config_gateway_tests.rs"]
mod config_gateway_tests;

fn config_access() -> desktop_config::ConfigAccess {
    desktop_config::ConfigAccess::new(gateway_client_endpoint(), None)
}
