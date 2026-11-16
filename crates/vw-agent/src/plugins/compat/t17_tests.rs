//! Package fixtures shared by the remaining real host OAuth checks.
use crate::plugins::PluginDiscoveryOptions;
use config_plugins::PluginsConfig;
use serde_json::{Value, json};
use std::{fs, path::{Path, PathBuf}};

pub(super) fn write(path: &Path, value: Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}
pub(super) fn options(temp: &Path) -> PluginDiscoveryOptions {
    PluginDiscoveryOptions::new(
        temp.join("workspace"),
        temp.join("config"),
        PluginsConfig::default(),
    )
}
pub(super) fn plugin(options: &PluginDiscoveryOptions, name: &str, server: Value) -> PathBuf {
    let root = options.workspace_dir.join(".vibewindow/plugins").join(name);
    write(
        &root.join(".codex-plugin/plugin.json"),
        json!({"name":name,"mcpServers":{"shared":server}}),
    );
    root
}
