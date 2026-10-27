//! 验证受信插件钩子的注册、执行、超时与诊断脱敏边界。

use super::{
    PluginDiscoveryOptions, PluginHookTrustStore, discover_plugins, load_plugin_hooks_manifest,
    plugin_hook_trust_file, trusted_plugin_hook_handlers,
};
use crate::app::agent::config::Config;
use crate::app::agent::hooks::{HookResult, HookRunner};
use crate::app::agent::tools::ToolResult;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::subscriber::Interest;
use vw_config_types::plugins::PluginsConfig;

// 夹具先写入精确钩子摘要再注册，用来区分受信执行与未审查跳过。
fn register_trusted_plugin_hooks(
    runner: &mut HookRunner,
    config: &Config,
) -> anyhow::Result<usize> {
    let handlers = trusted_plugin_hook_handlers(config)?;
    let registered = handlers.len();
    for handler in handlers {
        runner.register(handler);
    }
    Ok(registered)
}

fn write_json(path: &Path, value: Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("json parent");
    }
    fs::write(path, serde_json::to_vec_pretty(&value).expect("json bytes")).expect("write json");
}

fn write_plugin_manifest(plugin_root: &Path, name: &str) {
    write_json(
        &plugin_root.join(".vibewindow-plugin/plugin.json"),
        json!({
            "name": name,
            "version": "0.1.0",
            "description": format!("{name} test plugin"),
            "hooks": "./hooks/hooks.json",
        }),
    );
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

fn test_config(workspace_dir: PathBuf, config_dir: PathBuf) -> Config {
    let mut config = Config::default();
    config.workspace_dir = workspace_dir;
    config.config_path = config_dir.join("vibewindow.json");
    config.hooks.enabled = true;
    config.hooks.builtin.command_logger = false;
    config.hooks.plugins.enabled = true;
    config.plugins = PluginsConfig::default();
    config
}

fn trust_plugin_hook(
    config_dir: &Path,
    plugin_root: &Path,
    workspace_dir: &Path,
    plugin_name: &str,
) {
    let hooks_path = plugin_root.join("hooks/hooks.json");
    let hooks =
        load_plugin_hooks_manifest(&hooks_path, plugin_root, workspace_dir).expect("hooks parse");
    let trust_path = plugin_hook_trust_file(config_dir);
    let mut store = PluginHookTrustStore::load_or_default(&trust_path).expect("trust store");
    store.trust_hook(plugin_name, &hooks[0]).expect("trust hook");
}

fn discovered_enabled_names(config: &Config) -> Vec<String> {
    let config_dir = config.config_path.parent().expect("config dir").to_path_buf();
    let options = PluginDiscoveryOptions::new(
        config.workspace_dir.clone(),
        config_dir,
        config.plugins.clone(),
    );
    discover_plugins(&options)
        .expect("discover plugins")
        .enabled
        .into_iter()
        .map(|plugin| plugin.manifest.name)
        .collect()
}

#[derive(Clone, Default)]
struct RecordingSubscriber {
    events: Arc<Mutex<Vec<tracing::Level>>>,
}

impl RecordingSubscriber {
    fn levels(&self) -> Vec<tracing::Level> {
        self.events.lock().expect("recorded events").clone()
    }
}

impl tracing::Subscriber for RecordingSubscriber {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        metadata.target() == "vw_agent::plugins::hook_handler"
    }

    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::Id {
        tracing::Id::from_u64(1)
    }

    fn record(&self, _span: &tracing::Id, _values: &tracing::span::Record<'_>) {}

    fn record_follows_from(&self, _span: &tracing::Id, _follows: &tracing::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        self.events.lock().expect("record event").push(*event.metadata().level());
    }

    fn enter(&self, _span: &tracing::Id) {}

    fn exit(&self, _span: &tracing::Id) {}

    fn register_callsite(&self, metadata: &'static tracing::Metadata<'static>) -> Interest {
        if metadata.target() == "vw_agent::plugins::hook_handler" {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(tracing::metadata::LevelFilter::TRACE)
    }
}

#[cfg(unix)]
#[tokio::test]
async fn trusted_plugin_before_tool_call_can_cancel() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/trusted-plugin");
    let script_path = plugin_root.join("scripts/block.sh");
    write_plugin_manifest(&plugin_root, "trusted-plugin");
    write_script(
        &script_path,
        "#!/bin/sh\ncat >/dev/null\necho 'blocked by trusted plugin' >&2\nexit 23\n",
    );
    write_json(
        &plugin_root.join("hooks/hooks.json"),
        json!({
            "hooks": [{
                "event": "before_tool_call",
                "command": script_path.display().to_string(),
                "timeout_secs": 2,
                "working_dir": "./scripts"
            }]
        }),
    );
    let config = test_config(workspace_dir.clone(), config_dir.clone());
    assert_eq!(discovered_enabled_names(&config), vec!["trusted-plugin"]);
    trust_plugin_hook(&config_dir, &plugin_root, &workspace_dir, "trusted-plugin");

    let mut runner = HookRunner::new();
    let registered = register_trusted_plugin_hooks(&mut runner, &config).expect("register hooks");
    let result = runner
        .run_before_tool_call(
            "shell".to_string(),
            json!({"command": "echo hi", "api_key": "secret-value"}),
        )
        .await;

    println!("trusted before_tool_call registration count: {registered}");
    let HookResult::Cancel(reason) = result else {
        panic!("trusted plugin hook should cancel tool call");
    };
    println!("trusted before_tool_call cancel reason: {reason}");
    assert_eq!(registered, 1);
    assert!(reason.contains("trusted-plugin"), "{reason}");
    assert!(reason.contains("before_tool_call"), "{reason}");
    assert!(reason.contains("exit status 23"), "{reason}");
    assert!(reason.contains("blocked by trusted plugin"), "{reason}");
    assert!(!reason.contains("secret-value"), "{reason}");
    assert!(!reason.contains("api_key"), "{reason}");
}

#[cfg(unix)]
#[tokio::test]
async fn trusted_plugin_after_tool_call_receives_json_payload() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/trusted-plugin");
    let script_path = plugin_root.join("scripts/capture.sh");
    let payload_path = dir.path().join("after-payload.json");
    write_plugin_manifest(&plugin_root, "trusted-plugin");
    write_script(&script_path, "#!/bin/sh\ncat > \"$1\"\n");
    write_json(
        &plugin_root.join("hooks/hooks.json"),
        json!({
            "hooks": [{
                "event": "after_tool_call",
                "command": script_path.display().to_string(),
                "args": [payload_path.display().to_string()],
                "timeout_secs": 2,
                "working_dir": "./scripts"
            }]
        }),
    );
    let config = test_config(workspace_dir.clone(), config_dir.clone());
    trust_plugin_hook(&config_dir, &plugin_root, &workspace_dir, "trusted-plugin");

    let mut runner = HookRunner::new();
    let registered = register_trusted_plugin_hooks(&mut runner, &config).expect("register hooks");
    let tool_result =
        ToolResult { success: true, output: "lookup complete".to_string(), error: None };
    runner.fire_after_tool_call("lookup", &tool_result, Duration::from_millis(42)).await;

    println!("trusted after_tool_call registration count: {registered}");
    assert_eq!(registered, 1);
    let payload: Value = serde_json::from_slice(&fs::read(&payload_path).expect("payload file"))
        .expect("payload json");
    println!("trusted after_tool_call payload: {payload}");
    assert_eq!(payload["event"], "after_tool_call");
    assert_eq!(payload["tool"]["name"], "lookup");
    assert_eq!(payload["result"]["success"], true);
    assert_eq!(payload["result"]["output"], "lookup complete");
    assert_eq!(payload["duration_ms"], 42);
}

#[cfg(unix)]
#[tokio::test]
async fn trusted_plugin_before_tool_call_reports_spawn_failure() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/spawn-plugin");
    write_plugin_manifest(&plugin_root, "spawn-plugin");
    fs::create_dir_all(plugin_root.join("scripts")).expect("scripts dir");
    write_json(
        &plugin_root.join("hooks/hooks.json"),
        json!({
            "hooks": [{
                "event": "before_tool_call",
                "command": "definitely-missing-vw-hook-command",
                "timeout_secs": 2,
                "working_dir": "./scripts"
            }]
        }),
    );
    let config = test_config(workspace_dir.clone(), config_dir.clone());
    trust_plugin_hook(&config_dir, &plugin_root, &workspace_dir, "spawn-plugin");

    let mut runner = HookRunner::new();
    assert_eq!(register_trusted_plugin_hooks(&mut runner, &config).expect("register hooks"), 1);
    let result = runner.run_before_tool_call("shell".to_string(), json!({})).await;

    let HookResult::Cancel(reason) = result else {
        panic!("spawn failure should cancel tool call");
    };
    assert!(reason.contains("failed to spawn plugin hook spawn-plugin:before_tool_call"));
}

#[cfg(unix)]
#[tokio::test]
async fn trusted_plugin_before_tool_call_reports_timeout() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/timeout-plugin");
    let script_path = plugin_root.join("scripts/sleep.sh");
    write_plugin_manifest(&plugin_root, "timeout-plugin");
    write_script(&script_path, "#!/bin/sh\nsleep 2\n");
    write_json(
        &plugin_root.join("hooks/hooks.json"),
        json!({
            "hooks": [{
                "event": "before_tool_call",
                "command": script_path.display().to_string(),
                "timeout_secs": 1,
                "working_dir": "./scripts"
            }]
        }),
    );
    let config = test_config(workspace_dir.clone(), config_dir.clone());
    trust_plugin_hook(&config_dir, &plugin_root, &workspace_dir, "timeout-plugin");

    let mut runner = HookRunner::new();
    assert_eq!(register_trusted_plugin_hooks(&mut runner, &config).expect("register hooks"), 1);
    let result = runner.run_before_tool_call("shell".to_string(), json!({})).await;

    let HookResult::Cancel(reason) = result else {
        panic!("timeout should cancel tool call");
    };
    assert!(reason.contains("timeout-plugin:before_tool_call timed out after 1s"));
}

#[cfg(unix)]
#[tokio::test]
// 诊断输出同时覆盖脱敏和截断，防止钩子把秘密或超大载荷写入日志。
async fn trusted_plugin_before_tool_call_redacts_and_truncates_stdout_diagnostic() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/stdout-plugin");
    let script_path = plugin_root.join("scripts/stdout.sh");
    write_plugin_manifest(&plugin_root, "stdout-plugin");
    write_script(
        &script_path,
        "#!/bin/sh\nprintf 'token=secret-value\\n'\nprintf '%*s\\n' 600 '' | tr ' ' x\nexit 9\n",
    );
    write_json(
        &plugin_root.join("hooks/hooks.json"),
        json!({
            "hooks": [{
                "event": "before_tool_call",
                "command": script_path.display().to_string(),
                "timeout_secs": 2,
                "working_dir": "./scripts"
            }]
        }),
    );
    let config = test_config(workspace_dir.clone(), config_dir.clone());
    trust_plugin_hook(&config_dir, &plugin_root, &workspace_dir, "stdout-plugin");

    let mut runner = HookRunner::new();
    assert_eq!(register_trusted_plugin_hooks(&mut runner, &config).expect("register hooks"), 1);
    let result = runner.run_before_tool_call("shell".to_string(), json!({})).await;

    let HookResult::Cancel(reason) = result else {
        panic!("stdout failure should cancel tool call");
    };
    println!("stdout diagnostic cancel reason: {reason}");
    assert!(reason.contains("stdout-plugin:before_tool_call"));
    assert!(reason.contains("stdout: [redacted sensitive hook output line]"));
    assert!(reason.contains("...[truncated]"));
    assert!(!reason.contains("secret-value"));
}

#[cfg(unix)]
#[tokio::test]
async fn trusted_plugin_untrusted_hook_is_not_registered() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/untrusted-plugin");
    let script_path = plugin_root.join("scripts/write-marker.sh");
    let marker_path = dir.path().join("untrusted-ran");
    write_plugin_manifest(&plugin_root, "untrusted-plugin");
    write_script(&script_path, "#!/bin/sh\ntouch \"$1\"\n");
    write_json(
        &plugin_root.join("hooks/hooks.json"),
        json!({
            "hooks": [{
                "event": "before_tool_call",
                "command": script_path.display().to_string(),
                "args": [marker_path.display().to_string()],
                "timeout_secs": 2,
                "working_dir": "./scripts"
            }]
        }),
    );
    let config = test_config(workspace_dir, config_dir);

    let mut runner = HookRunner::new();
    let registered = register_trusted_plugin_hooks(&mut runner, &config).expect("register hooks");
    let result = runner.run_before_tool_call("shell".to_string(), json!({})).await;

    println!("untrusted plugin hook registration count: {registered}");
    assert_eq!(registered, 0);
    assert!(matches!(result, HookResult::Continue((_, _))));
    assert!(!marker_path.exists(), "untrusted hook script must not execute");
}

#[test]
fn plugin_hook_manifest_load_failure_reports_plugin_and_path_context() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/broken-plugin");
    write_plugin_manifest(&plugin_root, "broken-plugin");
    fs::create_dir_all(plugin_root.join("hooks")).expect("hooks dir");
    fs::write(plugin_root.join("hooks/hooks.json"), "{not json").expect("broken hooks");
    let config = test_config(workspace_dir, config_dir);
    let mut runner = HookRunner::new();

    let error = register_trusted_plugin_hooks(&mut runner, &config)
        .expect_err("broken hooks should fail registration")
        .to_string();

    assert!(error.contains("failed to load hooks for plugin 'broken-plugin'"));
    assert!(error.contains("hooks.json"));
}

#[tokio::test]
async fn untrusted_plugin_hook_skip_does_not_warn() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/untrusted-plugin");
    write_plugin_manifest(&plugin_root, "untrusted-plugin");
    fs::create_dir_all(plugin_root.join("scripts")).expect("scripts dir");
    write_json(
        &plugin_root.join("hooks/hooks.json"),
        json!({
            "hooks": [{
                "event": "before_tool_call",
                "command": "noop",
                "timeout_secs": 2,
                "working_dir": "./scripts"
            }]
        }),
    );
    let config = test_config(workspace_dir, config_dir);
    let subscriber = RecordingSubscriber::default();

    let mut runner = HookRunner::new();
    let registered = tracing::subscriber::with_default(subscriber.clone(), || {
        register_trusted_plugin_hooks(&mut runner, &config).expect("register hooks")
    });
    let levels = subscriber.levels();

    println!("untrusted plugin hook skip levels: {levels:?}");
    assert_eq!(registered, 0);
    assert!(levels.iter().any(|level| *level == tracing::Level::DEBUG));
    assert!(!levels.iter().any(|level| *level == tracing::Level::WARN));
}

#[test]
fn disabled_plugin_hooks_skip_discovery() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut config = test_config(dir.path().join("missing-workspace"), dir.path().join("config"));
    config.hooks.plugins.enabled = false;
    let mut runner = HookRunner::new();

    let registered =
        register_trusted_plugin_hooks(&mut runner, &config).expect("disabled hooks skip cleanly");

    assert_eq!(registered, 0);
}

#[test]
fn trusted_unsupported_plugin_hook_is_not_registered() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/trusted-plugin");
    write_plugin_manifest(&plugin_root, "trusted-plugin");
    fs::create_dir_all(plugin_root.join("scripts")).expect("scripts dir");
    write_json(
        &plugin_root.join("hooks/hooks.json"),
        json!({
            "hooks": [{
                "event": "user_prompt_submit",
                "command": "never-executed",
                "working_dir": "./scripts"
            }]
        }),
    );
    let config = test_config(workspace_dir.clone(), config_dir.clone());
    trust_plugin_hook(&config_dir, &plugin_root, &workspace_dir, "trusted-plugin");
    let subscriber = RecordingSubscriber::default();
    let mut runner = HookRunner::new();

    let registered = tracing::subscriber::with_default(subscriber.clone(), || {
        register_trusted_plugin_hooks(&mut runner, &config).expect("register hooks")
    });

    assert_eq!(registered, 0);
    assert!(subscriber.levels().iter().any(|level| *level == tracing::Level::WARN));
}

#[cfg(unix)]
#[tokio::test]
async fn trusted_plugin_before_llm_call_rejects_before_model_request() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    let config_dir = dir.path().join("config");
    let plugin = workspace.join(".vibewindow/plugins/model-budget");
    let script = plugin.join("scripts/guard.sh");
    write_plugin_manifest(&plugin, "model-budget");
    write_script(&script, "#!/bin/sh\ncat >/dev/null\nexit 23\n");
    write_json(
        &plugin.join("hooks/hooks.json"),
        json!({"hooks": [{
            "event": "before_llm_call", "command": script.display().to_string(),
            "timeout_secs": 2, "working_dir": "./scripts"
        }]}),
    );
    trust_plugin_hook(&config_dir, &plugin, &workspace, "model-budget");
    let config = test_config(workspace, config_dir);
    let handlers = trusted_plugin_hook_handlers(&config).expect("handlers");
    assert_eq!(handlers.len(), 1);
    let result = handlers[0].before_llm_call(Vec::new(), "local-test".into()).await;
    assert!(matches!(result, HookResult::Cancel(_)));
}
