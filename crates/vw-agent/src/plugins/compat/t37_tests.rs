use serde_json::json;

#[tokio::test]
async fn codex_plugin_compat_t37_preserves_host_permission_denial() {
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let manifest: CodexHooksManifest = serde_json::from_value(json!({"hooks":{"PermissionRequest":[{"hooks":[{"type":"command","command":"cat >/dev/null; printf '%s' '{\"hookSpecificOutput\":{\"hookEventName\":\"PermissionRequest\",\"decision\":{\"behavior\":\"deny\"}}}'"}]}]}})).unwrap();
    let mut trust =
        PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
    assert!(
        codex_handlers("fixture", root.path(), &manifest, &trust).is_ok(),
        "permission request must be admitted"
    );
}

#[tokio::test]
async fn codex_plugin_compat_t37_runs_real_command_in_own_turn() {
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    use crate::tools::{ToolUseContext, scope_tool_use_context};
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    for behavior in ["allow", "deny"] {
        let output = json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":behavior}}});
        let manifest: CodexHooksManifest = serde_json::from_value(json!({"hooks":{"PermissionRequest":[{"matcher":"marker","hooks":[{"type":"command","command":format!("cat > '{}'; printf '%s' '{}'", state.path().join("input").display(), output)}]}]}})).unwrap();
        let mut trust =
            PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
        trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
        let mut runner = crate::hooks::new_hook_runner();
        for handler in codex_handlers("fixture", root.path(), &manifest, &trust).unwrap() {
            runner.register(handler);
        }
        let context = Arc::new(
            ToolUseContext::new("t37", Some(root.path().to_string_lossy().into_owned()))
                .with_turn_id("turn")
                .with_tool_call_id("call")
                .with_model("model"),
        );
        let result = scope_tool_use_context(
            context,
            runner.run_permission_request("marker", &json!({"value":1})),
        )
        .await
        .unwrap();
        assert_eq!(result, Some(behavior == "allow"));
        let input: serde_json::Value =
            serde_json::from_slice(&std::fs::read(state.path().join("input")).unwrap()).unwrap();
        assert_eq!(input["session_id"], "t37");
        assert_eq!(input["turn_id"], "turn");
        assert_eq!(input["tool_input"], json!({"value":1}));
        assert_eq!(runner.run_permission_request("other", &json!({})).await.unwrap(), None);
        assert!(runner.run_permission_request("marker", &json!({})).await.is_err());
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn codex_plugin_compat_t37_real_command_timeout_fails_closed() {
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    use crate::tools::{ToolUseContext, scope_tool_use_context};
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let manifest: CodexHooksManifest = serde_json::from_value(json!({"hooks":{"PermissionRequest":[{"hooks":[{"type":"command","command":"exec sleep 5","timeout":1}]}]}})).unwrap();
    let mut trust =
        PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
    let mut runner = crate::hooks::new_hook_runner();
    for handler in codex_handlers("fixture", root.path(), &manifest, &trust).unwrap() {
        runner.register(handler);
    }
    let context = Arc::new(
        ToolUseContext::new("t37", Some(root.path().to_string_lossy().into_owned()))
            .with_turn_id("turn")
            .with_tool_call_id("call")
            .with_model("model"),
    );
    assert!(
        scope_tool_use_context(context, runner.run_permission_request("marker", &json!({})))
            .await
            .is_err()
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
