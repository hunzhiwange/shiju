//! 组合安全安装服务、现有 npm 获取与租户连接撤销动作。
use anyhow::Result;
use std::path::{Path, PathBuf};
pub use agent_plugins::install::publish_materialized_install;
pub use plugin_types::PluginInstallResult;
/// Return the plugin installation directory for a loaded runtime config.
/// 根据配置解析本地插件安装根目录。
pub(crate) fn install_dir_from_config(config: &crate::config::Config) -> PathBuf {
    config
        .config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| config.workspace_dir.clone())
        .join("plugins")
}


pub fn install_plugin_from_source(source: &str, plugins_dir: &Path) -> Result<PluginInstallResult> {
    agent_plugins::install::install_plugin_from_source(source, plugins_dir, &super::compat::npm_archive::install)
}
pub fn install_plugin_from_marketplace(source: &str, plugins_dir: &Path, marketplace: &str) -> Result<PluginInstallResult> {
    agent_plugins::install::install_plugin_from_marketplace(source, plugins_dir, marketplace, &super::compat::npm_archive::install)
}
pub fn install_plugin_marketplace_entry(entry: &super::MarketplaceEntry, plugins_dir: &Path) -> Result<PluginInstallResult> {
    agent_plugins::install::install_plugin_marketplace_entry(entry, plugins_dir, &super::compat::npm_archive::install)
}
pub fn set_plugin_enabled(plugins_dir: &Path, name: &str, enabled: bool) -> Result<()> {
    agent_plugins::install::set_plugin_enabled(plugins_dir, name, enabled, &super::compat::mcp_connection_sessions::close_plugin)
}
pub fn remove_installed_plugin(plugins_dir: &Path, name: &str) -> Result<()> {
    agent_plugins::install::remove_installed_plugin(plugins_dir, name, &super::compat::mcp_connection_sessions::close_plugin)
}
