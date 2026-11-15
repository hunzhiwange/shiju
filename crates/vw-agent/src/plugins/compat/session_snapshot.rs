//! Compose immutable plugin turns with the real tenant, hook and tool context.
use crate::{config::Config, plugins::{PluginDiscovery, PluginDiscoveryOptions, discover_plugins}};
use anyhow::Result;
pub use agent_plugins::session_snapshot::PluginSessionSnapshot;

pub(crate) struct PluginTurnSnapshot(agent_plugins::session_snapshot::PluginTurnSnapshot);
impl std::ops::Deref for PluginTurnSnapshot {
    type Target = agent_plugins::session_snapshot::PluginTurnSnapshot;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl std::ops::DerefMut for PluginTurnSnapshot {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}
impl PluginTurnSnapshot {
    pub(crate) fn capture(config: &Config) -> Result<Self> {
        agent_plugins::session_snapshot::PluginTurnSnapshot::capture(
            super::super::mcp::plugin_mcp_discovery_options(config),
            || crate::hooks::execution_hook_runner(config),
            || !config.hooks.required.is_empty()
                || std::env::var_os("VW_REQUIRED_EXECUTION_HOOKS").is_some(),
            crate::session::ui_store::current_tenant_scope,
            |hooks| crate::session::prompt::block_on(hooks.shutdown()),
        ).map(Self)
    }
}
pub(crate) fn selection_for_runtime(
    options: &PluginDiscoveryOptions,
) -> Result<(PluginDiscovery, PluginDiscoveryOptions)> {
    if let Some(context) = crate::tools::current_tool_use_context()
        && let Some(snapshot) = context.plugin_snapshot()
    {
        return snapshot.selection(options);
    }
    Ok((discover_plugins(options)?, options.clone()))
}
pub(crate) fn discover_for_runtime(options: &PluginDiscoveryOptions) -> Result<PluginDiscovery> {
    selection_for_runtime(options).map(|(discovery, _)| discovery)
}
