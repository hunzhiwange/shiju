use crate::plugins::{PluginHooksManifest, load_plugin_hook_source, load_plugin_manifest};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

fn write(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

fn package(root: &Path, portable: bool, inline: bool, hooks: &Value) {
    let declaration = if inline { hooks.clone() } else { json!("./hooks/hooks.json") };
    let (path, manifest) = if portable {
        (
            root.join("plugin.json"),
            json!({
                "$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
                "name":"hook-fixture", "extensions":{"com.openai":{"hooks":declaration}}
            }),
        )
    } else {
        (root.join(".codex-plugin/plugin.json"), json!({"name":"hook-fixture","hooks":declaration}))
    };
    write(&path, &manifest);
    if !inline {
        write(&root.join("hooks/hooks.json"), hooks);
    }
}

#[test]
fn codex_plugin_compat_t32_runtime_requires_explicit_content_trust() {
    use crate::app::agent::config::Config;
    use crate::plugins::{
        PluginHookTrustStore, plugin_hook_trust_file, trusted_plugin_hook_handlers,
    };
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let root = workspace.join(".vibewindow/plugins/hook-fixture");
    let declaration = json!({"hooks":{"PreToolUse":[{"hooks":[{
        "type":"command", "command":"echo synthetic"
    }]}]}});
    package(&root, false, false, &declaration);
    let config_dir = temp.path().join("config");
    fs::create_dir(&config_dir).unwrap();
    let mut config = Config::default();
    config.workspace_dir = workspace;
    config.config_path = config_dir.join("vibewindow.json");
    config.hooks.enabled = true;
    config.hooks.plugins.enabled = true;
    let manifest_bytes = fs::read(root.join(".codex-plugin/plugin.json")).unwrap();
    let hook_bytes = fs::read(root.join("hooks/hooks.json")).unwrap();

    // Parsing must not grant execution authority, even though execution is now supported.
    assert!(trusted_plugin_hook_handlers(&config).unwrap().is_empty());
    assert_eq!(fs::read_dir(&config_dir).unwrap().count(), 0);
    let plugin = load_plugin_manifest(&root).unwrap();
    let PluginHooksManifest::Codex(parsed) =
        load_plugin_hook_source(&plugin.hooks[0], &root, &config.workspace_dir).unwrap()
    else {
        panic!("expected Codex hook declarations")
    };
    let mut trust =
        PluginHookTrustStore::load_or_default(&plugin_hook_trust_file(&config_dir)).unwrap();
    trust.trust_codex_hooks("hook-fixture", &root, &parsed).unwrap();
    assert_eq!(trusted_plugin_hook_handlers(&config).unwrap().len(), 1);
    assert!(!config_dir.join("plugin-hook-data").exists(), "admission must not execute hooks");
    fs::write(root.join("payload"), b"changed package content").unwrap();
    assert!(trusted_plugin_hook_handlers(&config).unwrap().is_empty());
    assert_eq!(fs::read(root.join(".codex-plugin/plugin.json")).unwrap(), manifest_bytes);
    assert_eq!(fs::read(root.join("hooks/hooks.json")).unwrap(), hook_bytes);
    assert!(!root.join("plugin-hook-trust.json").exists());
}
