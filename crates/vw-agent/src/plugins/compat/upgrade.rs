//! 升级事务由agent-plugins持有，宿主注入原租户连接撤销动作。
pub use agent_plugins::upgrade::{PluginUpgradePreview, prepare_plugin_upgrade, prepare_plugin_marketplace_upgrade};
pub fn activate_plugin_upgrade(preview: PluginUpgradePreview) -> anyhow::Result<()> {
    agent_plugins::upgrade::activate_plugin_upgrade(preview, &super::mcp_connection_sessions::close_plugin)
}
