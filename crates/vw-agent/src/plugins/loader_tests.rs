//! 验证插件来源发现、禁用过滤、去重优先级与无效目录处理。

use super::{DiscoveredPlugin, PluginDiscoveryOptions, discover_plugins};
use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use vw_config_types::plugins::PluginsConfig;

fn write_plugin_manifest(plugin_root: &Path, name: &str) {
    let manifest_dir = plugin_root.join(".vibewindow-plugin");
    fs::create_dir_all(&manifest_dir).expect("manifest dir");
    fs::write(
        manifest_dir.join("plugin.json"),
        serde_json::to_vec_pretty(&json!({
            "name": name,
            "version": "0.1.0",
            "description": format!("{name} test plugin")
        }))
        .expect("manifest json"),
    )
    .expect("write manifest");
}

fn plugin_names<'a>(plugins: impl IntoIterator<Item = &'a DiscoveredPlugin>) -> Vec<&'a str> {
    plugins.into_iter().map(|plugin| plugin.manifest.name.as_str()).collect()
}

#[test]
fn plugin_loader_discovers_project_and_global_plugins() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let project_plugins = workspace_dir.join(".vibewindow/plugins");
    let global_plugins = config_dir.join("plugins");
    write_plugin_manifest(&project_plugins.join("project-plugin"), "project-plugin");
    write_plugin_manifest(&global_plugins.join("global-plugin"), "global-plugin");

    let options = PluginDiscoveryOptions::new(workspace_dir, config_dir, PluginsConfig::default());
    let discovery = discover_plugins(&options).expect("discover plugins");

    let installed = plugin_names(&discovery.installed);
    let enabled = plugin_names(&discovery.enabled);
    println!("discovered plugin names: {:?}", installed);
    assert_eq!(installed, vec!["project-plugin", "global-plugin"]);
    assert_eq!(enabled, installed);
}

#[test]
fn plugin_loader_skips_disabled_plugin() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let project_plugins = workspace_dir.join(".vibewindow/plugins");
    write_plugin_manifest(&project_plugins.join("active-plugin"), "active-plugin");
    write_plugin_manifest(&project_plugins.join("muted-plugin"), "muted-plugin");
    fs::write(project_plugins.join("muted-plugin.disabled"), "").expect("disabled marker");

    let options = PluginDiscoveryOptions::new(workspace_dir, config_dir, PluginsConfig::default());
    let discovery = discover_plugins(&options).expect("discover plugins");

    let installed = plugin_names(&discovery.installed);
    let enabled = plugin_names(&discovery.enabled);
    println!("enabled plugins after disabled marker: {:?}", enabled);
    assert_eq!(installed, vec!["active-plugin", "muted-plugin"]);
    assert_eq!(enabled, vec!["active-plugin"]);
}

#[test]
fn plugin_loader_uses_configured_source_and_enabled_filter() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    write_plugin_manifest(
        &workspace_dir.join(".vibewindow/plugins/project-plugin"),
        "project-plugin",
    );
    write_plugin_manifest(&workspace_dir.join("extra/configured-plugin"), "configured-plugin");
    let config = PluginsConfig {
        enabled: vec![" configured-plugin ".to_string()],
        sources: vec![" ".to_string(), " extra ".to_string()],
    };
    let options = PluginDiscoveryOptions::new(workspace_dir, config_dir, config);

    let discovery = discover_plugins(&options).expect("discover plugins");

    assert_eq!(plugin_names(&discovery.installed), vec!["project-plugin", "configured-plugin"]);
    assert_eq!(plugin_names(&discovery.enabled), vec!["configured-plugin"]);
}

#[test]
fn plugin_loader_accepts_absolute_configured_source_and_deduplicates_canonical_sources() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let project_plugins = workspace_dir.join(".vibewindow/plugins");
    write_plugin_manifest(&project_plugins.join("project-plugin"), "project-plugin");
    let absolute_source = project_plugins.to_string_lossy().into_owned();
    let config =
        PluginsConfig { enabled: vec![], sources: vec![absolute_source.clone(), absolute_source] };
    let options = PluginDiscoveryOptions::new(workspace_dir, config_dir, config);

    let discovery = discover_plugins(&options).expect("discover plugins");

    assert_eq!(plugin_names(&discovery.installed), vec!["project-plugin"]);
    assert_eq!(discovery.installed[0].source_dir, project_plugins);
}

#[test]
fn plugin_loader_prefers_later_source_for_duplicate_name() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let project_plugins = workspace_dir.join(".vibewindow/plugins");
    let global_plugins = config_dir.join("plugins");
    write_plugin_manifest(&project_plugins.join("project-copy"), "duplicate-plugin");
    write_plugin_manifest(&global_plugins.join("global-copy"), "duplicate-plugin");
    let options = PluginDiscoveryOptions::new(workspace_dir, config_dir, PluginsConfig::default());

    let discovery = discover_plugins(&options).expect("discover plugins");

    assert_eq!(discovery.installed.len(), 1);
    assert_eq!(discovery.installed[0].manifest.name, "duplicate-plugin");
    assert_eq!(discovery.installed[0].source_dir, global_plugins);
}

#[test]
#[cfg(unix)]
fn plugin_loader_rejects_source_that_is_symlink() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let real_plugins = dir.path().join("real-plugins");
    fs::create_dir_all(workspace_dir.join(".vibewindow")).expect("workspace plugin parent");
    fs::create_dir_all(&real_plugins).expect("real plugins dir");
    std::os::unix::fs::symlink(&real_plugins, workspace_dir.join(".vibewindow/plugins"))
        .expect("plugin source symlink");
    let options = PluginDiscoveryOptions::new(workspace_dir, config_dir, PluginsConfig::default());

    let err = discover_plugins(&options).expect_err("symlink source rejected").to_string();

    assert!(err.contains("plugin source must not be a symlink"), "{err}");
    assert!(err.contains(".vibewindow/plugins"), "{err}");
}

#[test]
fn plugin_loader_rejects_source_that_is_not_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    fs::create_dir_all(&workspace_dir).expect("workspace dir");
    fs::write(workspace_dir.join("plugins"), "not a directory").expect("invalid source");
    let options = PluginDiscoveryOptions::new(workspace_dir, config_dir, PluginsConfig::default());

    let err = discover_plugins(&options).expect_err("non-directory source rejected").to_string();

    assert!(err.contains("plugin source must be a directory"), "{err}");
    assert!(err.contains("plugins"), "{err}");
}

#[test]
fn plugin_loader_propagates_configured_source_errors() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    fs::create_dir_all(&workspace_dir).expect("workspace dir");
    fs::write(workspace_dir.join("extra-source"), "not a directory").expect("invalid source");
    let config = PluginsConfig { enabled: vec![], sources: vec!["extra-source".to_string()] };
    let options = PluginDiscoveryOptions::new(workspace_dir, config_dir, config);

    let err = discover_plugins(&options).expect_err("configured non-directory source rejected");
    let err = err.to_string();

    assert!(err.contains("plugin source must be a directory"), "{err}");
    assert!(err.contains("extra-source"), "{err}");
}

#[test]
#[cfg(unix)]
fn plugin_loader_reports_disabled_marker_inspection_errors() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let project_plugins = workspace_dir.join(".vibewindow/plugins");
    write_plugin_manifest(&project_plugins.join("nested-name"), "hidden/plugin");
    let hidden_dir = project_plugins.join("hidden");
    fs::create_dir_all(&hidden_dir).expect("hidden marker parent");
    fs::set_permissions(&hidden_dir, fs::Permissions::from_mode(0o000))
        .expect("lock marker parent");

    let options = PluginDiscoveryOptions::new(workspace_dir, config_dir, PluginsConfig::default());
    let err = discover_plugins(&options).expect_err("disabled marker inspection fails");
    fs::set_permissions(&hidden_dir, fs::Permissions::from_mode(0o755))
        .expect("restore marker parent");
    let err = err.to_string();

    assert!(err.contains("failed to inspect plugin disabled marker"), "{err}");
    assert!(err.contains("hidden/plugin.disabled"), "{err}");
}

#[test]
#[cfg(unix)]
fn push_source_dir_reports_metadata_errors() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("temp dir");
    let locked_parent = dir.path().join("locked");
    fs::create_dir_all(&locked_parent).expect("locked parent");
    fs::set_permissions(&locked_parent, fs::Permissions::from_mode(0o000)).expect("lock parent");
    let mut dirs = Vec::new();
    let mut seen = HashSet::new();

    let err = super::push_source_dir(&mut dirs, &mut seen, locked_parent.join("plugins"))
        .expect_err("metadata error propagated");
    fs::set_permissions(&locked_parent, fs::Permissions::from_mode(0o755)).expect("restore parent");
    let err = err.to_string();

    assert!(err.contains("failed to inspect plugin source"), "{err}");
    assert!(err.contains("plugins"), "{err}");
    assert!(dirs.is_empty());
}

#[test]
fn plugin_loader_reports_plugin_root_when_manifest_load_fails() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/broken-plugin");
    let manifest_dir = plugin_root.join(".vibewindow-plugin");
    fs::create_dir_all(&manifest_dir).expect("manifest dir");
    fs::write(manifest_dir.join("plugin.json"), "{").expect("invalid manifest");
    let options = PluginDiscoveryOptions::new(workspace_dir, config_dir, PluginsConfig::default());

    let err = discover_plugins(&options).expect_err("invalid manifest rejected").to_string();

    assert!(err.contains("failed to load plugin"), "{err}");
    assert!(err.contains("broken-plugin"), "{err}");
}
