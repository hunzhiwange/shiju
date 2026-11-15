//! Host actions retained until plugin lifecycle orchestration moves to its owner.
use std::{collections::HashMap, path::Path, sync::Mutex};
use agent_config::runtime::Config;
use agent_plugins::{PluginInstallResult, PluginManifest};
use anyhow::Result;
use axum::http::HeaderMap;
use gateway_types::ApiError;
use plugin_install::marketplace::MarketplaceEntry;
use gateway_desktop_settings::DesktopPluginDependencyDto;
use crate::operations::Pending;

pub trait PluginHost: Send + Sync + 'static {
    type State: Clone + Send + Sync + 'static;
    type Upgrade: Send + 'static;
    fn state_config(state: &Self::State) -> Config;
    fn authorize(state: &Self::State, headers: &HeaderMap) -> Result<(), ApiError>;
    fn pending() -> &'static Mutex<HashMap<String, Pending<Self::Upgrade>>>;
    fn set_plugin_enabled(root: &Path, id: &str, enabled: bool) -> Result<()>;
    fn remove_installed_plugin(root: &Path, id: &str) -> Result<()>;
    fn inspect_plugin_dependencies(root: &Path) -> Result<Vec<DesktopPluginDependencyDto>>;
    fn install_plugin_marketplace_entry(entry: &MarketplaceEntry, root: &Path) -> Result<PluginInstallResult>;
    fn install_plugin_from_source(source: &str, root: &Path) -> Result<PluginInstallResult>;
    fn prepare_plugin_upgrade(source: &str, root: &Path, id: &str) -> Result<Self::Upgrade>;
    fn prepare_plugin_marketplace_upgrade(entry: &MarketplaceEntry, root: &Path) -> Result<Self::Upgrade>;
    fn activate_plugin_upgrade(preview: Self::Upgrade) -> Result<()>;
    fn manifest(preview: &Self::Upgrade) -> &PluginManifest;
    fn old_manifest(preview: &Self::Upgrade) -> &PluginManifest;
    fn identity_key(preview: &Self::Upgrade) -> String;
}
