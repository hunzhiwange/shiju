use std::{collections::HashMap, path::Path, sync::{Mutex,OnceLock}};
use agent_config::runtime::Config;
use agent_plugins::{PluginInstallResult,PluginManifest,MarketplaceEntry};
use gateway_desktop_settings::DesktopPluginDependencyDto;
use axum::http::HeaderMap;
use gateway_types::ApiError;
use anyhow::Result;
use crate::{PluginHost,Pending};
pub(super) struct Host;
impl PluginHost for Host {
    type State = Config;
    type Upgrade = agent_plugins::upgrade::PluginUpgradePreview;
    fn state_config(state: &Config) -> Config { state.clone() }
    fn authorize(_state: &Config, headers: &HeaderMap) -> Result<(), ApiError> {
        // Explicit external authorization port; production authentication stays in the host test.
        if headers.get("authorization").and_then(|value| value.to_str().ok()) == Some("Bearer fixture-token") { Ok(()) }
        else { Err(ApiError { status: axum::http::StatusCode::UNAUTHORIZED, message: "Unauthorized".into() }) }
    }
    fn pending() -> &'static Mutex<HashMap<String, Pending<agent_plugins::upgrade::PluginUpgradePreview>>> {
        static PENDING: OnceLock<Mutex<HashMap<String, Pending<agent_plugins::upgrade::PluginUpgradePreview>>>> = OnceLock::new();
        PENDING.get_or_init(|| Mutex::new(HashMap::new()))
    }
    fn set_plugin_enabled(root: &Path, id: &str, enabled: bool) -> Result<()> {
        agent_plugins::install::set_plugin_enabled(root, id, enabled, &|_| {})
    }
    fn remove_installed_plugin(root: &Path, id: &str) -> Result<()> {
        agent_plugins::install::remove_installed_plugin(root, id, &|_| {})
    }
    fn inspect_plugin_dependencies(root: &Path) -> Result<Vec<DesktopPluginDependencyDto>> {
        let manifest = agent_plugins::load_plugin_manifest(root)?;
        let (outcome, _) = agent_plugins::mcp::load_plugin_mcp_declarations(&manifest)?;
        let checks = plugin_install::dependencies::inspect_plugin_dependencies(
            &manifest, outcome,
            |manifest| plugin_install::skill_roots::skill_roots(manifest, &agent_plugins::audit::resolve_component_path),
            |root| plugin_install::skill_roots::skill_files(root, plugin_install::skill_roots::package_format(&manifest)),
            |root, skill| plugin_install::agents::load_metadata(root, skill, agent_plugins::audit::resolve_component_path),
            plugin_mcp::mcp_portable::env_names_match,
        )?;
        Ok(checks.into_iter().map(|check| DesktopPluginDependencyDto { status: check.state.as_str().into(), component: check.component, dependency: check.dependency, detail: check.detail }).collect())
    }
    fn install_plugin_marketplace_entry(entry: &MarketplaceEntry, root: &Path) -> Result<PluginInstallResult> {
        agent_plugins::install_plugin_marketplace_entry(entry, root)
    }
    fn install_plugin_from_source(source: &str, root: &Path) -> Result<PluginInstallResult> {
        agent_plugins::install_plugin_from_source(source, root)
    }
    fn prepare_plugin_upgrade(source: &str, root: &Path, id: &str) -> Result<agent_plugins::upgrade::PluginUpgradePreview> {
        agent_plugins::prepare_plugin_upgrade(source, root, id)
    }
    fn prepare_plugin_marketplace_upgrade(entry: &MarketplaceEntry, root: &Path) -> Result<agent_plugins::upgrade::PluginUpgradePreview> {
        agent_plugins::prepare_plugin_marketplace_upgrade(entry, root)
    }
    fn activate_plugin_upgrade(preview: agent_plugins::upgrade::PluginUpgradePreview) -> Result<()> {
        agent_plugins::upgrade::activate_plugin_upgrade(preview, &|_| {})
    }
    fn manifest(preview: &agent_plugins::upgrade::PluginUpgradePreview) -> &PluginManifest { preview.manifest() }
    fn old_manifest(preview: &agent_plugins::upgrade::PluginUpgradePreview) -> &PluginManifest { preview.old_manifest() }
    fn identity_key(preview: &agent_plugins::upgrade::PluginUpgradePreview) -> String { preview.identity_key() }
}
