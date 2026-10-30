//! Real discovery entry-point checks for manifest formats, priority and rejected-package isolation.

use crate::plugins::{PluginDiscoveryOptions, discover_plugins, install_plugin_from_marketplace};
use std::fs;
use std::path::{Path, PathBuf};
use vw_config_types::plugins::PluginsConfig;

const NATIVE: &[u8] = include_bytes!("../../../testdata/plugin_compat/t07/native.json");
const LEGACY: &[u8] = include_bytes!("../../../testdata/plugin_compat/t07/legacy.json");
const PORTABLE: &[u8] = include_bytes!("../../../testdata/plugin_compat/t07/portable.json");

fn write_manifest(root: &Path, relative: &str, bytes: &[u8]) -> PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn codex_plugin_compat_t07_discovers_all_manifest_formats() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    let source = workspace.join(".vibewindow/plugins");
    let cases = [
        ("a-native", ".vibewindow-plugin/plugin.json", NATIVE, "native-plugin"),
        ("b-codex", ".codex-plugin/plugin.json", LEGACY, "legacy-plugin"),
        ("c-claude", ".claude-plugin/plugin.json", LEGACY, "legacy-plugin"),
        ("d-cursor", ".cursor-plugin/plugin.json", LEGACY, "legacy-plugin"),
        ("e-portable", "plugin.json", PORTABLE, "portable-plugin"),
    ];
    let mut originals = Vec::new();
    for (directory, relative, bytes, name) in cases {
        let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        value["name"] = format!("{name}-{directory}").into();
        let bytes = serde_json::to_vec(&value).unwrap();
        let path = write_manifest(&source.join(directory), relative, &bytes);
        originals.push((path, bytes));
    }
    let found =
        discover_plugins(&PluginDiscoveryOptions::new(workspace, config, PluginsConfig::default()))
            .unwrap();
    assert_eq!(found.installed.len(), 5, "all five supported entry points must be discovered");
    assert_eq!(found.enabled.len(), 5);
    assert!(found.diagnostics.is_empty());
    assert_eq!(
        found.installed.iter().map(|p| p.manifest.name.as_str()).collect::<Vec<_>>(),
        [
            "native-plugin-a-native",
            "legacy-plugin-b-codex",
            "legacy-plugin-c-claude",
            "legacy-plugin-d-cursor",
            "portable-plugin-e-portable",
        ]
    );
    for (path, bytes) in originals {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn codex_plugin_compat_t07_rejects_invalid_input() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let source = workspace.join(".vibewindow/plugins");
    let good = write_manifest(&source.join("a-healthy"), ".vibewindow-plugin/plugin.json", NATIVE);
    let broken_root = source.join("b-broken");
    let broken = write_manifest(&broken_root, ".vibewindow-plugin/plugin.json", b"{");
    let found = discover_plugins(&PluginDiscoveryOptions::new(
        workspace,
        temp.path().join("config"),
        PluginsConfig::default(),
    ))
    .expect("one damaged package must not discard healthy packages");
    assert_eq!(found.enabled.len(), 1);
    assert_eq!(found.enabled[0].manifest.name, "native-plugin");
    assert_eq!(found.diagnostics.len(), 1);
    assert_eq!(found.diagnostics[0].plugin_root, broken_root);
    assert!(found.diagnostics[0].message.contains("failed to parse plugin manifest"));
    assert_eq!(fs::read(broken).unwrap(), b"{");
    assert_eq!(fs::read(good).unwrap(), NATIVE);
}

#[test]
fn codex_plugin_compat_t07_preserves_isolation() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    let preferred = workspace.join(".vibewindow/plugins/a-workspace");
    write_manifest(&preferred, ".vibewindow-plugin/plugin.json", NATIVE);
    write_manifest(&config.join("plugins/a-global"), ".vibewindow-plugin/plugin.json", NATIVE);
    write_manifest(&workspace.join("extra/a-explicit"), ".vibewindow-plugin/plugin.json", NATIVE);
    let package = temp.path().join("market-package");
    write_manifest(&package, ".vibewindow-plugin/plugin.json", NATIVE);
    let market = config.join("plugins");
    install_plugin_from_marketplace(package.to_str().unwrap(), &market, "alpha").unwrap();
    install_plugin_from_marketplace(package.to_str().unwrap(), &market, "beta").unwrap();
    fs::write(market.join("native-plugin@beta.disabled"), b"").unwrap();
    let excluded = temp.path().join(".codex/plugins/should-not-be-scanned");
    let excluded_manifest = write_manifest(&excluded, "plugin.json", PORTABLE);
    let found = discover_plugins(&PluginDiscoveryOptions::new(
        &workspace,
        &config,
        PluginsConfig { sources: vec!["extra".into()], enabled: vec![] },
    ))
    .unwrap();
    let local = found.installed.iter().find(|p| p.identity.marketplace_name().is_none()).unwrap();
    assert_eq!(local.manifest.root, preferred.canonicalize().unwrap(), "workspace takes priority");
    assert_eq!(found.installed.len(), 3, "same-name marketplaces remain distinct");
    assert_eq!(found.enabled.len(), 2);
    assert!(found.enabled.iter().all(|p| p.identity.as_key() != "native-plugin@beta"));
    assert_eq!(fs::read(excluded_manifest).unwrap(), PORTABLE);
    assert_eq!(fs::read(package.join(".vibewindow-plugin/plugin.json")).unwrap(), NATIVE);
}

#[test]
fn codex_plugin_compat_t07_selects_portable_then_native_then_legacy() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let root = workspace.join("plugins/package");
    let codex = write_manifest(&root, ".codex-plugin/plugin.json", LEGACY);
    write_manifest(&root, ".claude-plugin/plugin.json", br#"{"name":"claude-copy"}"#);
    write_manifest(&root, ".cursor-plugin/plugin.json", br#"{"name":"cursor-copy"}"#);
    let options = PluginDiscoveryOptions::new(
        &workspace,
        temp.path().join("config"),
        PluginsConfig::default(),
    );
    let found = discover_plugins(&options).unwrap();
    assert_eq!(found.installed[0].manifest.manifest_path, codex.canonicalize().unwrap());
    let native = write_manifest(&root, ".vibewindow-plugin/plugin.json", NATIVE);
    assert_eq!(discover_plugins(&options).unwrap().installed[0].manifest.name, "native-plugin");
    let portable = write_manifest(&root, "plugin.json", PORTABLE);
    let found = discover_plugins(&options).unwrap();
    assert_eq!(found.installed[0].manifest.name, "portable-plugin");
    assert_eq!(found.installed[0].manifest.manifest_path, portable.canonicalize().unwrap());
    fs::write(&portable, br#"{"$schema":"https://agent-plugins.org/schemas/9.0.0/plugin.schema.json","name":"future"}"#).unwrap();
    let rejected = discover_plugins(&options).unwrap();
    assert!(rejected.installed.is_empty(), "unknown portable versions cannot downgrade");
    assert!(rejected.diagnostics[0].message.contains("unsupported Agent Plugins schema"));
    assert_eq!(fs::read(native).unwrap(), NATIVE);
    assert_eq!(fs::read(codex).unwrap(), LEGACY);
}

#[test]
fn codex_plugin_compat_t07_ignores_unrelated_roots_and_keeps_legacy_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let source = workspace.join("plugins");
    let root = source.join("package");
    let npm = br#"{"name":"npm-metadata","version":"1"}"#;
    write_manifest(&source.join("not-a-plugin"), "plugin.json", npm);
    write_manifest(&root, "plugin.json", npm);
    let claude = write_manifest(&root, ".claude-plugin/plugin.json", LEGACY);
    let options = PluginDiscoveryOptions::new(
        workspace,
        temp.path().join("config"),
        PluginsConfig::default(),
    );
    let found = discover_plugins(&options).unwrap();
    assert_eq!(found.installed.len(), 1);
    assert_eq!(found.installed[0].manifest.manifest_path, claude.canonicalize().unwrap());
    assert!(found.diagnostics.is_empty());
    write_manifest(&root, ".codex-plugin/plugin.json", b"{");
    let rejected = discover_plugins(&options).unwrap();
    assert!(rejected.installed.is_empty(), "damaged selected legacy cannot downgrade");
    assert!(rejected.diagnostics[0].message.contains("failed to parse plugin manifest"));
    assert_eq!(fs::read(root.join("plugin.json")).unwrap(), npm);
}

#[test]
fn codex_plugin_compat_t07_preserves_disabled_priority_and_stable_order() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    let preferred = workspace.join(".vibewindow/plugins");
    write_manifest(&preferred.join("b-later"), ".vibewindow-plugin/plugin.json", NATIVE);
    let first = preferred.join("a-first");
    write_manifest(&first, ".vibewindow-plugin/plugin.json", NATIVE);
    write_manifest(
        &workspace.join("plugins/c-workspace"),
        ".vibewindow-plugin/plugin.json",
        NATIVE,
    );
    write_manifest(&config.join("plugins/d-global"), ".vibewindow-plugin/plugin.json", NATIVE);
    fs::write(preferred.join("native-plugin.disabled"), b"").unwrap();
    let options = PluginDiscoveryOptions::new(workspace, config, PluginsConfig::default());
    let initial = discover_plugins(&options).unwrap();
    assert_eq!(initial.installed.len(), 1);
    assert_eq!(initial.installed[0].manifest.root, first.canonicalize().unwrap());
    assert!(initial.enabled.is_empty(), "a lower-priority copy cannot revive a disabled identity");
    assert_eq!(discover_plugins(&options).unwrap(), initial);
}

#[test]
fn codex_plugin_compat_t07_reports_invalid_explicit_sources() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    for (source, message) in
        [("missing-source", "does not exist"), ("file-source", "must be a directory")]
    {
        fs::write(workspace.join("file-source"), b"unchanged").unwrap();
        let options = PluginDiscoveryOptions::new(
            &workspace,
            temp.path().join("config"),
            PluginsConfig { enabled: vec![], sources: vec![source.to_owned()] },
        );
        let error = discover_plugins(&options).unwrap_err().to_string();
        assert!(error.contains(source), "{error}");
        assert!(error.contains(message), "{error}");
    }
    assert_eq!(fs::read(workspace.join("file-source")).unwrap(), b"unchanged");
}

#[test]
fn codex_plugin_compat_t07_isolates_rejected_installation_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    let package = temp.path().join("package");
    write_manifest(&package, ".vibewindow-plugin/plugin.json", NATIVE);
    let installed = install_plugin_from_marketplace(
        package.to_str().unwrap(),
        &config.join("plugins"),
        "alpha",
    )
    .unwrap();
    let metadata = super::identity::metadata_path(&installed.installed_dir).unwrap();
    fs::write(&metadata, b"{").unwrap();
    let healthy = workspace.join("plugins/healthy");
    write_manifest(&healthy, ".vibewindow-plugin/plugin.json", NATIVE);
    let found =
        discover_plugins(&PluginDiscoveryOptions::new(workspace, config, PluginsConfig::default()))
            .unwrap();
    assert_eq!(found.enabled.len(), 1);
    assert_eq!(found.enabled[0].manifest.root, healthy.canonicalize().unwrap());
    assert!(found.diagnostics.iter().any(|d| d.plugin_root == installed.installed_dir));
    assert_eq!(fs::read(metadata).unwrap(), b"{");
    assert_eq!(fs::read(package.join(".vibewindow-plugin/plugin.json")).unwrap(), NATIVE);
}

#[test]
fn codex_plugin_compat_t07_preserves_healthy_skill_and_mcp_consumers() {
    use crate::app::agent::config::Config;
    use crate::app::agent::skills::SkillLoadMode;

    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let config_dir = temp.path().join("config");
    let source = workspace.join("plugins");
    let healthy = source.join("a-healthy");
    let skill_path = healthy.join("skills/kept-skill/SKILL.md");
    fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
    fs::write(&skill_path, "# kept-skill\nHealthy plugin skill\n").unwrap();
    fs::write(
        healthy.join("mcp.json"),
        br#"[{"name":"kept-mcp","command":"unused-test-command"}]"#,
    )
    .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(NATIVE).unwrap();
    value["skills"] = "./skills".into();
    value["mcpServers"] = "./mcp.json".into();
    write_manifest(
        &healthy,
        ".vibewindow-plugin/plugin.json",
        &serde_json::to_vec(&value).unwrap(),
    );
    write_manifest(&source.join("b-broken"), ".codex-plugin/plugin.json", b"{");
    let mut config = Config::default();
    config.workspace_dir = workspace.clone();
    config.config_path = config_dir.join("vibewindow.json");
    let skills = crate::plugins::load_plugin_skills(&workspace, &config, SkillLoadMode::Full);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "kept-skill");
    let mut servers = Vec::new();
    crate::plugins::merge_enabled_plugin_mcp_servers(
        &mut servers,
        &PluginDiscoveryOptions::new(workspace, config_dir, PluginsConfig::default()),
    )
    .unwrap();
    assert_eq!(servers.len(), 1);
    assert!(
        matches!(&servers[0], agent_client_protocol::McpServer::Stdio(server) if server.name == "kept-mcp")
    );
    assert_eq!(fs::read_to_string(skill_path).unwrap(), "# kept-skill\nHealthy plugin skill\n");
}

#[cfg(unix)]
#[test]
fn codex_plugin_compat_t07_rejects_symlinks_without_following_them() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let source = workspace.join("plugins");
    write_manifest(&source.join("healthy"), ".vibewindow-plugin/plugin.json", NATIVE);
    let outside = temp.path().join("outside");
    let outside_manifest = write_manifest(&outside, "plugin.json", PORTABLE);
    symlink(&outside, source.join("linked-root")).unwrap();
    let linked_manifest = source.join("linked-manifest");
    fs::create_dir_all(&linked_manifest).unwrap();
    symlink(&outside_manifest, linked_manifest.join("plugin.json")).unwrap();
    let found = discover_plugins(&PluginDiscoveryOptions::new(
        &workspace,
        temp.path().join("config"),
        PluginsConfig::default(),
    ))
    .unwrap();
    assert_eq!(found.enabled.len(), 1);
    assert_eq!(found.diagnostics.len(), 2);
    assert!(found.diagnostics.iter().all(|d| d.message.contains("symlink")));
    // A dangling disabled-marker link must never be interpreted as an enabled package.
    symlink(temp.path().join("absent"), source.join("native-plugin.disabled")).unwrap();
    let found = discover_plugins(&PluginDiscoveryOptions::new(
        workspace,
        temp.path().join("config"),
        PluginsConfig::default(),
    ))
    .unwrap();
    assert!(found.enabled.is_empty());
    assert!(
        found
            .diagnostics
            .iter()
            .any(|d| d.plugin_root == source.join("healthy")
                && d.message.contains("disabled marker"))
    );
    assert_eq!(fs::read(outside_manifest).unwrap(), PORTABLE);
    assert!(!temp.path().join("absent").exists());
}
