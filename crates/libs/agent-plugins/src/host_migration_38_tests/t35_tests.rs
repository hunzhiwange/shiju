#[cfg(unix)]
#[tokio::test]
async fn codex_plugin_compat_t35_session_end_runs_async_declaration_synchronously() {
    use super::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let marker = state.path().join("finished");
    let manifest: CodexHooksManifest = serde_json::from_value(serde_json::json!({
        "hooks":{"SessionEnd":[{"hooks":[{"type":"command","async":true,
        "command":format!("cat >/dev/null; printf done > '{}'",marker.display())}]}]}
    }))
    .unwrap();
    let mut trust =
        PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
    let handlers = codex_handlers("fixture", root.path(), &manifest, &trust).unwrap();
    handlers[0].on_session_end("session", "cli").await;
    assert_eq!(std::fs::read_to_string(marker).unwrap(), "done");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[tokio::test]
async fn codex_plugin_compat_t35_async_command_cannot_revoke_decision() {
    use super::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let manifest: CodexHooksManifest = serde_json::from_value(serde_json::json!({
        "hooks":{"PreToolUse":[{"hooks":[{"type":"command","async":true,
        "command":"cat >/dev/null; printf '%s' '{\"continue\":false}'; exit 2"}]}]}
    }))
    .unwrap();
    let mut trust =
        PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
    let admitted = codex_handlers("fixture", root.path(), &manifest, &trust);
    assert!(admitted.is_ok(), "async commands must be admitted: {:?}", admitted.as_ref().err());
    let handlers = admitted.unwrap();
    assert!(
        matches!(handlers[0].before_tool_call("Read".into(), serde_json::json!({})).await, super::hooks::HookResult::Cancel(reason) if reason.contains("active session context"))
    );
    let context = std::sync::Arc::new(
        super::tools::ToolUseContext::default()
            .with_turn_id("turn")
            .with_tool_call_id("call")
            .with_model("model"),
    );
    super::tools::scope_tool_use_context(context.clone(), async {
        assert!(matches!(
            handlers[0].before_tool_call("Read".into(), serde_json::json!({})).await,
            super::hooks::HookResult::Continue(_)
        ));
        context.async_hooks.shutdown().await;
    })
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn codex_plugin_compat_t35_finished_turn_kills_real_process() {
    use super::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let started = state.path().join("started");
    let late = state.path().join("late");
    let manifest: CodexHooksManifest = serde_json::from_value(serde_json::json!({
        "hooks":{"PreToolUse":[{"hooks":[{"type":"command","async":true,
        "command":format!("cat >/dev/null; touch '{}'; sleep 1; touch '{}'; printf '%s' '{{\"hookSpecificOutput\":{{\"additionalContext\":\"late\"}}}}'",started.display(),late.display())}]}]}
    })).unwrap();
    let mut trust =
        PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
    let handlers = codex_handlers("fixture", root.path(), &manifest, &trust).unwrap();
    let context = std::sync::Arc::new(
        super::tools::ToolUseContext::default()
            .with_turn_id("finished")
            .with_tool_call_id("call")
            .with_model("model"),
    );
    super::tools::scope_tool_use_context(context.clone(), async {
        handlers[0].before_tool_call("Read".into(), serde_json::json!({})).await;
    })
    .await;
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !started.exists() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    context.async_hooks.shutdown().await;
    drop(context);
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    assert!(!late.exists(), "cancelled hook process escaped its turn");
    let next = super::tools::ToolUseContext::default().with_turn_id("next");
    assert!(next.take_hook_contexts().is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn codex_plugin_compat_t35_delivers_only_to_original_live_turn() {
    use super::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let manifest: CodexHooksManifest = serde_json::from_value(serde_json::json!({"hooks":{"PreToolUse":[{"hooks":[{"type":"command","async":true,"command":"cat >/dev/null; printf '%s' '{\"continue\":false,\"hookSpecificOutput\":{\"additionalContext\":\"owned context\"}}'"}]}]}})).unwrap();
    let mut trust =
        PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
    let handlers = codex_handlers("fixture", root.path(), &manifest, &trust).unwrap();
    let original =
        super::tools::ToolUseContext::default().with_turn_id("original").with_model("model");
    let next = original.clone().with_turn_id("next");
    let _turn = original.async_hooks.turn_guard();
    let tool = std::sync::Arc::new(original.clone().with_tool_call_id("call"));
    super::tools::scope_tool_use_context(tool, async {
        assert!(matches!(
            handlers[0].before_tool_call("Read".into(), serde_json::json!({})).await,
            super::hooks::HookResult::Continue(_)
        ));
    })
    .await;
    let delivered = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let result = original.take_hook_contexts();
            if !result.is_empty() {
                break result;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(delivered, vec!["owned context"]);
    assert!(next.take_hook_contexts().is_empty());
    original.async_hooks.shutdown().await;
}
