//! Host composition for the plugin HTTP routes.
use anyhow::Result;
use std::{collections::HashMap, path::Path, sync::{Mutex, OnceLock}};
use axum::{Router, http::HeaderMap};
#[cfg(test)]
use axum::{Json, extract::Query};
#[cfg(test)]
use std::path::PathBuf;
use gateway_plugins::{PluginHost, Pending};
use gateway_types::ApiError;
use crate::{config::Config, gateway::AppState, plugins};
use plugins::{PluginInstallResult, PluginManifest, PluginUpgradePreview, MarketplaceEntry};
use gateway_desktop_settings::DesktopPluginDependencyDto;
#[cfg(test)]
use gateway_desktop_settings::{DesktopPluginCatalogEntryDto, DesktopPluginDetailDto, DesktopPluginPathDto};
#[cfg(test)]
use gateway_plugins::{PluginDetailQuery, PluginMutationRequest, PluginSetEnabledRequest};

struct Host;
impl PluginHost for Host {
    type State = AppState;
    type Upgrade = PluginUpgradePreview;
    fn state_config(state: &AppState) -> Config { state.config.lock().clone() }
    fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
        super::super::auth::require_auth(state, headers)
            .map_err(|(status, _)| ApiError { status, message: "Unauthorized".into() })
    }
    fn pending() -> &'static Mutex<HashMap<String, Pending<PluginUpgradePreview>>> {
        static PENDING: OnceLock<Mutex<HashMap<String, Pending<PluginUpgradePreview>>>> = OnceLock::new();
        PENDING.get_or_init(|| Mutex::new(HashMap::new()))
    }
    fn set_plugin_enabled(root: &Path, id: &str, enabled: bool) -> Result<()> {
        plugins::set_plugin_enabled(root, id, enabled)
    }
    fn remove_installed_plugin(root: &Path, id: &str) -> Result<()> {
        plugins::remove_installed_plugin(root, id)
    }
    fn inspect_plugin_dependencies(root: &Path) -> Result<Vec<DesktopPluginDependencyDto>> {
        plugins::inspect_plugin_dependencies(root).map(|checks| checks.into_iter().map(|check| DesktopPluginDependencyDto {
            status: check.state.as_str().into(), component: check.component,
            dependency: check.dependency, detail: check.detail,
        }).collect())
    }
    fn install_plugin_marketplace_entry(entry: &MarketplaceEntry, root: &Path) -> Result<PluginInstallResult> {
        plugins::install_plugin_marketplace_entry(entry, root)
    }
    fn install_plugin_from_source(source: &str, root: &Path) -> Result<PluginInstallResult> {
        plugins::install_plugin_from_source(source, root)
    }
    fn prepare_plugin_upgrade(source: &str, root: &Path, id: &str) -> Result<PluginUpgradePreview> {
        plugins::prepare_plugin_upgrade(source, root, id)
    }
    fn prepare_plugin_marketplace_upgrade(entry: &MarketplaceEntry, root: &Path) -> Result<PluginUpgradePreview> {
        plugins::prepare_plugin_marketplace_upgrade(entry, root)
    }
    fn activate_plugin_upgrade(preview: PluginUpgradePreview) -> Result<()> {
        plugins::activate_plugin_upgrade(preview)
    }
    fn manifest(preview: &PluginUpgradePreview) -> &PluginManifest { preview.manifest() }
    fn old_manifest(preview: &PluginUpgradePreview) -> &PluginManifest { preview.old_manifest() }
    fn identity_key(preview: &PluginUpgradePreview) -> String { preview.identity_key() }
}

pub(crate) fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    gateway_plugins::router::<S, Host>()
}
pub(crate) fn stateful_router() -> Router<AppState> { gateway_plugins::stateful_router::<Host>() }

#[cfg(test)]
use gateway_plugins::non_empty_string;

#[cfg(test)]
fn collect_plugin_catalog(workspace: impl Into<PathBuf>, config: impl Into<PathBuf>, plugins_config: config_plugins::PluginsConfig) -> Result<Vec<DesktopPluginCatalogEntryDto>> { gateway_plugins::collect_plugin_catalog::<Host>(workspace, config, plugins_config) }

#[cfg(test)]
fn resolve_plugin_detail(workspace: impl Into<PathBuf>, config: impl Into<PathBuf>, plugins_config: config_plugins::PluginsConfig, id: &str) -> Result<DesktopPluginDetailDto> { gateway_plugins::resolve_plugin_detail::<Host>(workspace, config, plugins_config, id) }

#[cfg(test)]
async fn detail_get(query: Query<PluginDetailQuery>) -> Result<Json<DesktopPluginDetailDto>, ApiError> { gateway_plugins::detail_get::<Host>(query).await }

#[cfg(test)]
async fn set_enabled_post(body: Json<PluginSetEnabledRequest>) -> Result<Json<DesktopPluginPathDto>, ApiError> { gateway_plugins::set_enabled_post::<Host>(body).await }

#[cfg(test)]
async fn delete_post(body: Json<PluginMutationRequest>) -> Result<Json<DesktopPluginPathDto>, ApiError> { gateway_plugins::delete_post::<Host>(body).await }

#[cfg(test)]
#[path = "../../../../../libs/gateway-plugins/src/plugins_tests.rs"]
mod plugins_tests;
