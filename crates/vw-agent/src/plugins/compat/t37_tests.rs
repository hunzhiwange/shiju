use super::hook_permission::decision;
use serde_json::json;
#[test]
fn codex_plugin_compat_t37_rejects_invalid_input() {
    assert!(decision(&json!({"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow","updatedPermissions":[]}}})).is_err());
}
#[test]
fn codex_plugin_compat_t37_preserves_isolation() {
    assert!(decision(&json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","decision":{"behavior":"allow"}}})).is_err());
    assert_eq!(decision(&json!({})).unwrap(), None);
}
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
        let mut runner = crate::hooks::HookRunner::new();
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
async fn codex_plugin_compat_t37_denial_wins_and_recursion_fails_closed() {
    struct Verdict(bool);
    #[async_trait::async_trait]
    impl crate::hooks::HookHandler for Verdict {
        fn name(&self) -> &str {
            "fixture-verdict"
        }
        async fn permission_request(
            &self,
            _: &str,
            _: &serde_json::Value,
        ) -> anyhow::Result<Option<bool>> {
            Ok(Some(self.0))
        }
    }
    let mut runner = crate::hooks::HookRunner::new();
    runner.register(Box::new(Verdict(true)));
    runner.register(Box::new(Verdict(false)));
    assert_eq!(runner.run_permission_request("marker", &json!({})).await.unwrap(), Some(false));
    struct Recursive;
    #[async_trait::async_trait]
    impl crate::hooks::HookHandler for Recursive {
        fn name(&self) -> &str {
            "fixture-recursive"
        }
        async fn permission_request(
            &self,
            name: &str,
            input: &serde_json::Value,
        ) -> anyhow::Result<Option<bool>> {
            crate::hooks::HookRunner::new().run_permission_request(name, input).await
        }
    }
    let mut runner = crate::hooks::HookRunner::new();
    runner.register(Box::new(Recursive));
    assert!(
        runner
            .run_permission_request("marker", &json!({}))
            .await
            .unwrap_err()
            .to_string()
            .contains("recursive")
    );
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
    let mut runner = crate::hooks::HookRunner::new();
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
