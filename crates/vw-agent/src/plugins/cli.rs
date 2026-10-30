//! 插件包管理的命令行处理入口。

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

use super::install::{
    install_dir_from_config, install_plugin_from_source, remove_installed_plugin,
    set_plugin_enabled,
};
use super::{PluginCommands, PluginDiscoveryOptions, PluginManifest, discover_plugins};

/// 执行插件列表、审计、安装、启停和移除命令。
///
/// 各分支只输出完成后的结果；路径与清单错误原样向调用方传播。
pub fn handle_command(command: PluginCommands, config: &crate::config::Config) -> Result<()> {
    let plugins_dir = install_dir_from_config(config);

    match command {
        PluginCommands::List => list_plugins(config, &plugins_dir),
        PluginCommands::Audit { source } => audit_plugin_source(&source, &plugins_dir),
        PluginCommands::Install { source } => {
            let result = install_plugin_from_source(&source, &plugins_dir)?;
            println!(
                "Plugin installed: {} {} at {}",
                result.identity.as_key(),
                result.manifest.version.as_deref().unwrap_or(""),
                result.installed_dir.display()
            );
            Ok(())
        }
        PluginCommands::Enable { name } => {
            set_plugin_enabled(&plugins_dir, &name, true)?;
            println!("Plugin enabled: {name}");
            Ok(())
        }
        PluginCommands::Disable { name } => {
            set_plugin_enabled(&plugins_dir, &name, false)?;
            println!("Plugin disabled: {name}");
            Ok(())
        }
        PluginCommands::Remove { name } => {
            remove_installed_plugin(&plugins_dir, &name)?;
            println!("Plugin removed: {name}");
            Ok(())
        }
    }
}

fn list_plugins(config: &crate::config::Config, plugins_dir: &Path) -> Result<()> {
    let config_dir = config
        .config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| config.workspace_dir.clone());
    let options = PluginDiscoveryOptions::new(
        config.workspace_dir.clone(),
        config_dir,
        config.plugins.clone(),
    );
    let discovery = discover_plugins(&options)?;

    if discovery.installed.is_empty() {
        println!("No plugins installed.");
        return Ok(());
    }

    println!("Installed plugins ({}):", discovery.installed.len());
    for plugin in discovery.installed {
        let status = if plugin.disabled_marker.is_some() { "disabled" } else { "enabled" };
        let components = component_summary(&plugin.manifest);
        let location = if plugin.source_dir == plugins_dir {
            "global"
        } else {
            plugin.source_dir.to_str().unwrap_or("custom")
        };
        println!(
            "  {} {} [{}] - {} ({})",
            plugin.identity.as_key(),
            plugin.manifest.version.as_deref().unwrap_or(""),
            status,
            components,
            location
        );
    }

    Ok(())
}

fn audit_plugin_source(source: &str, plugins_dir: &Path) -> Result<()> {
    let source = source.trim();
    if source.is_empty() {
        bail!("Plugin audit source cannot be empty");
    }

    let source_path = PathBuf::from(source);
    let target = if source_path.exists() { source_path } else { plugins_dir.join(source) };
    let manifest = super::load_plugin_manifest(&target)
        .with_context(|| format!("Plugin audit failed for {}", target.display()))?;
    println!(
        "Plugin audit passed: {} {} (components: {})",
        manifest.name,
        manifest.version.as_deref().unwrap_or(""),
        component_summary(&manifest)
    );

    Ok(())
}

fn component_summary(manifest: &PluginManifest) -> String {
    let mut components = Vec::new();
    if !manifest.skills.is_empty() {
        components.push("skills");
    }
    if manifest.mcp_servers.is_some() {
        components.push("mcp");
    }
    if !manifest.hooks.is_empty() {
        components.push("hooks");
    }
    if manifest.interface.is_some() {
        components.push("interface");
    }

    if components.is_empty() { "manifest".to_string() } else { components.join(",") }
}
