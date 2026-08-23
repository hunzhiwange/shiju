//! 验证插件命令行的安装、审计、启停、列举与删除流程。

use super::{PluginCommands, PluginDiscoveryOptions, discover_plugins, handle_command};
use crate::config::Config;
use serde_json::json;
use std::fs;
use std::path::Path;
use vw_config_types::plugins::PluginsConfig;

// 临时工作区与配置目录必须分离，避免命令测试读写真实用户配置。
fn config_for(dir: &tempfile::TempDir) -> Config {
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    fs::create_dir_all(&workspace_dir).expect("workspace dir");
    fs::create_dir_all(&config_dir).expect("config dir");

    let mut config = Config::default();
    config.workspace_dir = workspace_dir;
    config.config_path = config_dir.join("vibewindow.json");
    config
}

fn write_plugin(plugin_root: &Path, name: &str) {
    fs::create_dir_all(plugin_root.join(".vibewindow-plugin")).expect("manifest dir");
    fs::create_dir_all(plugin_root.join("skills")).expect("skills dir");
    fs::write(
        plugin_root.join(".mcp.json"),
        serde_json::to_vec_pretty(&json!([{ "name": "sample", "command": "sample-mcp" }]))
            .expect("mcp json"),
    )
    .expect("write mcp");
    fs::write(
        plugin_root.join(".vibewindow-plugin/plugin.json"),
        serde_json::to_vec_pretty(&json!({
            "name": name,
            "version": "0.1.0",
            "description": format!("{name} test plugin"),
            "skills": "./skills",
            "mcpServers": "./.mcp.json"
        }))
        .expect("manifest json"),
    )
    .expect("write manifest");
}

fn config_plugin_dir(config: &Config) -> std::path::PathBuf {
    config.config_path.parent().expect("config parent").join("plugins")
}

#[test]
// 安装成功用重新发现结果收口，确保复制结果也能被运行时加载。
fn plugin_cli_install_copies_local_plugin_after_audit() {
    let dir = tempfile::tempdir().expect("temp dir");
    let config = config_for(&dir);
    let source = dir.path().join("source-package");
    write_plugin(&source, "copied-plugin");

    handle_command(
        PluginCommands::Install { source: source.to_string_lossy().to_string() },
        &config,
    )
    .expect("install local plugin");

    let installed = config_plugin_dir(&config).join("copied-plugin");
    assert!(installed.join(".vibewindow-plugin/plugin.json").is_file());
    assert!(installed.join("skills").is_dir());
    assert!(!config_plugin_dir(&config).join("copied-plugin.disabled").exists());

    let discovery = discover_plugins(&PluginDiscoveryOptions::new(
        config.workspace_dir.clone(),
        config.config_path.parent().expect("config parent"),
        PluginsConfig::default(),
    ))
    .expect("discover installed plugin");
    assert_eq!(discovery.installed.len(), 1);
    assert_eq!(discovery.enabled.len(), 1);
    assert_eq!(discovery.installed[0].manifest.name, "copied-plugin");
}

#[test]
fn plugin_cli_enable_disable_toggles_marker() {
    let dir = tempfile::tempdir().expect("temp dir");
    let config = config_for(&dir);
    let installed = config_plugin_dir(&config).join("toggle-plugin");
    write_plugin(&installed, "toggle-plugin");

    handle_command(PluginCommands::Disable { name: "toggle-plugin".to_string() }, &config)
        .expect("disable plugin");
    let marker = config_plugin_dir(&config).join("toggle-plugin.disabled");
    assert!(marker.is_file());

    handle_command(PluginCommands::Enable { name: "toggle-plugin".to_string() }, &config)
        .expect("enable plugin");
    assert!(!marker.exists());
}

#[test]
fn plugin_cli_install_rejects_remote_source_for_now() {
    let dir = tempfile::tempdir().expect("temp dir");
    let config = config_for(&dir);

    let error = handle_command(
        PluginCommands::Install { source: "https://example.com/plugin".to_string() },
        &config,
    )
    .expect_err("remote source should be rejected");

    assert!(error.to_string().contains("not supported"));
    assert!(!config_plugin_dir(&config).exists());
}

#[test]
fn plugin_cli_list_accepts_empty_installation() {
    let dir = tempfile::tempdir().expect("temp dir");
    let config = config_for(&dir);

    handle_command(PluginCommands::List, &config).expect("list empty installation");
    assert!(!config_plugin_dir(&config).exists());
}

#[test]
fn plugin_cli_list_accepts_discovered_plugin() {
    let dir = tempfile::tempdir().expect("temp dir");
    let config = config_for(&dir);
    write_plugin(&config_plugin_dir(&config).join("listed-plugin"), "listed-plugin");

    handle_command(PluginCommands::List, &config).expect("list installed plugin");

    let discovery = discover_plugins(&PluginDiscoveryOptions::new(
        config.workspace_dir.clone(),
        config.config_path.parent().expect("config parent"),
        PluginsConfig::default(),
    ))
    .expect("discover listed plugin");
    assert_eq!(discovery.installed.len(), 1);
    assert_eq!(discovery.installed[0].manifest.name, "listed-plugin");
}

#[test]
fn plugin_cli_audit_rejects_blank_source() {
    let dir = tempfile::tempdir().expect("temp dir");
    let config = config_for(&dir);

    let error = handle_command(PluginCommands::Audit { source: " \t ".to_string() }, &config)
        .expect_err("blank audit source should fail");

    assert!(error.to_string().contains("cannot be empty"));
}

#[test]
fn plugin_cli_audit_accepts_direct_source_and_installed_name() {
    let dir = tempfile::tempdir().expect("temp dir");
    let config = config_for(&dir);
    let direct = dir.path().join("direct-plugin");
    let installed = config_plugin_dir(&config).join("installed-plugin");
    write_plugin(&direct, "direct-plugin");
    write_plugin(&installed, "installed-plugin");

    handle_command(PluginCommands::Audit { source: direct.to_string_lossy().to_string() }, &config)
        .expect("audit direct plugin source");
    handle_command(PluginCommands::Audit { source: "installed-plugin".to_string() }, &config)
        .expect("audit installed plugin by name");
}

#[test]
fn plugin_cli_audit_adds_target_context_for_missing_source() {
    let dir = tempfile::tempdir().expect("temp dir");
    let config = config_for(&dir);

    let error =
        handle_command(PluginCommands::Audit { source: "missing-plugin".to_string() }, &config)
            .expect_err("missing plugin audit should fail");

    assert!(error.to_string().contains("Plugin audit failed"));
    assert!(error.to_string().contains("missing-plugin"));
}

#[test]
fn plugin_cli_remove_deletes_plugin_and_disabled_marker() {
    let dir = tempfile::tempdir().expect("temp dir");
    let config = config_for(&dir);
    let plugin = config_plugin_dir(&config).join("removed-plugin");
    write_plugin(&plugin, "removed-plugin");

    handle_command(PluginCommands::Disable { name: "removed-plugin".to_string() }, &config)
        .expect("disable plugin before removal");
    handle_command(PluginCommands::Remove { name: "removed-plugin".to_string() }, &config)
        .expect("remove plugin");

    assert!(!plugin.exists());
    assert!(!config_plugin_dir(&config).join("removed-plugin.disabled").exists());
}
