//! 插件包的发现、校验、安装与运行时集成入口。
//!
//! 子模块分别维护路径审计、钩子信任、清单解析、MCP 与技能加载边界；
//! 本模块只重导出经过这些边界约束的稳定能力。

mod audit;
mod cli;
mod hook_handler;
mod hook_trust;
mod hooks;
mod install;
mod loader;
mod manifest;
mod mcp;
mod skills;
mod types;

mod compat;

/// 插件管理命令与命令行执行入口。
pub use crate::cli_commands::PluginCommands;
pub use cli::handle_command;
pub(crate) use hook_handler::trusted_plugin_hook_handlers;
pub use hook_trust::{
    PluginHookTrustStatus, PluginHookTrustStore, plugin_hook_trust_file, plugin_hook_trust_hash,
};
pub use hooks::{PluginHook, PluginHookEvent, PluginHookMode, load_plugin_hooks_manifest};
pub use install::{
    PluginInstallResult, install_plugin_from_marketplace, install_plugin_from_source,
};
pub(crate) use install::{install_dir_from_config, remove_installed_plugin, set_plugin_enabled};
pub use loader::{
    DiscoveredPlugin, PluginDiscovery, PluginDiscoveryDiagnostic, PluginDiscoveryOptions,
    discover_plugins,
};
pub use manifest::load_plugin_manifest;
pub(crate) use mcp::{merge_enabled_plugin_mcp_servers, plugin_mcp_discovery_options};
pub(crate) use skills::load_plugin_skills;
/// 已完成路径审计和字段规范化的插件清单类型。
pub use types::{
    PluginAuthor, PluginAuthorDetails, PluginHookSource, PluginIdentity, PluginInstallMetadata,
    PluginInterface, PluginManifest, PluginMcpServers, PluginPath,
};

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod manifest_tests;

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod mcp_tests;

#[cfg(test)]
#[path = "hooks_tests.rs"]
mod hooks_tests;

#[cfg(test)]
#[path = "hook_trust_tests.rs"]
mod hook_trust_tests;

#[cfg(test)]
#[path = "hook_handler_tests.rs"]
mod hook_handler_tests;

#[cfg(test)]
#[path = "install_tests.rs"]
mod install_tests;

#[cfg(test)]
#[path = "cli_tests.rs"]
mod cli_tests;

#[cfg(test)]
#[path = "skills_tests.rs"]
mod skills_tests;
