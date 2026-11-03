//! Actual host command selection, without pretending to execute another platform.
#[tokio::test]
async fn codex_plugin_compat_t64_executes_host_hook_command() {
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    use crate::tools::{ToolUseContext, scope_tool_use_context};
    use std::{fs, sync::Arc};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("插件 hook package");
    let state = temp.path().join("状态 state");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&state).unwrap();
    let manifest: CodexHooksManifest = serde_json::from_value(serde_json::json!({
        "hooks":{"PreToolUse":[{"hooks":[{
            "type":"command",
            "command":"printf unix > \"$PLUGIN_DATA/result.txt\"",
            "commandWindows":"echo windows> \"%PLUGIN_DATA%\\result.txt\""
        }]}]}
    }))
    .unwrap();
    let mut trust = PluginHookTrustStore::load_or_default(&state.join("trust.json")).unwrap();
    trust.trust_codex_hooks("platform-fixture", &root, &manifest).unwrap();
    let handlers = codex_handlers("platform-fixture", &root, &manifest, &trust).unwrap();
    assert_eq!(handlers.len(), 1);
    let context = Arc::new(
        ToolUseContext::default()
            .with_turn_id("platform-turn")
            .with_tool_call_id("platform-call")
            .with_model("fixture-model")
            .with_hook_input(serde_json::json!({})),
    );
    let result = scope_tool_use_context(
        context,
        handlers[0].before_tool_call("Read".into(), serde_json::json!({})),
    )
    .await;
    assert!(matches!(result, crate::hooks::HookResult::Continue(_)), "{result:?}");
    let data =
        fs::read_dir(state.join("plugin-hook-data")).unwrap().next().unwrap().unwrap().path();
    let output = fs::read_to_string(data.join("result.txt")).unwrap();
    assert_eq!(output.trim(), if cfg!(windows) { "windows" } else { "unix" });
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0, "state must stay outside package");
}
