//! 本地插件包发现与去重。

use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use vw_config_types::plugins::PluginsConfig;

use super::{PluginManifest, load_plugin_manifest};

const WORKSPACE_VIBEWINDOW_PLUGINS: &str = ".vibewindow/plugins";
const WORKSPACE_PLUGINS: &str = "plugins";
const GLOBAL_PLUGINS: &str = "plugins";

/// Discovery inputs with explicit directory roots for deterministic tests and callers.
#[derive(Debug, Clone)]
pub struct PluginDiscoveryOptions {
    pub workspace_dir: PathBuf,
    pub config_dir: PathBuf,
    pub config: PluginsConfig,
}

impl PluginDiscoveryOptions {
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
pub struct DiscoveredPlugin {
    pub source_dir: PathBuf,
    pub manifest: PluginManifest,
    pub disabled_marker: Option<PathBuf>,
}

/// Installed plugins and the subset enabled for runtime use.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginDiscovery {
    pub installed: Vec<DiscoveredPlugin>,
    pub enabled: Vec<DiscoveredPlugin>,
}

/// Discover installed plugin packages and filter enabled plugins.
pub fn discover_plugins(options: &PluginDiscoveryOptions) -> Result<PluginDiscovery> {
    let source_dirs = plugin_source_dirs(options)?;
    let mut installed = Vec::new();

    for source_dir in source_dirs {
        for plugin_root in plugin_roots(&source_dir)? {
            let manifest = load_plugin_manifest(&plugin_root)
                .with_context(|| format!("failed to load plugin: {}", plugin_root.display()))?;
            let disabled_marker = disabled_marker_path(&source_dir, &manifest.name);
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
                DiscoveredPlugin { source_dir: source_dir.clone(), manifest, disabled_marker },
            );
        }
    }

    let enabled_names = enabled_name_filter(&options.config);
    let enabled = installed
        .iter()
        .filter(|plugin| plugin.disabled_marker.is_none())
        .filter(|plugin| {
            enabled_names.as_ref().is_none_or(|names| names.contains(plugin.manifest.name.as_str()))
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

fn upsert_plugin(plugins: &mut Vec<DiscoveredPlugin>, plugin: DiscoveredPlugin) {
    if let Some(index) =
        plugins.iter().position(|existing| existing.manifest.name == plugin.manifest.name)
    {
        plugins.remove(index);
    }
    plugins.push(plugin);
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
