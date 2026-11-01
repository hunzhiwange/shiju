use crate::plugins::{
    CodexHookEvent, CodexHookHandler, PluginHookSource, PluginHooksManifest,
    load_plugin_hook_source, load_plugin_manifest,
};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

fn write(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../testdata/plugin_compat/t32/hooks.json")).unwrap()
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
fn codex_plugin_compat_t32_loads_all_baseline_hook_events() {
    let temp = tempfile::tempdir().unwrap();
    for portable in [false, true] {
        for inline in [false, true] {
            let root = temp.path().join(format!("{portable}-{inline}"));
            package(&root, portable, inline, &fixture());
            let plugin = load_plugin_manifest(&root).unwrap();
            assert_eq!(plugin.hooks.len(), 1);
            let parsed = load_plugin_hook_source(&plugin.hooks[0], &root, temp.path())
                .expect("Codex event objects must parse through the real plugin source");
            let PluginHooksManifest::Codex(parsed) = parsed else { panic!("expected Codex hooks") };
            assert_eq!(parsed.hooks.len(), 14);
            assert_eq!(parsed.description.as_deref(), Some("Synthetic hook declarations"));
            assert_eq!(
                serde_json::to_value(&parsed.hooks[&CodexHookEvent::PreToolUse][0].hooks[0])
                    .unwrap(),
                fixture()["hooks"]["PreToolUse"][0]["hooks"][0]
            );
            let group = &parsed.hooks[&CodexHookEvent::PreToolUse][0];
            assert_eq!(group.matcher.as_deref(), Some("Read|Write"));
            assert!(matches!(&group.hooks[1], CodexHookHandler::McpTool { server, tool, .. }
                if server == "public" && tool == "inspect"));
            let diagnostics = parsed.diagnostics();
            assert_eq!(diagnostics.len(), 3);
            for message in ["prompt", "agent", "SessionEnd MCP"] {
                assert!(diagnostics.iter().any(|d| d.message.contains(message)));
            }
        }
    }
}

#[test]
fn codex_plugin_compat_t32_rejects_invalid_input() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("package");
    fs::create_dir(&root).unwrap();
    for value in [
        json!({"hooks":{"NotAnEvent":[]}}),
        json!({"hooks":{"PreToolUse":{}}}),
        json!({"hooks":{"PreToolUse":[{"hooks":[{"type":"unknown"}]}]}}),
        json!({"hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":5}]}]}}),
        json!({"hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":"echo","timeout":-1}]}]}}),
        json!({"hooks":{"PreToolUse":[{"hooks":[{"type":"mcp_tool","server":"s","tool":"t","input":{"nested":[null]}}]}]}}),
        json!({"hooks":{"PreToolUse":[{"hooks":[{"type":"mcp_tool","server":"s","tool":"t","input":{"n":18446744073709551615u64}}]}]}}),
    ] {
        let source = PluginHookSource::Inline(value.as_object().unwrap().clone());
        assert!(load_plugin_hook_source(&source, &root, temp.path()).is_err(), "{value}");
    }
    let valid = json!({"hooks":{"PreToolUse":[{"hooks":[{"type":"mcp_tool","server":" ","tool":"t"},{"type":"command","command":" "}]}]}});
    let source = PluginHookSource::Inline(valid.as_object().unwrap().clone());
    let PluginHooksManifest::Codex(parsed) =
        load_plugin_hook_source(&source, &root, temp.path()).unwrap()
    else {
        panic!("Codex")
    };
    assert_eq!(parsed.diagnostics().len(), 2);
}

#[test]
fn codex_plugin_compat_t32_preserves_isolation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("package");
    package(&root, false, false, &fixture());
    let plugin = load_plugin_manifest(&root).unwrap();
    let path = root.join("hooks/hooks.json");
    let before = fs::read(&path).unwrap();
    let manifest_before = fs::read(&plugin.manifest_path).unwrap();
    for turn in ["turn-a", "turn-b"] {
        let workspace = temp.path().join(turn);
        fs::create_dir(&workspace).unwrap();
        let parsed = load_plugin_hook_source(&plugin.hooks[0], &root, &workspace).unwrap();
        assert!(matches!(parsed, PluginHooksManifest::Codex(_)));
        assert_eq!(fs::read_dir(&workspace).unwrap().count(), 0);
    }
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::read(&plugin.manifest_path).unwrap(), manifest_before);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
    let mut escaped = plugin.hooks[0].clone();
    let PluginHookSource::Path(ref mut path) = escaped else { panic!("path") };
    path.canonical_path = temp.path().join("outside.json");
    write(&path.canonical_path, &fixture());
    assert!(load_plugin_hook_source(&escaped, &root, temp.path()).is_err());
    // Legacy declarations continue through the same audited source entry point.
    write(
        &root.join("hooks/hooks.json"),
        &json!({"hooks":[{"event":"before_tool_call","command":"echo","args":["legacy"]}]}),
    );
    let PluginHooksManifest::Legacy(hooks) =
        load_plugin_hook_source(&plugin.hooks[0], &root, temp.path()).unwrap()
    else {
        panic!("legacy")
    };
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0].args, ["legacy"]);
    assert_eq!(hooks[0].working_dir, root.canonicalize().unwrap());
}

#[test]
fn codex_plugin_compat_t32_loads_defaults_and_preserves_explicit_empty_hooks() {
    let temp = tempfile::tempdir().unwrap();
    for portable in [false, true] {
        let root = temp.path().join(format!("default-{portable}"));
        package(&root, portable, false, &fixture());
        let path = if portable {
            root.join("plugin.json")
        } else {
            root.join(".codex-plugin/plugin.json")
        };
        let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        if portable {
            manifest.as_object_mut().unwrap().remove("extensions");
        } else {
            manifest.as_object_mut().unwrap().remove("hooks");
        }
        write(&path, &manifest);
        let plugin = load_plugin_manifest(&root).unwrap();
        assert_eq!(plugin.hooks.len(), 1);
        assert!(matches!(
            load_plugin_hook_source(&plugin.hooks[0], &root, temp.path()).unwrap(),
            PluginHooksManifest::Codex(_)
        ));
        if portable {
            manifest["extensions"] = json!({"com.openai":{"hooks":[]}});
        } else {
            manifest["hooks"] = json!([]);
        }
        write(&path, &manifest);
        assert!(load_plugin_manifest(&root).unwrap().hooks.is_empty());
    }
}

#[test]
fn codex_plugin_compat_t32_preserves_handler_defaults_and_alias() {
    let temp = tempfile::tempdir().unwrap();
    let value = json!({"hooks":{"PreToolUse":[{"hooks":[
        {"type":"command","command":"echo synthetic","command_windows":"echo windows"},
        {"type":"mcp_tool","server":"s","tool":"t"}
    ]}]}});
    let source = PluginHookSource::Inline(value.as_object().unwrap().clone());
    let PluginHooksManifest::Codex(parsed) =
        load_plugin_hook_source(&source, temp.path(), temp.path()).unwrap()
    else {
        panic!("Codex")
    };
    let handlers = &parsed.hooks[&CodexHookEvent::PreToolUse][0].hooks;
    assert!(matches!(&handlers[0], CodexHookHandler::Command {
        command_windows: Some(windows), timeout: None, r#async: false,
        status_message: None, additional_context_limit: None, ..
    } if windows == "echo windows"));
    assert!(matches!(&handlers[1], CodexHookHandler::McpTool {
        input, timeout: None, status_message: None, ..
    } if input.is_empty()));
    assert!(parsed.diagnostics().is_empty());
}

#[test]
fn codex_plugin_compat_t32_loads_source_arrays_with_explicit_precedence() {
    let temp = tempfile::tempdir().unwrap();
    for portable in [false, true] {
        for inline in [false, true] {
            let root = temp.path().join(format!("array-{portable}-{inline}"));
            package(&root, portable, inline, &fixture());
            let path = if portable {
                root.join("plugin.json")
            } else {
                root.join(".codex-plugin/plugin.json")
            };
            write(&root.join("selected.json"), &fixture());
            let declarations = if inline {
                json!([fixture(), fixture()])
            } else {
                json!(["./selected.json", "./selected.json"])
            };
            let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            if portable {
                manifest["extensions"]["com.openai"]["hooks"] = declarations;
            } else {
                manifest["hooks"] = declarations;
            }
            write(&path, &manifest);
            // A default file must never override explicit declarations or be parsed with them.
            fs::create_dir_all(root.join("hooks")).unwrap();
            fs::write(root.join("hooks/hooks.json"), b"invalid default").unwrap();
            let plugin = load_plugin_manifest(&root).unwrap();
            assert_eq!(plugin.hooks.len(), 2);
            for source in &plugin.hooks {
                let PluginHooksManifest::Codex(parsed) =
                    load_plugin_hook_source(source, &root, temp.path()).unwrap()
                else {
                    panic!("Codex")
                };
                assert_eq!(parsed.hooks.len(), 14);
            }
            #[cfg(unix)]
            {
                fs::remove_file(root.join("hooks/hooks.json")).unwrap();
                let outside = temp.path().join("outside-default.json");
                write(&outside, &fixture());
                std::os::unix::fs::symlink(&outside, root.join("hooks/hooks.json")).unwrap();
                assert_eq!(load_plugin_manifest(&root).unwrap().hooks.len(), 2);
            }
        }
    }
}

#[test]
fn codex_plugin_compat_t32_runtime_reports_execution_as_unavailable() {
    use crate::app::agent::config::Config;
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let root = workspace.join(".vibewindow/plugins/hook-fixture");
    package(&root, false, false, &fixture());
    let mut config = Config::default();
    config.workspace_dir = workspace;
    config.config_path = temp.path().join("config/vibewindow.json");
    config.hooks.enabled = true;
    config.hooks.plugins.enabled = true;
    let error = match crate::plugins::trusted_plugin_hook_handlers(&config) {
        Ok(_) => panic!("parsing must not imply execution support or trust"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("Codex hook execution is not supported"), "{error}");
    assert!(!temp.path().join("config").exists());
}
