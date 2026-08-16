use super::hook_trust::{
    PluginHookTrustStatus, PluginHookTrustStore, plugin_hook_trust_file, plugin_hook_trust_hash,
};
use super::hooks::{PluginHook, PluginHookEvent, PluginHookMode};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn hook(command: &str) -> PluginHook {
    PluginHook {
        event: PluginHookEvent::BeforeToolCall,
        command: command.to_string(),
        args: vec!["--fast".to_string()],
        timeout_secs: Some(5),
        working_dir: PathBuf::from("/workspace/project").join("hooks"),
        mode: PluginHookMode::Blocking,
        manifest_path: PathBuf::from(
            "/workspace/project/.vibewindow/plugins/demo/hooks/hooks.json",
        ),
    }
}

#[test]
fn plugin_hook_trust_hash_changes_when_command_changes() {
    let first = plugin_hook_trust_hash("demo-plugin", &hook("first-command"));
    let second = plugin_hook_trust_hash("demo-plugin", &hook("second-command"));

    println!("plugin hook trust hashes: first={first}, second={second}");
    assert_ne!(first, second);
}

#[test]
fn plugin_hook_trust_marks_unreviewed_hooks_pending() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trust_path = dir.path().join("plugin-hook-trust.json");
    let mut store = PluginHookTrustStore::load_or_default(&trust_path).expect("load store");
    let first_hook = hook("first-command");

    let status = store.review_status("demo-plugin", &first_hook);

    println!("plugin hook trust status: {:?}", status);
    assert_eq!(status, PluginHookTrustStatus::Pending);
    assert_eq!(
        plugin_hook_trust_file(Path::new("/tmp/vw-config")),
        Path::new("/tmp/vw-config").join("plugin-hook-trust.json")
    );
    store.trust_hook("demo-plugin", &first_hook).expect("trust hook");
    let reloaded = PluginHookTrustStore::load_or_default(&trust_path).expect("reload store");
    assert_eq!(reloaded.review_status("demo-plugin", &first_hook), PluginHookTrustStatus::Trusted);
    assert_eq!(
        reloaded.review_status("demo-plugin", &hook("changed-command")),
        PluginHookTrustStatus::Pending
    );
}

#[test]
fn plugin_hook_trust_revokes_only_the_exact_hash() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trust_path = dir.path().join("plugin-hook-trust.json");
    let mut store = PluginHookTrustStore::load_or_default(&trust_path).expect("load store");
    let first = hook("first-command");
    let second = hook("second-command");
    store.trust_hook("demo-plugin", &first).expect("trust first");
    store.trust_hook("demo-plugin", &second).expect("trust second");

    assert!(store.revoke_hash(&plugin_hook_trust_hash("demo-plugin", &first)).unwrap());

    let reloaded = PluginHookTrustStore::load_or_default(&trust_path).expect("reload store");
    assert_eq!(reloaded.review_status("demo-plugin", &first), PluginHookTrustStatus::Pending);
    assert_eq!(reloaded.review_status("demo-plugin", &second), PluginHookTrustStatus::Trusted);
}

#[test]
fn plugin_hook_trust_rejects_malformed_store() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trust_path = dir.path().join("plugin-hook-trust.json");
    fs::write(&trust_path, "{not-json").expect("malformed trust store");

    let err = PluginHookTrustStore::load_or_default(&trust_path)
        .expect_err("malformed store rejected")
        .to_string();

    assert!(err.contains("failed to parse plugin hook trust store"), "{err}");
    assert!(err.contains("plugin-hook-trust.json"), "{err}");
}

#[test]
fn plugin_hook_trust_upgrades_zero_version_when_saved() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trust_path = dir.path().join("plugin-hook-trust.json");
    fs::write(&trust_path, r#"{"version":0,"trusted_hashes":[]}"#).expect("legacy trust store");
    let mut store = PluginHookTrustStore::load_or_default(&trust_path).expect("legacy store");

    store.trust_hook("demo-plugin", &hook("reviewed-command")).expect("save upgraded store");

    let saved: Value = serde_json::from_slice(&fs::read(&trust_path).expect("saved store"))
        .expect("saved trust json");
    assert_eq!(saved["version"], 1);
    assert_eq!(saved["trusted_hashes"].as_array().expect("trusted hashes").len(), 1);
}

#[test]
fn plugin_hook_trust_defaults_missing_version_when_saved() {
    let dir = tempfile::tempdir().expect("temp dir");
    let trust_path = dir.path().join("plugin-hook-trust.json");
    fs::write(&trust_path, r#"{"trusted_hashes":[]}"#).expect("versionless trust store");
    let mut store = PluginHookTrustStore::load_or_default(&trust_path).expect("versionless store");

    store.trust_hook("demo-plugin", &hook("version-defaulted")).expect("save defaulted store");

    let saved: Value = serde_json::from_slice(&fs::read(&trust_path).expect("saved store"))
        .expect("saved trust json");
    assert_eq!(saved["version"], 1);
}

#[test]
fn plugin_hook_trust_reports_read_error_when_path_is_directory() {
    let dir = tempfile::tempdir().expect("temp dir");

    let err = PluginHookTrustStore::load_or_default(dir.path())
        .expect_err("directory path should not load as trust store")
        .to_string();

    assert!(err.contains("failed to read plugin hook trust store"), "{err}");
}

#[test]
fn plugin_hook_trust_reports_create_dir_error_when_parent_is_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file_parent = dir.path().join("not-a-directory");
    fs::write(&file_parent, "not a directory").expect("file parent");
    let trust_path = file_parent.join("plugin-hook-trust.json");
    let mut store = PluginHookTrustStore::load_or_default(&trust_path).expect("default store");

    let err = store
        .trust_hook("demo-plugin", &hook("unwritable-parent"))
        .expect_err("file parent should prevent trust store save")
        .to_string();

    assert!(err.contains("failed to create plugin hook trust dir"), "{err}");
}
