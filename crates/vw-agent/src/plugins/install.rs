//! 组合安全安装服务、现有 npm 获取与租户连接撤销动作。
use anyhow::Result;
use std::path::Path;
pub use agent_plugins::install::publish_materialized_install;
pub use plugin_types::PluginInstallResult;
pub(crate) use agent_plugins::cli::install_dir_from_config;

pub use agent_plugins::install::{install_plugin_from_source, install_plugin_from_marketplace, install_plugin_marketplace_entry};
pub fn set_plugin_enabled(plugins_dir: &Path, name: &str, enabled: bool) -> Result<()> {
    agent_plugins::install::set_plugin_enabled(plugins_dir, name, enabled, &super::compat::mcp_connection_sessions::close_plugin)
}
pub fn remove_installed_plugin(plugins_dir: &Path, name: &str) -> Result<()> {
    agent_plugins::install::remove_installed_plugin(plugins_dir, name, &super::compat::mcp_connection_sessions::close_plugin)
}
