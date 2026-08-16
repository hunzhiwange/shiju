use super::{HookResult, build_hook_runtime};
use crate::app::agent::config::Config;
use crate::plugins::{
    PluginHookTrustStore, load_plugin_hooks_manifest, plugin_hook_trust_file,
};
use crate::app::agent::tools::ToolResult;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::time::Duration;

fn write_json(path: &Path, value: serde_json::Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("json parent");
    }
    fs::write(path, serde_json::to_vec_pretty(&value).expect("json bytes"))
        .expect("write json");
}

#[cfg(unix)]
fn write_script(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("script parent");
    }
    fs::write(path, body).expect("write script");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("script executable");
}

#[cfg(unix)]
#[tokio::test]
async fn hook_runtime_respects_configured_handlers() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/runtime-plugin");
    let marker_path = dir.path().join("plugin-ran");
    let script_path = plugin_root.join("scripts/mark.sh");
    write_json(
        &plugin_root.join(".vibewindow-plugin/plugin.json"),
        json!({
            "name": "runtime-plugin",
            "version": "0.1.0",
            "description": "runtime test plugin",
            "hooks": "./hooks/hooks.json"
        }),
    );
    write_script(&script_path, "#!/bin/sh\ntouch \"$1\"\n");
    write_json(
        &plugin_root.join("hooks/hooks.json"),
        json!({
            "hooks": [{
                "event": "before_tool_call",
                "command": script_path.display().to_string(),
                "args": [marker_path.display().to_string()],
                "working_dir": "./scripts"
            }]
        }),
    );

    let hooks = load_plugin_hooks_manifest(
        &plugin_root.join("hooks/hooks.json"),
        &plugin_root,
        &workspace_dir,
    )
    .expect("hooks manifest");
    let trust_path = plugin_hook_trust_file(&config_dir);
    let mut trust = PluginHookTrustStore::load_or_default(&trust_path).expect("load trust store");
    trust.trust_hook("runtime-plugin", &hooks[0]).expect("trust hook");

    let mut config = Config::default();
    config.workspace_dir = workspace_dir;
    config.config_path = config_dir.join("vibewindow.json");
    config.hooks.enabled = false;
    config.hooks.builtin.command_logger = true;
    config.hooks.plugins.enabled = true;

    let disabled = build_hook_runtime(&config).expect("disabled runtime");
    assert!(disabled.runner_snapshot().is_none());

    config.hooks.enabled = true;
    config.hooks.builtin.command_logger = false;
    config.hooks.plugins.enabled = false;
    let plugins_disabled = build_hook_runtime(&config).expect("plugins disabled runtime");
    let runner = plugins_disabled.runner_snapshot().expect("enabled empty runner");
    let result = runner.run_before_tool_call("shell".into(), json!({})).await;
    assert!(matches!(result, HookResult::Continue(_)));
    assert!(!marker_path.exists());

    config.hooks.plugins.enabled = true;
    let plugins_enabled = build_hook_runtime(&config).expect("plugins enabled runtime");
    let runner = plugins_enabled.runner_snapshot().expect("plugin runner");
    let result = runner.run_before_tool_call("shell".into(), json!({})).await;
    assert!(matches!(result, HookResult::Continue(_)));
    assert!(marker_path.exists());

    config.hooks.plugins.enabled = false;
    config.hooks.builtin.command_logger = true;
    let builtin_enabled = build_hook_runtime(&config).expect("builtin enabled runtime");
    let runner = builtin_enabled.runner_snapshot().expect("builtin runner");
    let sensitive = ToolResult {
        success: false,
        output: "secret result".into(),
        error: Some("token=secret".into()),
    };
    runner.fire_after_tool_call("shell", &sensitive, Duration::from_millis(7)).await;
    let entries = builtin_enabled.audit_store().entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].tool_name, "shell");
    assert_eq!(entries[0].duration_ms, 7);
    assert!(!entries[0].success);
    let serialized = serde_json::to_string(&entries[0]).expect("serialize audit entry");
    assert!(!serialized.contains("secret result"));
    assert!(!serialized.contains("token=secret"));
}

#[test]
fn hook_runtime_replaces_only_complete_candidates() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut config = Config::default();
    config.workspace_dir = dir.path().join("workspace");
    config.config_path = dir.path().join("config/vibewindow.json");
    config.hooks.builtin.command_logger = true;
    config.hooks.plugins.enabled = false;
    let runtime = build_hook_runtime(&config).expect("initial runtime");
    let initial = runtime.runner_snapshot().expect("initial runner");

    let broken_plugin = config.workspace_dir.join(".vibewindow/plugins/broken");
    write_json(
        &broken_plugin.join(".vibewindow-plugin/plugin.json"),
        json!({
            "name": "broken",
            "version": "0.1.0",
            "description": "broken runtime test plugin",
            "hooks": "./hooks/hooks.json"
        }),
    );
    fs::create_dir_all(broken_plugin.join("hooks")).expect("hooks dir");
    fs::write(broken_plugin.join("hooks/hooks.json"), "{not json").expect("broken hooks");
    config.hooks.plugins.enabled = true;

    assert!(runtime.build_candidate(&config).is_err());
    let after_error = runtime.runner_snapshot().expect("runner retained after candidate failure");
    assert!(std::sync::Arc::ptr_eq(&initial, &after_error));

    config.hooks.enabled = false;
    let disabled = runtime.build_candidate(&config).expect("disabled candidate");
    runtime.replace(disabled);
    assert!(runtime.runner_snapshot().is_none());
}
