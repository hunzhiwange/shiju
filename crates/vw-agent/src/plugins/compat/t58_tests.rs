use crate::plugins::{DependencyState, inspect_plugin_dependencies};
use serde_json::json;
use std::{fs, path::Path};
fn package(root: &Path, command: &str, args: serde_json::Value) {
    fs::create_dir_all(root).unwrap();
    fs::write(root.join("plugin.json"),json!({"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"fixture","version":"1"}).to_string()).unwrap();
    fs::write(root.join("mcp.json"),json!({"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{"fixture":{"type":"stdio","command":command,"args":args}}}).to_string()).unwrap();
}
#[test]
fn codex_plugin_compat_t58_reports_missing_runtime_before_execution() {
    let root = tempfile::tempdir().unwrap();
    package(root.path(), "shiju-fixture-runtime-not-installed-672a", json!([]));
    let checks = inspect_plugin_dependencies(root.path()).unwrap();
    assert!(checks.iter().any(|c| c.state == DependencyState::MissingHost));
    assert!(!root.path().join("plugin-data").exists());
}
#[test]
fn codex_plugin_compat_t58_rejects_invalid_input() {
    let root = tempfile::tempdir().unwrap();
    package(root.path(), "node", json!(["${PLUGIN_ROOT}/../escape.js"]));
    assert!(inspect_plugin_dependencies(root.path()).is_err());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}
#[test]
fn codex_plugin_compat_t58_preserves_isolation() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    for root in [first.path(), second.path()] {
        package(root, "node", json!(["${PLUGIN_ROOT}/server.js"]));
    }
    fs::write(first.path().join("server.js"), "require('fs').writeFileSync('MUST_NOT_RUN','bad')")
        .unwrap();
    let before = fs::read(first.path().join("server.js")).unwrap();
    let a = inspect_plugin_dependencies(first.path()).unwrap();
    let b = inspect_plugin_dependencies(second.path()).unwrap();
    assert!(a.iter().any(|c| c.state == DependencyState::Present && c.dependency == "server.js"));
    assert!(b.iter().any(|c| c.state == DependencyState::MissingPackage));
    assert_eq!(fs::read(first.path().join("server.js")).unwrap(), before);
    assert!(!first.path().join("MUST_NOT_RUN").exists());
    assert!(!second.path().join("MUST_NOT_RUN").exists());
}

#[test]
fn codex_plugin_compat_t58_recognizes_bundled_python_module_without_importing() {
    let root = tempfile::tempdir().unwrap();
    package(root.path(), "python3", json!(["-m", "bundled"]));
    fs::create_dir(root.path().join("bundled")).unwrap();
    fs::write(root.path().join("bundled/__main__.py"), "open('MUST_NOT_RUN','w').write('bad')")
        .unwrap();
    let checks = inspect_plugin_dependencies(root.path()).unwrap();
    assert!(
        checks
            .iter()
            .any(|c| c.state == DependencyState::Present && c.dependency == "bundled/__main__.py")
    );
    assert!(!root.path().join("MUST_NOT_RUN").exists());
}
#[cfg(unix)]
#[test]
fn codex_plugin_compat_t58_rejects_dependency_symlink() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("server.js"), "private").unwrap();
    package(root.path(), "node", json!(["${PLUGIN_ROOT}/server.js"]));
    std::os::unix::fs::symlink(outside.path().join("server.js"), root.path().join("server.js"))
        .unwrap();
    assert!(inspect_plugin_dependencies(root.path()).is_err());
    assert_eq!(fs::read(outside.path().join("server.js")).unwrap(), b"private");
}
