use serde_json::json;

struct Marker(std::path::PathBuf);
#[async_trait::async_trait]
impl super::tools::Tool for Marker {
    fn name(&self) -> &str {
        "marker"
    }
    fn description(&self) -> &str {
        "Write a harmless fixture marker"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        json!({"type":"object"})
    }
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<super::tools::ToolResult> {
        std::fs::write(&self.0, args.to_string())?;
        Ok(super::tools::ToolResult { success: true, output: "written".into(), error: None })
    }
}

#[cfg(unix)]
#[tokio::test]
async fn codex_plugin_compat_t36_blocks_tool_before_side_effect() {
    use super::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    use super::tools::{ToolUseContext, execute_tool_from_registry};
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let marker = state.path().join("marker");
    let tools: Vec<Box<dyn super::tools::Tool>> = vec![Box::new(Marker(marker.clone()))];
    for decision in ["deny", "allow"] {
        let output = json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":decision,"permissionDecisionReason":"fixture denied","updatedInput":{"value":"rewritten"},"additionalContext":"fixture context"}});
        let manifest: CodexHooksManifest = serde_json::from_value(json!({"hooks":{"PreToolUse":[{"hooks":[{"type":"command","command":format!("cat >/dev/null; printf '%s' '{}'", output)}]}]}})).unwrap();
        let mut trust =
            PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
        trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
        let mut runner = super::hooks::new_hook_runner();
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
