//! Actual application host binds trusted command handlers into its runner.
#[cfg(unix)]
#[tokio::test]
async fn plugin_hook_host_registers_and_cancels() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin = workspace.join(".vibewindow/plugins/host-hook");
    std::fs::create_dir_all(plugin.join(".vibewindow-plugin")).unwrap();
    std::fs::create_dir_all(plugin.join("hooks")).unwrap();
    std::fs::create_dir_all(plugin.join("scripts")).unwrap();
    std::fs::write(plugin.join(".vibewindow-plugin/plugin.json"), r#"{"name":"host-hook","hooks":"./hooks/hooks.json"}"#).unwrap();
    std::fs::write(plugin.join("hooks/hooks.json"), r#"{"hooks":[{"event":"before_tool_call","command":"/bin/sh","args":["-c","cat >/dev/null; exit 23"],"working_dir":"./scripts"}]}"#).unwrap();
    let hooks = super::load_plugin_hooks_manifest(&plugin.join("hooks/hooks.json"), &plugin, &workspace).unwrap();
    let mut trust = super::PluginHookTrustStore::load_or_default(&super::plugin_hook_trust_file(&config_dir)).unwrap();
    trust.trust_hook("host-hook", &hooks[0]).unwrap();
    let mut config = crate::config::Config::default();
    config.workspace_dir = workspace;
    config.config_path = config_dir.join("config.json");
    config.hooks.enabled = true;
    config.hooks.plugins.enabled = true;
    let handlers = super::trusted_plugin_hook_handlers(&config).unwrap();
    assert_eq!(handlers.len(), 1);
    let mut runner = crate::hooks::new_hook_runner();
    for handler in handlers { runner.register(handler); }
    assert!(matches!(runner.run_before_tool_call("shell".into(), serde_json::json!({})).await, crate::hooks::HookResult::Cancel(_)));
}
