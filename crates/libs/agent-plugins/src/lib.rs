//! Audited plugin paths and hook declaration loading.
pub mod audit;
pub mod hooks;
pub use plugin_types::{PluginHookSource, PluginPath};
pub use hooks::{PluginHook, PluginHookEvent, PluginHookMode, PluginHooksManifest, load_plugin_hook_source, load_plugin_hooks_manifest};

#[cfg(test)]
mod hooks_tests;

pub mod hook_trust;
pub mod manifest;
#[cfg(test)]
mod compat_t46_tests;
#[cfg(test)]
mod compat_t05_tests;
#[cfg(test)]
mod compat_t04_tests;
#[cfg(test)]
mod compat_t02_tests;
#[cfg(test)]
mod compat_t03_tests;
#[cfg(test)]
mod compat_t07_tests;
pub mod loader;
pub mod legacy_manifest;
pub mod portable_manifest;
pub use plugin_types::{PluginIdentity, PluginInstallMetadata, PluginManifest, PluginMcpServers};
pub use manifest::load_plugin_manifest;
pub use loader::{DiscoveredPlugin, PluginDiscovery, PluginDiscoveryDiagnostic, PluginDiscoveryOptions, discover_plugins};
pub use hook_trust::{PluginHookTrustStatus, PluginHookTrustStore, plugin_hook_trust_file, plugin_hook_trust_hash};

fn is_plugin_candidate(root: &std::path::Path) -> anyhow::Result<bool> {
    plugin_install::discovery::is_plugin_candidate(root, portable_manifest::SCHEMA_PREFIX, audit::canonical_plugin_root)
}

#[cfg(test)]
mod hook_trust_tests;
#[cfg(test)]
mod manifest_tests;

pub use plugin_types::PluginInstallResult;

pub mod install;
pub mod transaction_install;
pub use plugin_install::marketplace::{MarketplaceEntry, InstallationPolicy, MarketplaceSource};

#[cfg(test)]
mod install_tests;
#[cfg(test)]
mod transaction_install_tests;

pub mod npm_archive;

pub mod npm_source;
#[cfg(test)]
mod npm_source_tests;

pub mod upgrade;
pub use install::{install_plugin_from_source, install_plugin_from_marketplace, install_plugin_marketplace_entry};
pub use upgrade::{prepare_plugin_upgrade, prepare_plugin_marketplace_upgrade};
#[cfg(test)]
mod upgrade_tests;

pub mod cli;
pub mod onboarding;
pub mod session_snapshot;
pub use agent_cli_types::PluginCommands;
pub use plugin_types::CodexHooksManifest;
pub use plugin_install::marketplace::{add_plugin_marketplace, list_plugin_marketplaces, registered_plugin_entry, read_registered_marketplace};

pub mod hook_handler;

pub mod mcp;
#[cfg(test)]
mod mcp_tests;

pub mod skills;

#[cfg(test)]
mod compat_t12_tests;

#[cfg(test)]
mod compat_t06_tests;

#[cfg(test)]
mod compat_t09_tests;

#[cfg(test)]
mod compat_t11_tests;

#[cfg(test)]
mod compat_t14_tests;

#[cfg(test)]
mod compat_t16_tests;

#[cfg(test)]
mod compat_t17_auth_tests;

#[cfg(test)]
mod compat_t32_tests;

#[cfg(test)]
mod compat_t51_tests;

#[cfg(test)]
mod compat_t53_tests;

#[cfg(test)]
mod compat_t61_tests;

#[cfg(test)]
mod compat_t64_tests;

#[cfg(test)]
mod host_migration_39_tests;
