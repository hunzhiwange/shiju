//! Audited plugin paths and hook declaration loading.
pub mod audit;
pub mod hooks;
pub use plugin_types::{PluginHookSource, PluginPath};
pub use hooks::{PluginHook, PluginHookEvent, PluginHookMode, PluginHooksManifest, load_plugin_hook_source, load_plugin_hooks_manifest};

#[cfg(test)]
mod hooks_tests;

pub mod hook_trust;
pub mod manifest;
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
