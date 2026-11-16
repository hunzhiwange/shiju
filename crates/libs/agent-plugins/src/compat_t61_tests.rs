use crate::install::{remove_installed_plugin, set_plugin_enabled};
use crate::{PluginDiscoveryOptions, discover_plugins, install_plugin_from_marketplace};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

// Keep consuming the locked original archives, rather than copying their contents.
fn fixture_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vw-agent")
}

fn digest(root: &Path) -> Vec<(String, String)> {
    fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, String)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let kind = entry.file_type().unwrap();
            if kind.is_dir() {
                collect(root, &entry.path(), files);
            } else if kind.is_file() {
                files.push((
                    entry.path().strip_prefix(root).unwrap().to_string_lossy().into_owned(),
                    hex::encode(Sha256::digest(fs::read(entry.path()).unwrap())),
                ));
            }
        }
    }
    let mut files = Vec::new();
    collect(root, root, &mut files);
    files.sort();
    files
}

fn public_original(root: &Path, name: &str) -> std::path::PathBuf {
    let fixture = fixture_root()
        .join("testdata/plugin_compat/t61/public")
        .join(format!("{name}.tar.gz"));
    let archive = fs::File::open(fixture).unwrap();
    tar::Archive::new(flate2::read::GzDecoder::new(archive)).unpack(root).unwrap();
    root.join(name)
}

fn package(p: &Path) {
    fs::create_dir_all(p.join(".codex-plugin")).unwrap();
    fs::write(p.join(".codex-plugin/plugin.json"), r#"{"name":"original","version":"1"}"#).unwrap();
    fs::write(p.join("payload"), b"unchanged original bytes").unwrap();
}
#[test]
fn codex_plugin_compat_t61_executes_original_package_without_mutation() {
    let t = tempfile::tempdir().unwrap();
    let source = t.path().join("source");
    package(&source);
    let before = digest(&source);
    let config = t.path().join("config");
    let plugins = config.join("plugins");
    let installed =
        install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "market").unwrap();
    assert_eq!(digest(&installed.installed_dir), before);
    let options =
        PluginDiscoveryOptions::new(t.path().join("workspace"), &config, Default::default());
    assert_eq!(discover_plugins(&options).unwrap().enabled.len(), 1);
    set_plugin_enabled(&plugins, "original@market", false, &plugin_mcp::mcp_connection_sessions::close_plugin).unwrap();
    assert!(discover_plugins(&options).unwrap().enabled.is_empty());
    set_plugin_enabled(&plugins, "original@market", true, &plugin_mcp::mcp_connection_sessions::close_plugin).unwrap();
    assert_eq!(discover_plugins(&options).unwrap().enabled.len(), 1);
    remove_installed_plugin(&plugins, "original@market", &plugin_mcp::mcp_connection_sessions::close_plugin).unwrap();
    assert!(discover_plugins(&options).unwrap().installed.is_empty());
    assert_eq!(digest(&source), before);
}
#[test]
fn codex_plugin_compat_t61_rejects_invalid_input() {
    let t = tempfile::tempdir().unwrap();
    let source = t.path().join("source");
    package(&source);
    fs::write(source.join(".codex-plugin/plugin.json"), b"{broken").unwrap();
    assert!(
        install_plugin_from_marketplace(
            source.to_str().unwrap(),
            &t.path().join("plugins"),
            "market"
        )
        .is_err()
    );
    assert_eq!(fs::read(source.join("payload")).unwrap(), b"unchanged original bytes");
}
#[test]
fn codex_plugin_compat_t61_preserves_isolation() {
    let t = tempfile::tempdir().unwrap();
    let source = t.path().join("source");
    package(&source);
    let plugins = t.path().join("plugins");
    let a = install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "alpha").unwrap();
    let b = install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "beta").unwrap();
    remove_installed_plugin(&plugins, "original@alpha", &plugin_mcp::mcp_connection_sessions::close_plugin).unwrap();
    assert!(!a.installed_dir.exists());
    assert!(b.installed_dir.exists());
    assert_eq!(digest(&source), digest(&b.installed_dir));
}

#[test]
fn codex_plugin_compat_t61_public_sources_match_locked_hashes() {
    let root = fixture_root().join("testdata/plugin_compat/t61/public");
    let sources: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("sources.json")).unwrap()).unwrap();
    for package in sources["packages"].as_array().unwrap() {
        let directory = package["directory"].as_str().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let actual = digest(&public_original(temp.path(), directory))
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        let expected = package["files"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(path, hash)| (path.clone(), hash.as_str().unwrap().to_owned()))
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(actual, expected, "original source drift: {directory}");
    }
}
