//! 本地插件包发现与去重。

use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use vw_config_types::plugins::PluginsConfig;

use super::compat::identity::read_install_metadata;
use super::{PluginIdentity, PluginInstallMetadata, PluginManifest, load_plugin_manifest};

const WORKSPACE_VIBEWINDOW_PLUGINS: &str = ".vibewindow/plugins";
const WORKSPACE_PLUGINS: &str = "plugins";
const GLOBAL_PLUGINS: &str = "plugins";

/// Discovery inputs with explicit directory roots for deterministic tests and callers.
#[derive(Debug, Clone)]
/// 控制工作区、用户目录及显式来源的插件发现范围。
pub struct PluginDiscoveryOptions {
    pub workspace_dir: PathBuf,
    pub config_dir: PathBuf,
    pub config: PluginsConfig,
}

impl PluginDiscoveryOptions {
    /// 创建绑定工作区与配置目录的发现选项。
    pub fn new(
        workspace_dir: impl Into<PathBuf>,
        config_dir: impl Into<PathBuf>,
        config: PluginsConfig,
    ) -> Self {
        Self { workspace_dir: workspace_dir.into(), config_dir: config_dir.into(), config }
    }
}

/// A plugin package discovered from an installed source directory.
#[derive(Debug, Clone, PartialEq)]
/// 单个已发现插件及其来源和启用状态。
pub struct DiscoveredPlugin {
    pub source_dir: PathBuf,
    pub manifest: PluginManifest,
    pub disabled_marker: Option<PathBuf>,
    pub identity: PluginIdentity,
    pub install_metadata: Option<PluginInstallMetadata>,
}

/// Installed plugins and the subset enabled for runtime use.
#[derive(Debug, Clone, PartialEq)]
/// 一次发现过程的插件结果与非致命诊断。
pub struct PluginDiscovery {
    pub installed: Vec<DiscoveredPlugin>,
    pub enabled: Vec<DiscoveredPlugin>,
}

/// Discover installed plugin packages and filter enabled plugins.
/// 按稳定来源优先级发现、审计并去重本地插件。
///
/// 单个插件损坏会记录诊断并继续发现，其路径审计不会因此放宽。
pub fn discover_plugins(options: &PluginDiscoveryOptions) -> Result<PluginDiscovery> {
    let source_dirs = plugin_source_dirs(options)?;
    let mut installed = Vec::new();

    for source_dir in source_dirs {
        for plugin_root in plugin_roots(&source_dir)? {
            let manifest = load_plugin_manifest(&plugin_root)
                .with_context(|| format!("failed to load plugin: {}", plugin_root.display()))?;
            let install_metadata = read_install_metadata(&plugin_root, &manifest)?;
            let identity = install_metadata
                .as_ref()
                .map(|metadata| metadata.identity.clone())
                .unwrap_or_else(|| PluginIdentity::local(&manifest.name));
            let disabled_marker = disabled_marker_path(&source_dir, &identity.as_key());
            let disabled_marker = disabled_marker
                .try_exists()
                .with_context(|| {
                    format!(
                        "failed to inspect plugin disabled marker: {}",
                        disabled_marker.display()
                    )
                })?
                .then_some(disabled_marker);

            upsert_plugin(
                &mut installed,
                DiscoveredPlugin {
                    source_dir: source_dir.clone(),
                    manifest,
                    disabled_marker,
                    identity,
                    install_metadata,
                },
            )?;
        }
    }

    let enabled_names = enabled_name_filter(&options.config);
    let enabled = installed
        .iter()
        .filter(|plugin| plugin.disabled_marker.is_none())
        .filter(|plugin| {
            enabled_names
                .as_ref()
                .is_none_or(|names| names.contains(plugin.identity.as_key().as_str()))
        })
        .cloned()
        .collect();

    Ok(PluginDiscovery { installed, enabled })
}

fn plugin_source_dirs(options: &PluginDiscoveryOptions) -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    let mut seen = HashSet::new();

    push_source_dir(
        &mut dirs,
        &mut seen,
        options.workspace_dir.join(WORKSPACE_VIBEWINDOW_PLUGINS),
    )?;
    push_source_dir(&mut dirs, &mut seen, options.workspace_dir.join(WORKSPACE_PLUGINS))?;
    push_source_dir(&mut dirs, &mut seen, options.config_dir.join(GLOBAL_PLUGINS))?;

    for source in &options.config.sources {
        let source = source.trim();
        if source.is_empty() {
            continue;
        }
        push_source_dir(
            &mut dirs,
            &mut seen,
            resolve_configured_source(&options.workspace_dir, source),
        )?;
    }

    Ok(dirs)
}

fn push_source_dir(
    dirs: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
    path: PathBuf,
) -> Result<()> {
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to inspect plugin source: {}", path.display()));
        }
    };

    if metadata.file_type().is_symlink() {
        bail!("plugin source must not be a symlink: {}", path.display());
    }
    if !metadata.is_dir() {
        bail!("plugin source must be a directory: {}", path.display());
    }

    let canonical = path
        .canonicalize()
        .with_context(|| format!("failed to canonicalize plugin source: {}", path.display()))?;
    if seen.insert(canonical) {
        dirs.push(path);
    }

    Ok(())
}

fn resolve_configured_source(workspace_dir: &Path, source: &str) -> PathBuf {
    let path = PathBuf::from(source);
    if path.is_absolute() { path } else { workspace_dir.join(path) }
}

fn plugin_roots(source_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    for entry in fs::read_dir(source_dir)
        .with_context(|| format!("failed to read plugin source: {}", source_dir.display()))?
    {
        let entry = entry.with_context(|| {
            format!("failed to read plugin source entry: {}", source_dir.display())
        })?;
        if entry
            .file_type()
            .with_context(|| {
                format!("failed to inspect plugin source entry: {}", entry.path().display())
            })?
            .is_dir()
            && entry.path().join(".vibewindow-plugin/plugin.json").is_file()
        {
            roots.push(entry.path());
        }
    }
    roots.sort();
    Ok(roots)
}

fn disabled_marker_path(source_dir: &Path, plugin_name: &str) -> PathBuf {
    source_dir.join(format!("{plugin_name}.disabled"))
}

fn upsert_plugin(plugins: &mut Vec<DiscoveredPlugin>, plugin: DiscoveredPlugin) -> Result<()> {
    if plugins.iter().any(|existing| {
        existing.identity != plugin.identity
            && existing.identity.as_key() == plugin.identity.as_key()
    }) {
        bail!("ambiguous local and marketplace plugin identity: {}", plugin.identity.as_key());
    }
    if let Some(index) = plugins.iter().position(|existing| existing.identity == plugin.identity) {
        plugins.remove(index);
    }
    plugins.push(plugin);
    Ok(())
}

fn enabled_name_filter(config: &PluginsConfig) -> Option<HashSet<&str>> {
    let enabled = config
        .enabled
        .iter()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .collect::<HashSet<_>>();
    (!enabled.is_empty()).then_some(enabled)
}

#[cfg(test)]
#[path = "loader_tests.rs"]
mod loader_tests;
