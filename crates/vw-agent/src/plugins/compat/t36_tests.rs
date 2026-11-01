use super::hook_pre_tool::updated_input;
use serde_json::json;

#[test]
fn codex_plugin_compat_t36_rejects_invalid_input() {
    assert!(updated_input(&json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow","updatedInput":[]}})).is_err());
}

#[test]
fn codex_plugin_compat_t36_preserves_isolation() {
    let output = json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","updatedInput":{"path":"changed"}}});
    assert_eq!(updated_input(&output).unwrap(), None);
    assert_eq!(updated_input(&json!({})).unwrap(), None);
    let mut allowed = output.clone();
    allowed["hookSpecificOutput"]["permissionDecision"] = json!("allow");
    assert_eq!(updated_input(&allowed).unwrap(), Some(json!({"path":"changed"})));
    assert_eq!(output["hookSpecificOutput"]["updatedInput"]["path"], "changed");
}

struct Marker(std::path::PathBuf);
#[async_trait::async_trait]
impl crate::tools::Tool for Marker {
    fn name(&self) -> &str {
        "marker"
    }
    fn description(&self) -> &str {
        "Write a harmless fixture marker"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        json!({"type":"object"})
    }
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<crate::tools::ToolResult> {
        std::fs::write(&self.0, args.to_string())?;
        Ok(crate::tools::ToolResult { success: true, output: "written".into(), error: None })
    }
}

#[cfg(unix)]
#[tokio::test]
async fn codex_plugin_compat_t36_blocks_tool_before_side_effect() {
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    use crate::tools::{ToolUseContext, execute_tool_from_registry};
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let marker = state.path().join("marker");
    let tools: Vec<Box<dyn crate::tools::Tool>> = vec![Box::new(Marker(marker.clone()))];
    for decision in ["deny", "allow"] {
        let output = json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":decision,"permissionDecisionReason":"fixture denied","updatedInput":{"value":"rewritten"},"additionalContext":"fixture context"}});
        let manifest: CodexHooksManifest = serde_json::from_value(json!({"hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":format!("cat >/dev/null; printf '%s' '{}'", output)}]}]}})).unwrap();
        let mut trust =
            PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
        trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
        let mut runner = crate::hooks::HookRunner::new();
        for handler in codex_handlers("fixture", root.path(), &manifest, &trust).unwrap() {
            runner.register(handler);
        }
        let context = Arc::new(
            ToolUseContext::new("t36", Some(root.path().to_string_lossy().into_owned()))
                .with_turn_id("turn")
                .with_tool_call_id("call")
                .with_model("model")
                .with_hook_runner(Arc::new(runner)),
        );
        let result = execute_tool_from_registry(
            &tools,
            "marker",
            json!({"value":"original"}),
            context.clone(),
        )
        .await;
        if decision == "deny" {
            assert!(result.is_err());
            assert!(!marker.exists());
        } else {
            result.unwrap();
            assert_eq!(std::fs::read_to_string(&marker).unwrap(), r#"{"value":"rewritten"}"#);
            assert_eq!(context.take_hook_contexts(), vec!["fixture context"]);
        }
        assert!(ToolUseContext::default().take_hook_contexts().is_empty());
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
