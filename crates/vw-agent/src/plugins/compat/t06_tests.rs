//! 真实安装、发现与生命周期入口的来源身份验收。

use super::identity::metadata_path;
use crate::plugins::install::{
    install_plugin_from_marketplace, install_plugin_from_source, remove_installed_plugin,
    set_plugin_enabled,
};
use crate::plugins::{PluginDiscoveryOptions, PluginIdentity, discover_plugins};
use std::fs;
use std::path::Path;
use vw_config_types::plugins::PluginsConfig;

const PACKAGE: &[u8] = include_bytes!("../../../testdata/plugin_compat/t06/plugin.json");

fn source(root: &Path) -> &str {
    fs::create_dir_all(root.join(".vibewindow-plugin")).unwrap();
    fs::write(root.join(".vibewindow-plugin/plugin.json"), PACKAGE).unwrap();
    root.to_str().unwrap()
}

#[test]
fn codex_plugin_compat_t06_separates_same_name_marketplaces() {
    let temp = tempfile::tempdir().unwrap();
    let package = temp.path().join("package");
    let config = temp.path().join("config");
    let plugins = config.join("plugins");
    let first = install_plugin_from_marketplace(source(&package), &plugins, "alpha").unwrap();
    let second = install_plugin_from_marketplace(package.to_str().unwrap(), &plugins, "beta")
        .expect("same plugin name in another marketplace must install independently");
    assert_ne!(first.installed_dir, second.installed_dir);
    assert_eq!(first.installed_dir, plugins.join("sample.plugin@alpha"));
    assert_eq!(second.installed_dir, plugins.join("sample.plugin@beta"));
    assert_eq!(first.identity.as_key(), "sample.plugin@alpha");
    assert_eq!(second.identity.as_key(), "sample.plugin@beta");
    let discovery = discover_plugins(&PluginDiscoveryOptions::new(
        temp.path().join("workspace"),
        config,
        PluginsConfig::default(),
    ))
    .unwrap();
    assert_eq!(discovery.installed.len(), 2, "name-only deduplication loses marketplace identity");
    assert_eq!(discovery.enabled.len(), 2);
    for plugin in &discovery.installed {
        let metadata = plugin.install_metadata.as_ref().expect("persisted provenance");
        assert_eq!(metadata.identity, plugin.identity);
        assert_eq!(metadata.source, package.canonicalize().unwrap().to_string_lossy());
        assert_eq!(metadata.version.as_deref(), Some("1.2.3"));
        assert_eq!(
            fs::read(plugin.manifest.root.join(".vibewindow-plugin/plugin.json")).unwrap(),
            PACKAGE
        );
        assert!(!plugin.manifest.root.join("sample.plugin@alpha.install.json").exists());
    }
}

#[test]
fn codex_plugin_compat_t06_rejects_invalid_input() {
    let temp = tempfile::tempdir().unwrap();
    let package = temp.path().join("package");
    let plugins = temp.path().join("plugins");
    let error = install_plugin_from_marketplace(source(&package), &plugins, "../outside")
        .expect_err("invalid marketplace must be rejected before copying the package");
    assert!(error.to_string().contains("marketplace"));
    assert!(!plugins.exists());
    assert_eq!(fs::read(package.join(".vibewindow-plugin/plugin.json")).unwrap(), PACKAGE);
}

#[test]
fn codex_plugin_compat_t06_preserves_isolation() {
    let temp = tempfile::tempdir().unwrap();
    let package = temp.path().join("package");
    let config = temp.path().join("config");
    let plugins = config.join("plugins");
    let installed = install_plugin_from_marketplace(source(&package), &plugins, "alpha").unwrap();
    set_plugin_enabled(&plugins, "sample.plugin@alpha", false)
        .expect("lifecycle must resolve the exact marketplace identity");
    let local = install_plugin_from_source(package.to_str().unwrap(), &plugins).unwrap();
    assert_eq!(local.installed_dir, plugins.join("sample.plugin"));
    let options = PluginDiscoveryOptions::new(
        temp.path().join("workspace"),
        config,
        PluginsConfig::default(),
    );
    let discovery = discover_plugins(&options).unwrap();
    assert_eq!(discovery.installed.len(), 2);
    assert_eq!(discovery.enabled.len(), 1);
    assert_eq!(discovery.enabled[0].manifest.root, local.manifest.root);
    remove_installed_plugin(&plugins, "sample.plugin@alpha").unwrap();
    assert!(!installed.installed_dir.exists());
    assert!(!metadata_path(&installed.installed_dir).unwrap().exists());
    assert!(!plugins.join("sample.plugin@alpha.disabled").exists());
    assert!(local.installed_dir.is_dir());
    assert_eq!(fs::read(package.join(".vibewindow-plugin/plugin.json")).unwrap(), PACKAGE);
    assert_eq!(
        fs::read(local.installed_dir.join(".vibewindow-plugin/plugin.json")).unwrap(),
        PACKAGE
    );
}

#[test]
fn codex_plugin_compat_t06_validates_both_identity_segments() {
    for name in
        ["", ".", "..", ".sample", "sample.", "sample..other", "a/b", "a\\b", "a@b", "插件", " a"]
    {
        assert!(PluginIdentity::marketplace(name, "alpha").is_err(), "{name:?}");
    }
    for market in ["", ".", "..", "a.b", "../outside", "a/b", "a\\b", "a@b", "市场", " a"] {
        let temp = tempfile::tempdir().unwrap();
        let package = temp.path().join("package");
        let plugins = temp.path().join("plugins");
        assert!(
            install_plugin_from_marketplace(source(&package), &plugins, market).is_err(),
            "{market:?}"
        );
        assert!(!plugins.exists());
    }
    for key in ["sample", "@alpha", "sample@", "a@b@alpha", "a@alpha.beta"] {
        assert!(PluginIdentity::parse_marketplace(key).is_err(), "{key:?}");
    }
    let identity = PluginIdentity::parse_marketplace("Sample.one-two_3@Market_1").unwrap();
    assert_eq!(identity.name(), "Sample.one-two_3");
    assert_eq!(identity.marketplace_name(), Some("Market_1"));
    assert_eq!(identity.as_key(), "Sample.one-two_3@Market_1");
}

#[test]
fn codex_plugin_compat_t06_filters_exact_identities_without_local_aliases() {
    let temp = tempfile::tempdir().unwrap();
    let package = temp.path().join("package");
    let config = temp.path().join("config");
    let plugins = config.join("plugins");
    install_plugin_from_marketplace(source(&package), &plugins, "alpha").unwrap();
    install_plugin_from_marketplace(package.to_str().unwrap(), &plugins, "beta").unwrap();
    install_plugin_from_source(package.to_str().unwrap(), &plugins).unwrap();
    for wanted in ["sample.plugin@alpha", "sample.plugin@beta", "sample.plugin"] {
        let options = PluginDiscoveryOptions::new(
            temp.path().join("workspace"),
            &config,
            PluginsConfig { enabled: vec![wanted.to_owned()], sources: vec![] },
        );
        let found = discover_plugins(&options).unwrap();
        assert_eq!(found.installed.len(), 3);
        assert_eq!(found.enabled.len(), 1);
        assert_eq!(found.enabled[0].identity.as_key(), wanted);
    }
    set_plugin_enabled(&plugins, "sample.plugin@alpha", false).unwrap();
    set_plugin_enabled(&plugins, "sample.plugin@alpha", false).unwrap();
    set_plugin_enabled(&plugins, "sample.plugin@alpha", true).unwrap();
    let found = discover_plugins(&PluginDiscoveryOptions::new(
        temp.path().join("workspace"),
        config,
        PluginsConfig::default(),
    ))
    .unwrap();
    assert_eq!(found.enabled.len(), 3);
}

#[test]
fn codex_plugin_compat_t06_rejects_mismatched_or_missing_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let package = temp.path().join("package");
    let config = temp.path().join("config");
    let plugins = config.join("plugins");
    let installed = install_plugin_from_marketplace(source(&package), &plugins, "alpha").unwrap();
    let sidecar = metadata_path(&installed.installed_dir).unwrap();
    let bytes = fs::read(&sidecar).unwrap();
    let options = PluginDiscoveryOptions::new(
        temp.path().join("workspace"),
        &config,
        PluginsConfig::default(),
    );
    for (field, value) in
        [("name", "other"), ("marketplace", "beta"), ("version", "9.0.0"), ("source", " ")]
    {
        let mut record: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        record[field] = value.into();
        fs::write(&sidecar, serde_json::to_vec(&record).unwrap()).unwrap();
        let rejected = discover_plugins(&options).expect("bad package state is isolated");
        assert!(rejected.installed.is_empty(), "{field}");
        assert!(rejected.enabled.is_empty(), "{field}");
        assert!(
            rejected.diagnostics.iter().any(|d| d.plugin_root == installed.installed_dir),
            "{field}"
        );
        assert!(set_plugin_enabled(&plugins, "sample.plugin@alpha", false).is_err());
        assert!(remove_installed_plugin(&plugins, "sample.plugin@alpha").is_err());
        assert!(installed.installed_dir.is_dir());
        assert!(!plugins.join("sample.plugin@alpha.disabled").exists());
    }
    fs::remove_file(&sidecar).unwrap();
    assert!(discover_plugins(&options).unwrap().diagnostics.iter().any(|d| {
        d.plugin_root == installed.installed_dir
            && d.message.contains("missing installation metadata")
    }));
    assert!(remove_installed_plugin(&plugins, "sample.plugin@alpha").is_err());
    fs::write(&sidecar, bytes).unwrap();
    assert_eq!(discover_plugins(&options).unwrap().enabled.len(), 1);
    assert_eq!(
        fs::read(installed.installed_dir.join(".vibewindow-plugin/plugin.json")).unwrap(),
        PACKAGE
    );
}

#[test]
fn codex_plugin_compat_t06_preserves_existing_local_identity_spelling() {
    let temp = tempfile::tempdir().unwrap();
    let package = temp.path().join("package");
    let config = temp.path().join("config");
    let plugins = config.join("plugins");
    source(&package);
    let manifest = package.join(".vibewindow-plugin/plugin.json");
    fs::write(
        &manifest,
        br#"{"name":"existing@local","version":"1","description":"local fixture"}"#,
    )
    .unwrap();
    let installed = install_plugin_from_source(package.to_str().unwrap(), &plugins).unwrap();
    assert_eq!(installed.identity.as_key(), "existing@local");
    assert_eq!(installed.identity.marketplace_name(), None);
    fs::remove_file(metadata_path(&installed.installed_dir).unwrap()).unwrap();
    let found = discover_plugins(&PluginDiscoveryOptions::new(
        temp.path().join("workspace"),
        config,
        PluginsConfig::default(),
    ))
    .unwrap();
    assert_eq!(found.installed[0].identity.as_key(), "existing@local");
    assert_eq!(found.installed[0].identity.marketplace_name(), None);
    set_plugin_enabled(&plugins, "existing@local", false).unwrap();
    remove_installed_plugin(&plugins, "existing@local").unwrap();
    assert!(!installed.installed_dir.exists());
}

#[cfg(unix)]
#[test]
fn codex_plugin_compat_t06_rejects_symlink_state_without_touching_other_packages() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let package = temp.path().join("package");
    let config = temp.path().join("config");
    let plugins = config.join("plugins");
    let installed = install_plugin_from_marketplace(source(&package), &plugins, "alpha").unwrap();
    let outside = temp.path().join("outside");
    fs::write(&outside, "unchanged").unwrap();
    let marker = plugins.join("sample.plugin@alpha.disabled");
    symlink(&outside, &marker).unwrap();
    assert!(set_plugin_enabled(&plugins, "sample.plugin@alpha", false).is_err());
    assert_eq!(fs::read_to_string(&outside).unwrap(), "unchanged");
    fs::remove_file(&marker).unwrap();
    let sidecar = metadata_path(&installed.installed_dir).unwrap();
    fs::remove_file(&sidecar).unwrap();
    symlink(&outside, &sidecar).unwrap();
    let options = PluginDiscoveryOptions::new(
        temp.path().join("workspace"),
        &config,
        PluginsConfig::default(),
    );
    let rejected = discover_plugins(&options).unwrap();
    assert!(rejected.installed.is_empty());
    assert!(rejected.enabled.is_empty());
    assert!(rejected.diagnostics.iter().any(|d| d.plugin_root == installed.installed_dir));
    assert!(remove_installed_plugin(&plugins, "sample.plugin@alpha").is_err());
    assert!(installed.installed_dir.is_dir());
    assert_eq!(fs::read_to_string(&outside).unwrap(), "unchanged");
    let reserved = plugins.join("sample.plugin@beta.install.json");
    symlink(&outside, &reserved).unwrap();
    assert!(install_plugin_from_marketplace(package.to_str().unwrap(), &plugins, "beta").is_err());
    assert!(!plugins.join("sample.plugin@beta").exists());
    assert_eq!(fs::read_to_string(&outside).unwrap(), "unchanged");
}

#[test]
fn codex_plugin_compat_t06_rejects_local_marketplace_key_collisions_across_sources() {
    let temp = tempfile::tempdir().unwrap();
    let package = temp.path().join("package");
    let config = temp.path().join("config");
    let workspace = temp.path().join("workspace");
    install_plugin_from_marketplace(source(&package), &config.join("plugins"), "alpha").unwrap();
    let local = workspace.join(".vibewindow/plugins/local");
    source(&local);
    fs::write(
        local.join(".vibewindow-plugin/plugin.json"),
        br#"{"name":"sample.plugin@alpha","version":"1","description":"local collision fixture"}"#,
    )
    .unwrap();
    let error =
        discover_plugins(&PluginDiscoveryOptions::new(workspace, config, PluginsConfig::default()))
            .unwrap_err();
    assert!(error.to_string().contains("ambiguous local and marketplace plugin identity"));
    assert_eq!(fs::read(package.join(".vibewindow-plugin/plugin.json")).unwrap(), PACKAGE);
}
