use super::hook_post_tool::validate;
use serde_json::json;
#[test]
fn codex_plugin_compat_t38_rejects_invalid_input() {
    assert!(validate(&json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","additionalContext":"wrong turn"}})).is_err());
    assert!(
        validate(
            &json!({"hookSpecificOutput":{"hookEventName":"PostToolUse","updatedMCPToolOutput":{}}})
        )
        .is_err()
    );
}
#[test]
fn codex_plugin_compat_t38_preserves_isolation() {
    assert!(validate(&json!({})).is_ok());
    assert!(crate::tools::ToolUseContext::default().take_hook_contexts().is_empty());
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
async fn codex_plugin_compat_t38_delivers_result_context_once() {
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    use crate::tools::{ToolUseContext, execute_tool_from_registry};
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let marker = state.path().join("marker");
    let tools: Vec<Box<dyn crate::tools::Tool>> = vec![Box::new(Marker(marker.clone()))];
    for decision in ["allow", "deny"] {
        let output = json!({"hookSpecificOutput":{"hookEventName":"PostToolUse","permissionDecision":decision,"permissionDecisionReason":"fixture denied","updatedInput":{"value":"rewritten"},"additionalContext":"fixture context"}});
        let manifest: CodexHooksManifest = serde_json::from_value(json!({"hooks":{"PostToolUse":[{"hooks":[{"type":"command","command":format!("cat >/dev/null; printf '%s' '{}'", output)}]}]}})).unwrap();
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
            result.unwrap();
            assert!(marker.exists());
            assert!(
                context.take_hook_contexts().iter().any(|text| text.contains("fixture denied"))
            );
        } else {
            result.unwrap();
            assert_eq!(std::fs::read_to_string(&marker).unwrap(), r#"{"value":"original"}"#);
            assert_eq!(context.take_hook_contexts(), vec!["fixture context"]);
        }
        assert!(ToolUseContext::default().take_hook_contexts().is_empty());
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
