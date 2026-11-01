//! Real-process regression for the synchronous Codex exit protocol.
#[cfg(unix)]
#[tokio::test]
async fn codex_plugin_compat_t34_interprets_command_exit_protocol() {
    let fixture_context = std::sync::Arc::new(
        crate::tools::ToolUseContext::default()
            .with_turn_id("fixture-turn")
            .with_tool_call_id("fixture-call")
            .with_model("fixture-model")
            .with_hook_input(serde_json::json!({})),
    );
    crate::tools::scope_tool_use_context(fixture_context, async {
        use crate::app::agent::hooks::HookResult;
        use crate::plugins::{
            CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers,
        };
        let root = tempfile::tempdir().unwrap();
        let state = tempfile::tempdir().unwrap();
        let manifest: CodexHooksManifest = serde_json::from_value(serde_json::json!({
            "hooks": {"PreToolUse": [{"hooks": [{"type": "command", "command": "exit 1"}]}]}
        }))
        .unwrap();
        let mut trust =
            PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
        trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
        let handlers = codex_handlers("fixture", root.path(), &manifest, &trust).unwrap();
        assert_eq!(handlers.len(), 1);
        let result = handlers[0].before_tool_call("Read".into(), serde_json::json!({})).await;
        assert!(
            matches!(result, HookResult::Continue(_)),
            "exit 1 is a nonblocking diagnostic, got {result:?}"
        );
    })
    .await;
}

#[test]
fn codex_plugin_compat_t34_rejects_invalid_input() {
    use super::hook_command::interpret;
    assert!(interpret(None, b"").is_err());
    assert!(
        interpret(Some(2), b"sensitive output").unwrap_err().to_string().contains("exit status 2")
    );
    assert!(interpret(Some(0), br#"{"continue":false}"#).is_err());
    assert!(
        interpret(Some(0), br#"{"hookSpecificOutput":{"permissionDecision":"deny"}}"#).is_err()
    );
    assert!(interpret(Some(0), b"plain output").is_ok());
}

#[test]
fn codex_plugin_compat_t34_preserves_isolation() {
    use super::hook_command::interpret;
    let secret = br#"{"continue":false,"stopReason":"private fixture"}"#;
    let error = interpret(Some(0), secret).unwrap_err().to_string();
    assert!(!error.contains("private fixture"));
    assert!(interpret(Some(0), b"{}").is_ok());
    // A nonblocking failure must not apply control effects from its stdout.
    assert!(interpret(Some(7), secret).is_ok());
    assert_eq!(secret, br#"{"continue":false,"stopReason":"private fixture"}"#);
}

#[cfg(unix)]
#[tokio::test]
async fn codex_plugin_compat_t34_applies_updated_input() {
    let fixture_context = std::sync::Arc::new(
        crate::tools::ToolUseContext::default()
            .with_turn_id("fixture-turn")
            .with_tool_call_id("fixture-call")
            .with_model("fixture-model")
            .with_hook_input(serde_json::json!({})),
    );
    crate::tools::scope_tool_use_context(fixture_context, async {

    use crate::app::agent::hooks::HookResult;
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let manifest: CodexHooksManifest = serde_json::from_value(serde_json::json!({
        "hooks": {"PreToolUse": [{"hooks": [{"type":"command", "command": "printf '%s' '{\"hookSpecificOutput\":{\"hookEventName\":\"PreToolUse\",\"permissionDecision\":\"allow\",\"updatedInput\":{\"path\":\"updated\"}}}'"}]}]}
    })).unwrap();
    let mut trust =
        PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
    let handlers = codex_handlers("fixture", root.path(), &manifest, &trust).unwrap();
    match handlers[0].before_tool_call("Read".into(), serde_json::json!({"path":"original"})).await
    {
        HookResult::Continue((name, input)) => {
            assert_eq!(name, "Read");
            assert_eq!(input, serde_json::json!({"path":"updated"}));
        }
        other => panic!("unexpected hook result: {other:?}"),
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);

    }).await;
}

#[tokio::test]
async fn codex_plugin_compat_t34_spills_context_only_in_own_turn() {
    let fixture_context = std::sync::Arc::new(
        crate::tools::ToolUseContext::default()
            .with_turn_id("fixture-turn")
            .with_tool_call_id("fixture-call")
            .with_model("fixture-model")
            .with_hook_input(serde_json::json!({})),
    );
    crate::tools::scope_tool_use_context(fixture_context, async {
        use crate::tools::{ToolUseContext, scope_tool_use_context};
        let state = tempfile::tempdir().unwrap();
        let context = std::sync::Arc::new(
            ToolUseContext::new("fixture-session", None).with_turn_id("turn-a"),
        );
        let text = "context ".repeat(100);
        scope_tool_use_context(context.clone(), async {
            super::hook_command::retain_context(&text, Some(4), state.path()).unwrap();
        })
        .await;
        let visible = context.take_hook_contexts();
        assert_eq!(visible.len(), 1);
        let (_, path) = visible[0].split_once("Full hook output saved to: ").unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), text);
        let other = ToolUseContext::new("other-session", None).with_turn_id("turn-b");
        assert!(other.take_hook_contexts().is_empty());
        assert!(context.take_hook_contexts().is_empty());
        scope_tool_use_context(context.clone(), async {
            super::hook_command::retain_context(&text, Some(0), state.path()).unwrap();
        })
        .await;
        assert_eq!(context.take_hook_contexts(), vec![text]);
    })
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn codex_plugin_compat_t34_receives_host_input_and_environment() {
    let fixture_context = std::sync::Arc::new(
        crate::tools::ToolUseContext::default()
            .with_turn_id("fixture-turn")
            .with_tool_call_id("fixture-call")
            .with_model("fixture-model")
            .with_hook_input(serde_json::json!({})),
    );
    crate::tools::scope_tool_use_context(fixture_context, async {

    use crate::hooks::HookResult;
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    use crate::tools::{ToolUseContext, scope_tool_use_context};
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let manifest: CodexHooksManifest = serde_json::from_value(serde_json::json!({
        "hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":include_str!("../../../testdata/plugin_compat/t34/capture.sh")}]}]}
    })).unwrap();
    let mut trust =
        PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
    let handlers = codex_handlers("fixture", root.path(), &manifest, &trust).unwrap();
    let context = std::sync::Arc::new(
        ToolUseContext::new("session-a", Some(workspace.path().to_str().unwrap().into()))
            .with_turn_id("turn-a")
            .with_tool_call_id("tool-a")
            .with_model("fixture-model"),
    );
    let outcome = scope_tool_use_context(
        context,
        handlers[0].before_tool_call("Read".into(), serde_json::json!({"path":"fixture"})),
    )
    .await;
    assert!(matches!(outcome, HookResult::Continue(_)));
    let data_root = state.path().join("plugin-hook-data");
    let data = std::fs::read_dir(data_root).unwrap().next().unwrap().unwrap().path();
    let input: serde_json::Value =
        serde_json::from_slice(&std::fs::read(data.join("input.json")).unwrap()).unwrap();
    assert_eq!(input["session_id"], "session-a");
    assert_eq!(input["turn_id"], "turn-a");
    assert_eq!(input["tool_use_id"], "tool-a");
    assert_eq!(input["model"], "fixture-model");
    assert_eq!(input["permission_mode"], "default");
    assert_eq!(input["hook_event_name"], "PreToolUse");
    assert_eq!(input["tool_input"], serde_json::json!({"path":"fixture"}));
    let environment = std::fs::read_to_string(data.join("environment")).unwrap();
    let paths: Vec<_> = environment
        .lines()
        .map(|path| std::path::Path::new(path).canonicalize().unwrap())
        .collect();
    assert_eq!(
        paths,
        vec![root.path().canonicalize().unwrap(), workspace.path().canonicalize().unwrap()]
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);

    }).await;
}

#[cfg(unix)]
#[tokio::test]
async fn codex_plugin_compat_t34_bounds_command_runtime_and_output() {
    let fixture_context = std::sync::Arc::new(
        crate::tools::ToolUseContext::default()
            .with_turn_id("fixture-turn")
            .with_tool_call_id("fixture-call")
            .with_model("fixture-model")
            .with_hook_input(serde_json::json!({})),
    );
    crate::tools::scope_tool_use_context(fixture_context, async {
        use crate::hooks::HookResult;
        use crate::plugins::{
            CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers,
        };
        for (command, expected) in
            [("sleep 30", "timed out"), ("yes fixture", "output limit exceeded")]
        {
            let root = tempfile::tempdir().unwrap();
            let state = tempfile::tempdir().unwrap();
            let manifest: CodexHooksManifest = serde_json::from_value(serde_json::json!({
            "hooks":{"PreToolUse":[{"hooks":[{"type":"command", "timeout":1, "command":command}]}]}
        }))
        .unwrap();
            let mut trust =
                PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
            trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
            let handlers = codex_handlers("fixture", root.path(), &manifest, &trust).unwrap();
            let result = handlers[0].before_tool_call("Read".into(), serde_json::json!({})).await;
            assert!(
                matches!(result, HookResult::Cancel(ref reason) if reason.contains(expected)),
                "{result:?}"
            );
        }
    })
    .await;
}

#[test]
fn codex_plugin_compat_t34_rejects_missing_session_context() {
    let error = super::hook_command::bind_input(
        &mut serde_json::json!({}),
        crate::plugins::PluginHookEvent::BeforeToolCall,
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "Codex hook requires active session context");
}

#[cfg(unix)]
#[tokio::test]
async fn codex_plugin_compat_t34_model_hook_is_observation_only() {
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    use crate::tools::{ToolUseContext, scope_tool_use_context};
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let manifest: CodexHooksManifest = serde_json::from_value(serde_json::json!({
        "hooks":{"PreModelRequest":[{"hooks":[{"type":"command", "command":"exit 2"}]}]}
    }))
    .unwrap();
    let mut trust =
        PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
    let handlers = codex_handlers("fixture", root.path(), &manifest, &trust).unwrap();
    let context = std::sync::Arc::new(
        ToolUseContext::default().with_turn_id("turn-a").with_model("fixture-model"),
    );
    let result = scope_tool_use_context(
        context,
        handlers[0].before_llm_call(Vec::new(), "fixture-model".into()),
    )
    .await;
    assert!(matches!(result, crate::hooks::HookResult::Continue(_)));
}

#[cfg(unix)]
#[test]
fn codex_plugin_compat_t34_rejects_data_symlink_escape() {
    let host = tempfile::tempdir().unwrap();
    let package = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let host = host.path().canonicalize().unwrap();
    std::os::unix::fs::symlink(outside.path(), host.join("escape")).unwrap();
    assert!(
        super::hook_command::prepare_data_directory(&host.join("escape/child"), package.path())
            .is_err()
    );
    assert!(!outside.path().join("child").exists());
}
