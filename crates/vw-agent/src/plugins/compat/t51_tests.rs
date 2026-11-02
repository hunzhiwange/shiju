//! Acceptance of existing persisted identity and lifecycle behavior; no production changes.
use crate::plugins::install::{remove_installed_plugin, set_plugin_enabled};
use crate::plugins::{PluginDiscoveryOptions, discover_plugins, install_plugin_from_marketplace};
use std::{fs, path::Path};
use vw_config_types::plugins::PluginsConfig;
const MANIFEST: &[u8] = br#"{"name":"fixture","version":"1.2.3"}"#;
fn package(root: &Path) {
    fs::create_dir_all(root.join(".codex-plugin")).unwrap();
    fs::write(root.join(".codex-plugin/plugin.json"), MANIFEST).unwrap();
}
#[test]
fn codex_plugin_compat_t51_restores_exact_installed_identity() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    package(&source);
    let config = temp.path().join("config");
    let plugins = config.join("plugins");
    for market in ["alpha", "beta"] {
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, market).unwrap();
    }
    set_plugin_enabled(&plugins, "fixture@alpha", false).unwrap();
    for _ in 0..2 {
        let restored = discover_plugins(&PluginDiscoveryOptions::new(
            temp.path().join("workspace"),
            &config,
            PluginsConfig::default(),
        ))
        .unwrap();
        assert_eq!(restored.installed.len(), 2);
        assert_eq!(restored.enabled.len(), 1);
        assert_eq!(restored.enabled[0].identity.as_key(), "fixture@beta");
        for p in restored.installed {
            let record = p.install_metadata.unwrap();
            assert_eq!(record.identity, p.identity);
            assert_eq!(record.version.as_deref(), Some("1.2.3"));
            assert_eq!(record.source, source.canonicalize().unwrap().to_string_lossy());
            assert_eq!(
                fs::read(p.manifest.root.join(".codex-plugin/plugin.json")).unwrap(),
                MANIFEST
            );
        }
    }
}
#[test]
fn codex_plugin_compat_t51_rejects_invalid_input() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    package(&source);
    let config = temp.path().join("config");
    let plugins = config.join("plugins");
    let installed =
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "alpha").unwrap();
    fs::write(super::identity::metadata_path(&installed.installed_dir).unwrap(), b"{broken")
        .unwrap();
    let restored = discover_plugins(&PluginDiscoveryOptions::new(
        temp.path().join("workspace"),
        config,
        PluginsConfig::default(),
    ))
    .unwrap();
    assert!(restored.installed.is_empty());
    assert!(!restored.diagnostics.is_empty());
    assert!(remove_installed_plugin(&plugins, "fixture@alpha").is_err());
    assert!(installed.installed_dir.exists());
}
#[test]
fn codex_plugin_compat_t51_preserves_isolation() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    package(&source);
    let config = temp.path().join("config");
    let global = config.join("plugins");
    let workspace = temp.path().join("workspace");
    let local = workspace.join("plugins");
    for (root, market) in [(&global, "alpha"), (&global, "beta"), (&local, "alpha")] {
        install_plugin_from_marketplace(source.to_str().unwrap(), root, market).unwrap();
    }
    set_plugin_enabled(&local, "fixture@alpha", false).unwrap();
    let options = PluginDiscoveryOptions::new(workspace, config, PluginsConfig::default());
    let restored = discover_plugins(&options).unwrap();
    assert_eq!(restored.installed.len(), 2);
    assert_eq!(restored.enabled.len(), 1);
    assert_eq!(restored.enabled[0].identity.as_key(), "fixture@beta");
    let data = temp.path().join("plugin-data/fixture@alpha");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("state"), b"retained").unwrap();
    remove_installed_plugin(&local, "fixture@alpha").unwrap();
    assert_eq!(fs::read(data.join("state")).unwrap(), b"retained");
    assert!(global.join("fixture@alpha").exists());
    assert!(global.join("fixture@beta").exists());
    assert_eq!(discover_plugins(&options).unwrap().enabled.len(), 2);
}
