//! Real installation checkpoints inspect visible files, rejection and competing destinations.

use super::identity::metadata_path;
use crate::plugins::install::{
    install_plugin_from_marketplace, install_plugin_from_source,
    set_next_post_copy_observer_for_test,
};
use crate::plugins::{
    PluginDiscoveryOptions, PluginHookSource, PluginMcpServers, discover_plugins,
    load_plugin_manifest,
};
use std::fs;
use std::path::{Path, PathBuf};

const MANIFEST: &[u8] = include_bytes!("../../../testdata/plugin_compat/t08/plugin.json");
const PAYLOAD: &[u8] = include_bytes!("../../../testdata/plugin_compat/t08/payload.txt");

fn package(root: &Path) -> &str {
    fs::create_dir_all(root.join(".codex-plugin")).unwrap();
    fs::write(root.join(".codex-plugin/plugin.json"), MANIFEST).unwrap();
    fs::write(root.join("payload.txt"), PAYLOAD).unwrap();
    root.to_str().unwrap()
}

fn children(root: &Path) -> Vec<PathBuf> {
    let mut entries =
        fs::read_dir(root).unwrap().map(|entry| entry.unwrap().path()).collect::<Vec<_>>();
    entries.sort();
    entries
}

#[test]
fn codex_plugin_compat_t08_publishes_only_complete_package() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let plugins = temp.path().join("plugins");
    let destination = plugins.join("transaction-plugin");
    let final_path = destination.clone();
    set_next_post_copy_observer_for_test(move |copied| {
        assert_eq!(fs::read(copied.join("payload.txt")).unwrap(), PAYLOAD);
        assert!(!final_path.exists(), "unaudited package must not be published at its final path");
    });
    let installed = install_plugin_from_source(package(&source), &plugins).unwrap();
    assert_eq!(installed.installed_dir, destination);
    assert_eq!(installed.manifest.root, destination.canonicalize().unwrap());
    assert_eq!(fs::read(destination.join(".codex-plugin/plugin.json")).unwrap(), MANIFEST);
    assert_eq!(fs::read(destination.join("payload.txt")).unwrap(), PAYLOAD);
    assert_eq!(fs::read(source.join(".codex-plugin/plugin.json")).unwrap(), MANIFEST);
    assert_eq!(children(&plugins).len(), 2, "only the complete package and its sidecar remain");
}

#[test]
fn codex_plugin_compat_t08_rejects_invalid_input() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let plugins = temp.path().join("plugins");
    let destination = plugins.join("transaction-plugin");
    let final_path = destination.clone();
    set_next_post_copy_observer_for_test(move |copied| {
        assert!(!final_path.exists(), "a rejected copy must never enter the installed namespace");
        fs::write(copied.join(".codex-plugin/plugin.json"), b"invalid JSON").unwrap();
    });
    let error = install_plugin_from_source(package(&source), &plugins).unwrap_err();
    assert!(format!("{error:#}").contains("audit") || format!("{error:#}").contains("changed"));
    assert!(!destination.exists());
    assert!(!metadata_path(&destination).unwrap().exists());
    assert!(children(&plugins).is_empty(), "failed copy must leave no transaction entries");
    assert_eq!(fs::read(source.join(".codex-plugin/plugin.json")).unwrap(), MANIFEST);
}

#[test]
fn codex_plugin_compat_t08_preserves_isolation() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let plugins = temp.path().join("plugins");
    let destination = plugins.join("transaction-plugin@alpha");
    let final_path = destination.clone();
    set_next_post_copy_observer_for_test(move |_| {
        assert!(!final_path.exists(), "copying must not occupy another installer's destination");
        fs::create_dir(&final_path).unwrap();
    });
    let error = install_plugin_from_marketplace(package(&source), &plugins, "alpha").unwrap_err();
    assert!(format!("{error:#}").contains("exist"), "{error:#}");
    assert!(destination.is_dir(), "the competing destination must survive");
    assert!(children(&destination).is_empty(), "even an empty competing directory is not replaced");
    assert!(!metadata_path(&destination).unwrap().exists());
    assert_eq!(children(&plugins), vec![destination]);
    assert_eq!(fs::read(source.join(".codex-plugin/plugin.json")).unwrap(), MANIFEST);
}

#[test]
fn codex_plugin_compat_t08_preserves_existing_metadata_and_packages() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let plugins = temp.path().join("plugins");
    fs::create_dir(&plugins).unwrap();
    let destination = plugins.join("transaction-plugin");
    let metadata = metadata_path(&destination).unwrap();
    fs::write(&metadata, b"unrelated installation record").unwrap();
    let error = install_plugin_from_source(package(&source), &plugins).unwrap_err();
    assert!(error.to_string().contains("metadata"));
    assert!(error.to_string().contains("exists"));
    assert!(!destination.exists());
    assert_eq!(fs::read(&metadata).unwrap(), b"unrelated installation record");
    assert_eq!(children(&plugins), vec![metadata.clone()]);
    fs::remove_file(&metadata).unwrap();
    fs::create_dir(&destination).unwrap();
    let error = install_plugin_from_source(source.to_str().unwrap(), &plugins).unwrap_err();
    assert!(error.to_string().contains("already exists"));
    assert_eq!(children(&plugins), vec![destination.clone()]);
    assert!(children(&destination).is_empty());
}

#[test]
fn codex_plugin_compat_t08_rejects_changed_manifest_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let plugins = temp.path().join("plugins");
    set_next_post_copy_observer_for_test(|staged| {
        let mut value: serde_json::Value = serde_json::from_slice(MANIFEST).unwrap();
        value["description"] = "changed description".into();
        fs::write(staged.join(".codex-plugin/plugin.json"), serde_json::to_vec(&value).unwrap())
            .unwrap();
    });
    let error = install_plugin_from_source(package(&source), &plugins).unwrap_err();
    assert!(error.to_string().contains("manifest contents changed"), "{error:#}");
    assert!(children(&plugins).is_empty());
    assert_eq!(fs::read(source.join(".codex-plugin/plugin.json")).unwrap(), MANIFEST);
}

#[test]
fn codex_plugin_compat_t08_rejects_changed_manifest_selection() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let plugins = temp.path().join("plugins");
    set_next_post_copy_observer_for_test(|staged| {
        fs::write(
            staged.join("plugin.json"),
            serde_json::to_vec(&serde_json::json!({
                "$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
                "name": "transaction-plugin",
                "version": "1.0.0"
            }))
            .unwrap(),
        )
        .unwrap();
    });
    let error = install_plugin_from_source(package(&source), &plugins).unwrap_err();
    assert!(error.to_string().contains("selected manifest changed"), "{error:#}");
    assert!(children(&plugins).is_empty());
    assert!(!source.join("plugin.json").exists());
}

#[test]
fn codex_plugin_compat_t08_rebases_audited_paths_and_preserves_provenance() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let config = temp.path().join("config");
    let plugins = config.join("plugins");
    package(&source);
    fs::create_dir(source.join("skills")).unwrap();
    fs::write(source.join("skills/SKILL.md"), "# Fixture skill\n").unwrap();
    for file in ["mcp.json", "hooks.json", "apps.json", "icon.svg"] {
        fs::write(source.join(file), b"{}").unwrap();
    }
    let mut value: serde_json::Value = serde_json::from_slice(MANIFEST).unwrap();
    value["skills"] = "./skills".into();
    value["mcpServers"] = "./mcp.json".into();
    value["hooks"] = "./hooks.json".into();
    value["apps"] = "./apps.json".into();
    value["extensions"] = serde_json::json!({"com.openai":{"onboardingSkill":"./skills/SKILL.md"}});
    value["interface"] = serde_json::json!({
        "composerIcon":"./icon.svg", "logo":"./icon.svg", "logoDark":"./icon.svg",
        "screenshots":["./icon.svg"]
    });
    let bytes = serde_json::to_vec_pretty(&value).unwrap();
    fs::write(source.join(".codex-plugin/plugin.json"), &bytes).unwrap();
    let installed =
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "alpha").unwrap();
    assert_eq!(installed.manifest, load_plugin_manifest(&installed.installed_dir).unwrap());
    let root = &installed.manifest.root;
    assert_eq!(installed.manifest.skills[0].canonical_path, root.join("skills"));
    let Some(PluginMcpServers::Path(mcp)) = &installed.manifest.mcp_servers else {
        panic!("expected audited MCP path");
    };
    assert_eq!(mcp.canonical_path, root.join("mcp.json"));
    let PluginHookSource::Path(hook) = &installed.manifest.hooks[0] else {
        panic!("expected audited hook path");
    };
    assert_eq!(hook.canonical_path, root.join("hooks.json"));
    assert_eq!(installed.manifest.apps.as_ref().unwrap().canonical_path, root.join("apps.json"));
    assert_eq!(
        installed.manifest.onboarding_skill.as_ref().unwrap().canonical_path,
        root.join("skills/SKILL.md")
    );
    let interface = installed.manifest.interface.as_ref().unwrap();
    for icon in [&interface.composer_icon, &interface.logo, &interface.logo_dark] {
        assert_eq!(icon.as_ref().unwrap().canonical_path, root.join("icon.svg"));
    }
    assert_eq!(interface.screenshots[0].canonical_path, root.join("icon.svg"));
    let found = discover_plugins(&PluginDiscoveryOptions::new(
        temp.path().join("workspace"),
        config,
        vw_config_types::plugins::PluginsConfig::default(),
    ))
    .unwrap();
    assert_eq!(found.installed.len(), 1);
    assert!(found.diagnostics.is_empty());
    let record = found.installed[0].install_metadata.as_ref().unwrap();
    assert_eq!(record.identity, installed.identity);
    assert_eq!(record.source, source.canonicalize().unwrap().to_string_lossy());
    assert_eq!(fs::read(root.join(".codex-plugin/plugin.json")).unwrap(), bytes);
    assert_eq!(fs::read(source.join(".codex-plugin/plugin.json")).unwrap(), bytes);
    assert!(!root.join("transaction-plugin@alpha.install.json").exists());
    assert_eq!(children(&plugins).len(), 2);
}

#[test]
fn codex_plugin_compat_t08_keeps_concurrent_installations_separate() {
    use std::sync::{Arc, Barrier};

    let temp = tempfile::tempdir().unwrap();
    let plugins = temp.path().join("plugins");
    fs::create_dir(&plugins).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let mut workers = Vec::new();
    for number in 0..2 {
        let source = temp.path().join(format!("source-{number}"));
        package(&source);
        fs::write(source.join("payload.txt"), format!("payload-{number}")).unwrap();
        let plugins = plugins.clone();
        let barrier = barrier.clone();
        workers.push(std::thread::spawn(move || {
            set_next_post_copy_observer_for_test(move |_| {
                barrier.wait();
            });
            (number, install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "alpha"))
        }));
    }
    let results = workers.into_iter().map(|worker| worker.join().unwrap()).collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|(_, result)| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|(_, result)| result.is_err()).count(), 1);
    let (winner, installed) = results.iter().find(|(_, result)| result.is_ok()).unwrap();
    let installed = installed.as_ref().unwrap();
    assert_eq!(
        fs::read_to_string(installed.installed_dir.join("payload.txt")).unwrap(),
        format!("payload-{winner}")
    );
    assert_eq!(children(&plugins).len(), 2, "loser leaves no package or transaction state");
    assert_eq!(
        fs::read(installed.installed_dir.join(".codex-plugin/plugin.json")).unwrap(),
        MANIFEST
    );
}

#[test]
fn codex_plugin_compat_t08_rejects_recursive_install_roots() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    package(&source);
    let before = children(&source);
    for destination in [source.clone(), source.join("nested/plugins")] {
        let error = install_plugin_from_source(source.to_str().unwrap(), &destination).unwrap_err();
        assert!(error.to_string().contains("inside the source"));
        assert_eq!(children(&source), before);
    }
    assert_eq!(fs::read(source.join(".codex-plugin/plugin.json")).unwrap(), MANIFEST);
}

#[cfg(unix)]
#[test]
fn codex_plugin_compat_t08_cleans_real_copy_failures_without_following_symlinks() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let plugins = temp.path().join("plugins");
    package(&source);
    let outside = temp.path().join("outside.txt");
    fs::write(&outside, b"outside bytes").unwrap();
    std::os::unix::fs::symlink(&outside, source.join("untrusted.txt")).unwrap();
    let error = install_plugin_from_source(source.to_str().unwrap(), &plugins).unwrap_err();
    assert!(error.to_string().contains("Refusing to copy symlink"));
    assert!(children(&plugins).is_empty());
    assert_eq!(fs::read(&outside).unwrap(), b"outside bytes");
    assert_eq!(fs::read(source.join(".codex-plugin/plugin.json")).unwrap(), MANIFEST);
}

#[test]
fn codex_plugin_compat_t08_does_not_execute_package_commands() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let plugins = temp.path().join("plugins");
    package(&source);
    let sentinel = temp.path().join("must-not-exist");
    let script = format!("#!/bin/sh\nprintf executed > '{}'\n", sentinel.display());
    fs::write(source.join("setup.sh"), &script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(source.join("setup.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let installed = install_plugin_from_source(source.to_str().unwrap(), &plugins).unwrap();
    assert!(!sentinel.exists());
    assert_eq!(fs::read_to_string(installed.installed_dir.join("setup.sh")).unwrap(), script);
    assert_eq!(fs::read_to_string(source.join("setup.sh")).unwrap(), script);
}

#[test]
fn codex_plugin_compat_t08_keeps_staging_out_of_plugin_discovery() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let config = temp.path().join("config");
    let plugins = config.join("plugins");
    fs::create_dir_all(source.join(".vibewindow-plugin")).unwrap();
    let bytes = serde_json::to_vec(&serde_json::json!({
        "name":".codex-plugin", "version":"1", "description":"Native local name"
    }))
    .unwrap();
    fs::write(source.join(".vibewindow-plugin/plugin.json"), &bytes).unwrap();
    fs::write(source.join("plugin.json"), &bytes).unwrap();
    let workspace = temp.path().join("workspace");
    set_next_post_copy_observer_for_test(move |_| {
        let found = discover_plugins(&PluginDiscoveryOptions::new(
            workspace,
            config,
            vw_config_types::plugins::PluginsConfig::default(),
        ))
        .unwrap();
        assert!(found.installed.is_empty(), "staging cannot masquerade as an installed plugin");
        assert!(found.diagnostics.is_empty(), "staging cannot masquerade as a rejected plugin");
    });
    let installed = install_plugin_from_source(source.to_str().unwrap(), &plugins).unwrap();
    assert_eq!(installed.identity.as_key(), ".codex-plugin");
    assert_eq!(children(&plugins).len(), 2);
}
