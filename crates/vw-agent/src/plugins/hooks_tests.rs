use super::hooks::{PluginHookEvent, PluginHookMode, load_plugin_hooks_manifest};
use serde_json::json;
use std::fs;
use std::path::Path;

fn write_json(path: &Path, value: serde_json::Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("json parent");
    }
    fs::write(path, serde_json::to_vec_pretty(&value).expect("json bytes")).expect("write json");
}

#[test]
fn plugin_hook_events_and_modes_export_stable_names() {
    assert_eq!(PluginHookEvent::BeforeToolCall.as_str(), "before_tool_call");
    assert_eq!(PluginHookEvent::AfterToolCall.as_str(), "after_tool_call");
    assert_eq!(PluginHookEvent::BeforeLlmCall.as_str(), "before_llm_call");
    assert_eq!(PluginHookEvent::UserPromptSubmit.as_str(), "user_prompt_submit");
    assert_eq!(PluginHookMode::Blocking.as_str(), "blocking");
}

#[test]
fn plugin_hooks_parse_supported_events() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/hook-plugin");
    fs::create_dir_all(plugin_root.join("hooks")).expect("hooks dir");
    fs::create_dir_all(plugin_root.join("scripts")).expect("scripts dir");
    let hooks_path = plugin_root.join("hooks/hooks.json");
    write_json(
        &hooks_path,
        json!({
            "hooks": [
                {
                    "event": "before_tool_call",
                    "command": "lint-tool",
                    "args": ["--fast"],
                    "timeout_secs": 5,
                    "working_dir": "./scripts",
                    "mode": "blocking"
                },
                { "event": "after_tool_call", "command": "audit-tool" },
                { "event": "before_llm_call", "command": "prompt-audit" },
                { "event": "user_prompt_submit", "command": "prompt-submit" }
            ]
        }),
    );

    let hooks =
        load_plugin_hooks_manifest(&hooks_path, &plugin_root, &workspace_dir).expect("hooks parse");

    println!("plugin hooks events: {:?}", hooks.iter().map(|hook| hook.event).collect::<Vec<_>>());
    assert_eq!(hooks.len(), 4);
    assert_eq!(hooks[0].event, PluginHookEvent::BeforeToolCall);
    assert_eq!(hooks[1].event, PluginHookEvent::AfterToolCall);
    assert_eq!(hooks[2].event, PluginHookEvent::BeforeLlmCall);
    assert_eq!(hooks[3].event, PluginHookEvent::UserPromptSubmit);
    assert_eq!(hooks[0].command, "lint-tool");
    assert_eq!(hooks[0].args, vec!["--fast".to_string()]);
    assert_eq!(hooks[0].timeout_secs, Some(5));
    assert_eq!(
        hooks[0].working_dir,
        plugin_root.join("scripts").canonicalize().expect("canonical scripts")
    );
    assert_eq!(hooks[0].mode, PluginHookMode::Blocking);
}

#[test]
fn plugin_hooks_accept_absolute_workspace_working_dir() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/hook-plugin");
    let workspace_scripts = workspace_dir.join("scripts");
    fs::create_dir_all(&workspace_scripts).expect("workspace scripts dir");
    let hooks_path = plugin_root.join("hooks/hooks.json");
    write_json(
        &hooks_path,
        json!({
            "hooks": [{
                "event": "before_tool_call",
                "command": "workspace-lint",
                "working_dir": workspace_scripts
            }]
        }),
    );

    let hooks =
        load_plugin_hooks_manifest(&hooks_path, &plugin_root, &workspace_dir).expect("hooks parse");

    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0].working_dir, workspace_scripts.canonicalize().expect("canonical scripts"));
}

#[test]
fn plugin_hooks_reject_missing_workspace_dir() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let missing_workspace = dir.path().join("missing-workspace");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/hook-plugin");
    let hooks_path = plugin_root.join("hooks/hooks.json");
    write_json(
        &hooks_path,
        json!({
            "hooks": [{
                "event": "before_tool_call",
                "command": "lint-tool"
            }]
        }),
    );

    let err = load_plugin_hooks_manifest(&hooks_path, &plugin_root, &missing_workspace)
        .expect_err("missing workspace rejected")
        .to_string();

    assert!(err.contains("failed to canonicalize workspace"), "{err}");
    assert!(err.contains("missing-workspace"), "{err}");
}

#[test]
fn plugin_hooks_reject_manifest_outside_plugin_root() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/hook-plugin");
    fs::create_dir_all(&plugin_root).expect("plugin root");
    let outside_hooks_path = workspace_dir.join("outside-hooks.json");
    write_json(
        &outside_hooks_path,
        json!({
            "hooks": [{
                "event": "before_tool_call",
                "command": "lint-tool"
            }]
        }),
    );

    let err = load_plugin_hooks_manifest(&outside_hooks_path, &plugin_root, &workspace_dir)
        .expect_err("manifest path escape rejected")
        .to_string();

    assert!(err.contains("Plugin hooks manifest path escapes plugin root"), "{err}");
}

#[test]
fn plugin_hooks_reject_manifest_directory() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/hook-plugin");
    let hooks_dir = plugin_root.join("hooks");
    fs::create_dir_all(&hooks_dir).expect("hooks dir");

    let err = load_plugin_hooks_manifest(&hooks_dir, &plugin_root, &workspace_dir)
        .expect_err("manifest directory rejected")
        .to_string();

    assert!(err.contains("Plugin hooks manifest must be a file"), "{err}");
}

#[test]
fn plugin_hooks_reject_invalid_manifest_json() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/hook-plugin");
    let hooks_path = plugin_root.join("hooks/hooks.json");
    fs::create_dir_all(hooks_path.parent().expect("hooks parent")).expect("hooks dir");
    fs::write(&hooks_path, b"{ not json").expect("write invalid json");

    let err = load_plugin_hooks_manifest(&hooks_path, &plugin_root, &workspace_dir)
        .expect_err("invalid json rejected")
        .to_string();

    assert!(err.contains("failed to parse plugin hooks manifest"), "{err}");
}

#[cfg(unix)]
#[test]
fn plugin_hooks_report_unreadable_manifest() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/hook-plugin");
    let hooks_path = plugin_root.join("hooks/hooks.json");
    write_json(
        &hooks_path,
        json!({
            "hooks": [{
                "event": "before_tool_call",
                "command": "lint-tool"
            }]
        }),
    );
    fs::set_permissions(&hooks_path, fs::Permissions::from_mode(0o000))
        .expect("lock manifest permissions");

    let err = load_plugin_hooks_manifest(&hooks_path, &plugin_root, &workspace_dir)
        .expect_err("unreadable manifest rejected")
        .to_string();

    fs::set_permissions(&hooks_path, fs::Permissions::from_mode(0o600))
        .expect("restore manifest permissions");
    assert!(err.contains("failed to read plugin hooks manifest"), "{err}");
}

#[test]
fn plugin_hooks_reject_unknown_event() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/hook-plugin");
    let hooks_path = plugin_root.join("hooks/hooks.json");
    write_json(
        &hooks_path,
        json!({
            "hooks": [
                { "event": "after_everything", "command": "nope" }
            ]
        }),
    );

    let err = load_plugin_hooks_manifest(&hooks_path, &plugin_root, &workspace_dir)
        .expect_err("unknown event rejected")
        .to_string();

    assert!(err.contains("after_everything"), "{err}");
    assert!(err.contains("unsupported plugin hook event"), "{err}");
}

#[test]
fn plugin_hooks_reject_path_escape() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/hook-plugin");
    fs::create_dir_all(dir.path().join("outside")).expect("outside dir");
    let hooks_path = plugin_root.join("hooks/hooks.json");
    write_json(
        &hooks_path,
        json!({
            "hooks": [
                {
                    "event": "before_tool_call",
                    "command": "escape",
                    "working_dir": "../../../../outside"
                }
            ]
        }),
    );

    let err = load_plugin_hooks_manifest(&hooks_path, &plugin_root, &workspace_dir)
        .expect_err("path escape rejected")
        .to_string();

    assert!(err.contains("working_dir"), "{err}");
    assert!(err.contains("escapes plugin root and workspace"), "{err}");
}

#[test]
fn plugin_hooks_reject_file_working_dir() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/hook-plugin");
    let script_file = plugin_root.join("scripts/lint.sh");
    fs::create_dir_all(script_file.parent().expect("script parent")).expect("scripts dir");
    fs::write(&script_file, "#!/bin/sh\n").expect("script file");
    let hooks_path = plugin_root.join("hooks/hooks.json");
    write_json(
        &hooks_path,
        json!({
            "hooks": [{
                "event": "before_tool_call",
                "command": "lint-tool",
                "working_dir": "scripts/lint.sh"
            }]
        }),
    );

    let err = load_plugin_hooks_manifest(&hooks_path, &plugin_root, &workspace_dir)
        .expect_err("file working_dir rejected")
        .to_string();

    assert!(err.contains("plugin hook working_dir must be a directory"), "{err}");
    assert!(err.contains("scripts/lint.sh"), "{err}");
}

#[test]
fn plugin_hooks_accept_top_level_list_and_trim_values() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/hook-plugin");
    let hooks_path = plugin_root.join("hooks/hooks.json");
    write_json(
        &hooks_path,
        json!([{
            "event": " before_tool_call ",
            "command": "  lint-tool  ",
            "working_dir": " ",
            "mode": " "
        }]),
    );

    let hooks =
        load_plugin_hooks_manifest(&hooks_path, &plugin_root, &workspace_dir).expect("hooks parse");

    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0].event, PluginHookEvent::BeforeToolCall);
    assert_eq!(hooks[0].command, "lint-tool");
    assert_eq!(hooks[0].working_dir, plugin_root.canonicalize().expect("canonical root"));
    assert_eq!(hooks[0].mode, PluginHookMode::Blocking);
}

#[test]
fn plugin_hooks_reject_empty_command() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/hook-plugin");
    let hooks_path = plugin_root.join("hooks/hooks.json");
    write_json(&hooks_path, json!({"hooks": [{"event": "before_tool_call", "command": "  "}]}));

    let err = load_plugin_hooks_manifest(&hooks_path, &plugin_root, &workspace_dir)
        .expect_err("empty command rejected")
        .to_string();

    assert!(err.contains("command must not be empty"), "{err}");
}

#[test]
fn plugin_hooks_reject_unsupported_mode() {
    let dir = tempfile::tempdir().expect("temp dir");
    let workspace_dir = dir.path().join("workspace");
    let plugin_root = workspace_dir.join(".vibewindow/plugins/hook-plugin");
    let hooks_path = plugin_root.join("hooks/hooks.json");
    write_json(
        &hooks_path,
        json!({
            "hooks": [{
                "event": "before_tool_call",
                "command": "lint-tool",
                "mode": "background"
            }]
        }),
    );

    let err = load_plugin_hooks_manifest(&hooks_path, &plugin_root, &workspace_dir)
        .expect_err("unsupported mode rejected")
        .to_string();

    assert!(err.contains("unsupported plugin hook mode: background"), "{err}");
}
