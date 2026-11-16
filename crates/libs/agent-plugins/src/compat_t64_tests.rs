//! Local installation contracts for packages with Unicode and spaced paths.
use crate::{PluginDiscoveryOptions, install_plugin_from_marketplace};
use sha2::{Digest, Sha256};
use std::{fs, path::{Path, PathBuf}};

fn digest(root: &Path) -> Vec<(String, String)> {
    fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, String)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let kind = entry.file_type().unwrap();
            if kind.is_dir() {
                collect(root, &path, files);
            } else if kind.is_file() {
                files.push((
                    path.strip_prefix(root).unwrap().to_string_lossy().into_owned(),
                    hex::encode(Sha256::digest(fs::read(&path).unwrap())),
                ));
            }
        }
    }
    let mut files = Vec::new();
    collect(root, root, &mut files);
    files.sort();
    files
}

fn package(root: &Path) -> PathBuf {
    let source = root.join("插件 original package");
    fs::create_dir_all(&source).unwrap();
    // Keep the exact repository-authored package used by host MCP acceptance.
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vw-agent/testdata/plugin_compat/t61/package");
    for name in ["plugin.json", "mcp.json", "server.py"] {
        fs::copy(fixtures.join(name), source.join(name)).unwrap();
    }
    source
}

fn remove_installed_plugin(root: &Path, name: &str) -> anyhow::Result<()> {
    crate::install::remove_installed_plugin(
        root,
        name,
        &plugin_mcp::mcp_connection_sessions::close_plugin,
    )
}

#[test]
fn codex_plugin_compat_t64_rejects_invalid_input() {
    let t = tempfile::tempdir().unwrap();
    let source = package(t.path());
    fs::write(source.join("plugin.json"), b"{broken").unwrap();
    let before = digest(&source);
    let plugins = t.path().join("plugins");
    assert!(install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "market").is_err());
    assert_eq!(digest(&source), before);
    assert!(
        crate::discover_plugins(&PluginDiscoveryOptions::new(
            t.path().join("workspace"),
            t.path(),
            Default::default()
        ))
        .unwrap()
        .installed
        .is_empty()
    );
}
#[test]
fn codex_plugin_compat_t64_preserves_isolation() {
    let t = tempfile::tempdir().unwrap();
    let source = package(t.path());
    let plugins = t.path().join("plugins");
    let a = install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "alpha").unwrap();
    let b = install_plugin_from_marketplace(source.to_str().unwrap(), &plugins, "beta").unwrap();
    remove_installed_plugin(&plugins, "original@alpha").unwrap();
    assert!(!a.installed_dir.exists());
    assert!(b.installed_dir.exists());
    assert_eq!(digest(&source), digest(&b.installed_dir));
}
