//! 通道运行实现与宿主能力注入。
use super::*;
pub(crate) use channel_runtime::manager::execution_binding::*;
pub(crate) async fn resolve_execution_route(
    snapshot: &ChannelMessageSnapshot,
    fallback: ChannelRouteSelection,
) -> Result<(ChannelRouteSelection, bool)> { channel_runtime::manager::execution_binding::resolve_execution_route(&super::runtime_host::HOST, snapshot, fallback).await }
pub(crate) async fn execution_options(
    snapshot: &ChannelMessageSnapshot,
    session: &str,
    route: &ChannelRouteSelection,
    options: serde_json::Value,
) -> Result<serde_json::Value> { channel_runtime::manager::execution_binding::execution_options(&super::runtime_host::HOST, snapshot, session, route, options).await }
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/execution_binding_tests.rs"]
mod tests;

#[cfg(test)]
use config_agent_profile::{AgentDefault, AgentRuntime};
#[cfg(test)]
use config_agent::AgentDefinitionConfig;
#[cfg(test)]
use config_channels::ChannelExecutionConfig;
