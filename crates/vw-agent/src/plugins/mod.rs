//! Plugin package support.

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

pub use crate::cli_commands::PluginCommands;
pub use cli::handle_command;
pub(crate) use hook_handler::register_trusted_plugin_hooks;
pub use hook_trust::{
    PluginHookTrustStatus, PluginHookTrustStore, plugin_hook_trust_file, plugin_hook_trust_hash,
};
pub use hooks::{PluginHook, PluginHookEvent, PluginHookMode, load_plugin_hooks_manifest};
pub(crate) use install::{install_dir_from_config, remove_installed_plugin, set_plugin_enabled};
pub use loader::{DiscoveredPlugin, PluginDiscovery, PluginDiscoveryOptions, discover_plugins};
pub use manifest::load_plugin_manifest;
pub(crate) use mcp::{merge_enabled_plugin_mcp_servers, plugin_mcp_discovery_options};
pub(crate) use skills::load_plugin_skills;
pub use types::{PluginAuthor, PluginAuthorDetails, PluginInterface, PluginManifest, PluginPath};

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
