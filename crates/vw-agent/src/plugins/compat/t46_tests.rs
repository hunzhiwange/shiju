use crate::plugins::{CodexHooksManifest, PluginHookTrustStore};
fn manifest() -> CodexHooksManifest {
    serde_json::from_value(serde_json::json!({"hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":"sh script.sh"}]}]}})).unwrap()
}
#[test]
fn codex_plugin_compat_t46_revokes_trust_after_script_change() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("script.sh"), "exit 0").unwrap();
    let mut store =
        PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
    store.trust_codex_hooks("fixture", root.path(), &manifest()).unwrap();
    assert!(store.codex_hooks_trusted("fixture", root.path(), &manifest()).unwrap());
    let mut changed = serde_json::to_value(manifest()).unwrap();
    changed["hooks"]["PreToolUse"][0]["matcher"] = serde_json::json!("Read");
    let changed: CodexHooksManifest = serde_json::from_value(changed).unwrap();
    assert!(!store.codex_hooks_trusted("fixture", root.path(), &changed).unwrap());
    std::fs::write(root.path().join("script.sh"), "exit 1").unwrap();
    assert!(!store.codex_hooks_trusted("fixture", root.path(), &manifest()).unwrap());
}
#[test]
fn codex_plugin_compat_t46_rejects_invalid_input() {
    let root = tempfile::NamedTempFile::new().unwrap();
    let state = tempfile::tempdir().unwrap();
    let mut store =
        PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
    assert!(store.trust_codex_hooks("fixture", root.path(), &manifest()).is_err());
    assert!(!state.path().join("trust.json").exists());
}
#[test]
fn codex_plugin_compat_t46_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let mut store =
        PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
    store.trust_codex_hooks("fixture", root.path(), &manifest()).unwrap();
    assert!(!store.codex_hooks_trusted("fixture", other.path(), &manifest()).unwrap());
    assert!(!store.codex_hooks_trusted("other", root.path(), &manifest()).unwrap());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
