//! Gateway plugin catalog and local management handlers.

use anyhow::{Result, bail};
use axum::extract::Query;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use vw_config_types::plugins::PluginsConfig;
use vw_gateway_client::{
    DesktopPluginCapabilitiesDto, DesktopPluginCatalogEntryDto, DesktopPluginDetailDto,
    DesktopPluginPathDto,
};

use crate::app::agent::gateway::ApiError;
use crate::plugins::{
    DiscoveredPlugin, PluginDiscoveryOptions, PluginManifest, discover_plugins,
    install_dir_from_config, remove_installed_plugin, set_plugin_enabled,
};

/// 构建前端中立的插件 API 路由表。
pub(crate) fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/plugins", get(catalog_get))
        .route("/plugins/detail", get(detail_get))
        .route("/plugins/set-enabled", post(set_enabled_post))
        .route("/plugins/delete", post(delete_post))
}

#[derive(Debug, Deserialize)]
pub(crate) struct PluginDetailQuery {
    pub(crate) plugin_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PluginMutationRequest {
    pub(crate) plugin_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PluginSetEnabledRequest {
    pub(crate) plugin_id: String,
    pub(crate) enabled: bool,
}

/// 获取已发现插件目录。
pub(crate) async fn catalog_get() -> Result<Json<Vec<DesktopPluginCatalogEntryDto>>, ApiError> {
    let config = crate::app::agent::config::get().await;
    let workspace_dir = config.workspace_dir.clone();
    let config_dir = config_dir_from_config(&config);
    let plugins_config = config.plugins.clone();

    let items = tokio::task::spawn_blocking(move || {
        collect_plugin_catalog(workspace_dir, config_dir, plugins_config)
    })
    .await
    .map_err(|err| ApiError::internal(format!("plugins catalog task failed: {err}")))?
    .map_err(|err| ApiError::bad_request(err.to_string()))?;

    Ok(Json(items))
}

/// 获取单个插件详情。
pub(crate) async fn detail_get(
    Query(query): Query<PluginDetailQuery>,
) -> Result<Json<DesktopPluginDetailDto>, ApiError> {
    let plugin_id = normalize_plugin_id(&query.plugin_id)
        .map_err(|err| ApiError::bad_request(err.to_string()))?;
    let config = crate::app::agent::config::get().await;
    let workspace_dir = config.workspace_dir.clone();
    let config_dir = config_dir_from_config(&config);
    let plugins_config = config.plugins.clone();

    let detail = tokio::task::spawn_blocking(move || {
        resolve_plugin_detail(workspace_dir, config_dir, plugins_config, &plugin_id)
    })
    .await
    .map_err(|err| ApiError::internal(format!("plugin detail task failed: {err}")))?
    .map_err(|err| ApiError::bad_request(err.to_string()))?;

    Ok(Json(detail))
}

/// 设置本地插件启用状态。
pub(crate) async fn set_enabled_post(
    Json(body): Json<PluginSetEnabledRequest>,
) -> Result<Json<DesktopPluginPathDto>, ApiError> {
    let plugin_id = normalize_plugin_id(&body.plugin_id)
        .map_err(|err| ApiError::bad_request(err.to_string()))?;
    let config = crate::app::agent::config::get().await;
    let enabled = body.enabled;

    let path = tokio::task::spawn_blocking(move || {
        let plugins_dir = install_dir_from_config(&config);
        set_plugin_enabled(&plugins_dir, &plugin_id, enabled)?;
        Ok::<String, anyhow::Error>(path_string(&plugins_dir.join(plugin_id)))
    })
    .await
    .map_err(|err| ApiError::internal(format!("plugin toggle task failed: {err}")))?
    .map_err(|err| ApiError::bad_request(err.to_string()))?;

    Ok(Json(DesktopPluginPathDto { path }))
}

/// 删除本地插件目录。
pub(crate) async fn delete_post(
    Json(body): Json<PluginMutationRequest>,
) -> Result<Json<DesktopPluginPathDto>, ApiError> {
    let plugin_id = normalize_plugin_id(&body.plugin_id)
        .map_err(|err| ApiError::bad_request(err.to_string()))?;
    let config = crate::app::agent::config::get().await;

    let path = tokio::task::spawn_blocking(move || {
        let plugins_dir = install_dir_from_config(&config);
        let path = plugins_dir.join(&plugin_id);
        remove_installed_plugin(&plugins_dir, &plugin_id)?;
        Ok::<String, anyhow::Error>(path_string(&path))
    })
    .await
    .map_err(|err| ApiError::internal(format!("plugin delete task failed: {err}")))?
    .map_err(|err| ApiError::bad_request(err.to_string()))?;

    Ok(Json(DesktopPluginPathDto { path }))
}

pub(crate) fn collect_plugin_catalog(
    workspace_dir: impl Into<PathBuf>,
    config_dir: impl Into<PathBuf>,
    plugins_config: PluginsConfig,
) -> Result<Vec<DesktopPluginCatalogEntryDto>> {
    let config_dir = config_dir.into();
    let install_dir = config_dir.join("plugins");
    let options = PluginDiscoveryOptions::new(workspace_dir, config_dir, plugins_config);
    let discovery = discover_plugins(&options)?;
    let mut items = discovery
        .installed
        .into_iter()
        .map(|plugin| catalog_entry(&plugin, &install_dir, &options.workspace_dir))
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        left.display_name.cmp(&right.display_name).then(left.id.cmp(&right.id))
    });
    Ok(items)
}

pub(crate) fn resolve_plugin_detail(
    workspace_dir: impl Into<PathBuf>,
    config_dir: impl Into<PathBuf>,
    plugins_config: PluginsConfig,
    plugin_id: &str,
) -> Result<DesktopPluginDetailDto> {
    let plugin_id = normalize_plugin_id(plugin_id)?;
    let config_dir = config_dir.into();
    let install_dir = config_dir.join("plugins");
    let options = PluginDiscoveryOptions::new(workspace_dir, config_dir, plugins_config);
    let discovery = discover_plugins(&options)?;
    let plugin = discovery
        .installed
        .into_iter()
        .find(|plugin| plugin.identity.as_key() == plugin_id)
        .ok_or_else(|| anyhow::anyhow!("Plugin not found: {plugin_id}"))?;
    Ok(detail_entry(&plugin, &install_dir, &options.workspace_dir))
}

fn config_dir_from_config(config: &crate::config::Config) -> PathBuf {
    config
        .config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| config.workspace_dir.clone())
}

fn normalize_plugin_id(plugin_id: &str) -> Result<String> {
    let plugin_id = plugin_id.trim();
    if plugin_id.is_empty() {
        bail!("plugin_id is required");
    }
    Ok(plugin_id.to_string())
}

fn catalog_entry(
    plugin: &DiscoveredPlugin,
    install_dir: &Path,
    workspace: &Path,
) -> DesktopPluginCatalogEntryDto {
    let manifest = &plugin.manifest;
    DesktopPluginCatalogEntryDto {
        id: plugin.identity.as_key(),
        display_name: plugin_display_name(manifest),
        version: manifest.version.clone().unwrap_or_default(),
        description: plugin_description(manifest),
        enabled: plugin.disabled_marker.is_none(),
        component_count: component_count(manifest),
        dependency_checks: dependency_checks(manifest),
        origin: source_origin(plugin.install_metadata.as_ref(), manifest.version.clone()),
        hook_review: hook_review(manifest, install_dir.parent().unwrap_or(install_dir), workspace),
        source_path: Some(path_string(&manifest.root)),
    }
}

fn detail_entry(
    plugin: &DiscoveredPlugin,
    install_dir: &Path,
    workspace: &Path,
) -> DesktopPluginDetailDto {
    let manifest = &plugin.manifest;
    let manageable = plugin.source_dir == install_dir;
    DesktopPluginDetailDto {
        id: plugin.identity.as_key(),
        display_name: plugin_display_name(manifest),
        version: manifest.version.clone().unwrap_or_default(),
        description: plugin_description(manifest),
        enabled: plugin.disabled_marker.is_none(),
        source_path: Some(path_string(&manifest.root)),
        manifest_path: path_string(&manifest.manifest_path),
        capabilities: plugin_capabilities(manifest),
        dependency_checks: dependency_checks(manifest),
        origin: source_origin(plugin.install_metadata.as_ref(), manifest.version.clone()),
        hook_review: hook_review(manifest, install_dir.parent().unwrap_or(install_dir), workspace),
        can_toggle: manageable,
        can_delete: manageable,
    }
}

fn plugin_display_name(manifest: &PluginManifest) -> String {
    manifest
        .interface
        .as_ref()
        .and_then(|interface| non_empty_string(interface.display_name.as_deref()))
        .unwrap_or_else(|| manifest.name.clone())
}

fn plugin_description(manifest: &PluginManifest) -> String {
    manifest
        .interface
        .as_ref()
        .and_then(|interface| non_empty_string(interface.short_description.as_deref()))
        .unwrap_or_else(|| manifest.description.clone().unwrap_or_default())
}

fn non_empty_string(value: Option<&str>) -> Option<String> {
    value.map(str::trim).filter(|value| !value.is_empty()).map(ToOwned::to_owned)
}

fn plugin_capabilities(manifest: &PluginManifest) -> DesktopPluginCapabilitiesDto {
    DesktopPluginCapabilitiesDto {
        skills: manifest.skills.len(),
        mcp_servers: usize::from(manifest.mcp_servers.is_some()),
        hooks: manifest.hooks.len(),
        interface: manifest.interface.is_some(),
    }
}

fn component_count(manifest: &PluginManifest) -> usize {
    let capabilities = plugin_capabilities(manifest);
    capabilities.skills
        + capabilities.mcp_servers
        + capabilities.hooks
        + usize::from(capabilities.interface)
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
#[path = "plugins_tests.rs"]
mod plugins_tests;

#[path = "plugins_operations.rs"]
mod operations;

/// Mutations use the authenticated running gateway state, never a client-supplied install root.
pub(crate) fn stateful_router() -> Router<crate::gateway::AppState> {
    operations::router()
}

fn dependency_checks(
    manifest: &PluginManifest,
) -> Vec<vw_gateway_client::DesktopPluginDependencyDto> {
    match crate::plugins::inspect_plugin_dependencies(&manifest.root) {
        Ok(checks) => checks
            .into_iter()
            .map(|check| vw_gateway_client::DesktopPluginDependencyDto {
                status: check.state.as_str().into(),
                component: check.component,
                dependency: check.dependency,
                detail: check.detail,
            })
            .collect(),
        Err(_) => vec![vw_gateway_client::DesktopPluginDependencyDto {
            component: "package".into(),
            dependency: "声明与路径".into(),
            status: "invalid".into(),
            detail: "依赖声明未通过安全解析；请检查原始包与诊断".into(),
        }],
    }
}
fn dependency_status(checks: &[vw_gateway_client::DesktopPluginDependencyDto]) -> String {
    if checks.is_empty() {
        "no_declared_dependencies"
    } else if checks.iter().any(|c| matches!(c.status.as_str(), "missing_host" | "missing_package"))
    {
        "missing"
    } else if checks.iter().any(|c| matches!(c.status.as_str(), "unverified" | "invalid")) {
        "unverified"
    } else {
        "checked_presence"
    }
    .into()
}

fn source_origin(
    metadata: Option<&crate::plugins::PluginInstallMetadata>,
    version: Option<String>,
) -> vw_gateway_client::DesktopPluginSourceDto {
    let value: serde_json::Value =
        metadata.and_then(|m| serde_json::from_str(&m.source).ok()).unwrap_or_default();
    let field = |key| value.get(key).and_then(serde_json::Value::as_str).map(str::to_owned);
    vw_gateway_client::DesktopPluginSourceDto {
        kind: if metadata.is_none() {
            "directory"
        } else if value.get("npm").is_some() {
            "npm"
        } else if value.get("git").is_some() {
            "git"
        } else {
            "local"
        }
        .into(),
        resolved_version: field("version").or(version),
        revision: field("commit"),
        sha256: field("sha256"),
        marketplace: metadata.and_then(|m| m.identity.marketplace_name()).map(str::to_owned),
    }
}
fn hook_review(manifest: &PluginManifest, config_dir: &Path, workspace: &Path) -> String {
    use crate::plugins::{
        PluginHookTrustStatus, PluginHookTrustStore, PluginHooksManifest, load_plugin_hook_source,
        plugin_hook_trust_file,
    };
    if manifest.hooks.is_empty() {
        return "not_declared".into();
    }
    let result = (|| -> Result<&str> {
        let store = PluginHookTrustStore::load_or_default(&plugin_hook_trust_file(config_dir))?;
        let mut pending = false;
        for source in &manifest.hooks {
            match load_plugin_hook_source(source, &manifest.root, workspace)? {
                PluginHooksManifest::Legacy(hooks) => {
                    pending |= hooks.iter().any(|hook| {
                        store.review_status(&manifest.name, hook) != PluginHookTrustStatus::Trusted
                    })
                }
                PluginHooksManifest::Codex(hooks) => {
                    if !hooks.diagnostics().is_empty() {
                        return Ok("unsupported");
                    }
                    if hooks.hooks.values().any(|group| !group.is_empty()) {
                        pending |=
                            !store.codex_hooks_trusted(&manifest.name, &manifest.root, &hooks)?;
                    }
                }
            }
        }
        Ok(if pending { "pending" } else { "trusted" })
    })();
    result.unwrap_or("unverified").into()
}
