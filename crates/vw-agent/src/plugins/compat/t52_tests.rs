use crate::plugins::{
    activate_plugin_upgrade, install_plugin_from_marketplace, load_plugin_manifest,
    prepare_plugin_upgrade,
};
use std::{fs, path::Path};
fn package(root: &Path, version: &str) {
    fs::create_dir_all(root.join(".codex-plugin")).unwrap();
    fs::write(
        root.join(".codex-plugin/plugin.json"),
        format!(r#"{{"name":"fixture","version":"{version}"}}"#),
    )
    .unwrap();
}
#[test]
fn codex_plugin_compat_t52_restores_old_version_after_failed_upgrade() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let plugins = temp.path().join("plugins");
    package(&source, "1");
    let old = install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "alpha").unwrap();
    package(&source, "2");
    let preview =
        prepare_plugin_upgrade(source.to_str().unwrap(), &plugins, "fixture@alpha").unwrap();
    assert_eq!(preview.old_version.as_deref(), Some("1"));
    assert_eq!(preview.new_version.as_deref(), Some("2"));
    assert_eq!(load_plugin_manifest(&old.installed_dir).unwrap().version.as_deref(), Some("1"));
    // A changed installed package invalidates the preview without overwriting it.
    fs::write(old.installed_dir.join("changed"), b"changed after preview").unwrap();
    assert!(activate_plugin_upgrade(preview).is_err());
    assert_eq!(load_plugin_manifest(&old.installed_dir).unwrap().version.as_deref(), Some("1"));
    assert_eq!(fs::read(old.installed_dir.join("changed")).unwrap(), b"changed after preview");
}
#[test]
fn codex_plugin_compat_t52_rolls_back_after_partial_publication() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let plugins = temp.path().join("plugins");
    package(&source, "1");
    install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "alpha").unwrap();
    package(&source, "2");
    let preview =
        prepare_plugin_upgrade(source.to_str().unwrap(), &plugins, "fixture@alpha").unwrap();
    super::upgrade::FAIL_ACTIVATION.with(|flag| flag.set(true));
    assert!(activate_plugin_upgrade(preview).unwrap_err().to_string().contains("injected"));
    assert_eq!(
        load_plugin_manifest(&plugins.join("fixture@alpha")).unwrap().version.as_deref(),
        Some("1")
    );
    assert!(!plugins.join(".plugin-upgrade.json").exists());
}
#[test]
fn codex_plugin_compat_t52_recovers_journaled_interruption() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let plugins = temp.path().join("plugins");
    package(&source, "1");
    let installed =
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "alpha").unwrap();
    let transaction = plugins.join(".plugin-upgrade-interrupted");
    fs::create_dir_all(transaction.join("old")).unwrap();
    super::installed_state::write_journal(
        &plugins,
        &super::installed_state::Journal {
            key: "fixture@alpha".into(),
            transaction: ".plugin-upgrade-interrupted".into(),
            committed: false,
        },
        true,
    )
    .unwrap();
    fs::rename(&installed.installed_dir, transaction.join("old/fixture@alpha")).unwrap();
    // Restart path: taking the store lock performs recovery before discovery can read.
    let _guard = super::installed_state::lock(&plugins).unwrap();
    assert_eq!(
        load_plugin_manifest(&installed.installed_dir).unwrap().version.as_deref(),
        Some("1")
    );
    assert!(!transaction.exists());
    assert!(!plugins.join(".plugin-upgrade.json").exists());
}

#[test]
fn codex_plugin_compat_t52_rejects_invalid_input() {
    let temp = tempfile::tempdir().unwrap();
    assert!(prepare_plugin_upgrade("/missing", temp.path(), "../escape").is_err());
    assert!(prepare_plugin_upgrade("/missing", temp.path(), "missing@alpha").is_err());
}
#[test]
fn codex_plugin_compat_t52_preserves_isolation() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let plugins = temp.path().join("plugins");
    package(&source, "1");
    for market in ["alpha", "beta"] {
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, market).unwrap();
    }
    crate::plugins::install::set_plugin_enabled(&plugins, "fixture@alpha", false).unwrap();
    let data = temp.path().join("plugin-data");
    fs::create_dir(&data).unwrap();
    fs::write(data.join("retained"), b"state").unwrap();
    let hooks: crate::plugins::CodexHooksManifest =
        serde_json::from_value(serde_json::json!({"hooks":{}})).unwrap();
    let mut trust =
        crate::plugins::PluginHookTrustStore::load_or_default(&temp.path().join("trust.json"))
            .unwrap();
    trust.trust_codex_hooks("fixture@alpha", &plugins.join("fixture@alpha"), &hooks).unwrap();
    assert!(
        trust.codex_hooks_trusted("fixture@alpha", &plugins.join("fixture@alpha"), &hooks).unwrap()
    );
    package(&source, "2");
    let preview =
        prepare_plugin_upgrade(source.to_str().unwrap(), &plugins, "fixture@alpha").unwrap();
    activate_plugin_upgrade(preview).unwrap();
    assert_eq!(fs::read(data.join("retained")).unwrap(), b"state");
    assert!(
        !trust
            .codex_hooks_trusted("fixture@alpha", &plugins.join("fixture@alpha"), &hooks)
            .unwrap()
    );

    assert_eq!(
        load_plugin_manifest(&plugins.join("fixture@alpha")).unwrap().version.as_deref(),
        Some("2")
    );
    assert_eq!(
        load_plugin_manifest(&plugins.join("fixture@beta")).unwrap().version.as_deref(),
        Some("1")
    );
    assert!(plugins.join("fixture@alpha.disabled").exists());
    assert_eq!(
        super::identity::read_install_metadata(
            &plugins.join("fixture@alpha"),
            &load_plugin_manifest(&plugins.join("fixture@alpha")).unwrap()
        )
        .unwrap()
        .unwrap()
        .version
        .as_deref(),
        Some("2")
    );
}

#[test]
fn codex_plugin_compat_t52_concurrent_discovery_sees_complete_versions() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let plugins = temp.path().join("plugins");
    package(&source, "1");
    install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "alpha").unwrap();
    package(&source, "2");
    let preview =
        prepare_plugin_upgrade(source.to_str().unwrap(), &plugins, "fixture@alpha").unwrap();
    let root = temp.path().to_path_buf();
    let reader = std::thread::spawn(move || {
        for _ in 0..25 {
            let discovered =
                crate::plugins::discover_plugins(&crate::plugins::PluginDiscoveryOptions::new(
                    root.join("workspace"),
                    &root,
                    Default::default(),
                ))
                .unwrap();
            assert!(discovered.diagnostics.is_empty());
            assert_eq!(discovered.installed.len(), 1);
            let plugin = &discovered.installed[0];
            assert_eq!(plugin.install_metadata.as_ref().unwrap().version, plugin.manifest.version);
        }
    });
    activate_plugin_upgrade(preview).unwrap();
    reader.join().unwrap();
}
