use super::hook_prompt::{submit, validate};
use crate::{hooks::HookRunner, tools::ToolUseContext};
use serde_json::json;
use std::sync::Arc;

fn fixture(root: &std::path::Path, state: &std::path::Path, command: &str) -> Arc<ToolUseContext> {
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    let manifest: CodexHooksManifest = serde_json::from_value(json!({"hooks":{"UserPromptSubmit":[{"matcher":"never-matches", "hooks":[{"type":"command","command":command}]}]}})).unwrap();
    let mut trust = PluginHookTrustStore::load_or_default(&state.join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root, &manifest).unwrap();
    let handlers = codex_handlers("fixture", root, &manifest, &trust);
    assert!(handlers.is_ok(), "UserPromptSubmit must be registered: {:?}", handlers.as_ref().err());
    let mut runner = HookRunner::new();
    for handler in handlers.unwrap() {
        runner.register(handler);
    }
    Arc::new(
        ToolUseContext::new("session", Some(root.to_string_lossy().into_owned()))
            .with_turn_id("turn")
            .with_model("fixture")
            .with_hook_runner(Arc::new(runner)),
    )
}
#[tokio::test]
async fn codex_plugin_compat_t40_blocks_before_provider_request() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let marker = state.path().join("input");
    let command = format!(
        "cat > '{}'; printf '%s' '{{\"decision\":\"block\",\"reason\":\"policy denied\"}}'",
        marker.display()
    );
    let ctx = fixture(root.path(), state.path(), &command);
    let mut provider_requests = Vec::new();
    let result = submit(ctx.clone(), "current input").await;
    if result.is_ok() {
        provider_requests.push("request");
    }
    assert!(result.unwrap_err().to_string().contains("policy denied"));
    assert!(provider_requests.is_empty());
    assert!(ctx.take_hook_contexts().is_empty());
    let input: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(marker).unwrap()).unwrap();
    assert_eq!(input["prompt"], "current input");
    assert_eq!(input["turn_id"], "turn");
    assert_eq!(input["hook_event_name"], "UserPromptSubmit");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
#[tokio::test]
async fn codex_plugin_compat_t40_rejects_invalid_input() {
    assert!(validate(&json!({"hook_event_name":"UserPromptSubmit","prompt":42})).is_err());
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let marker = state.path().join("should-not-run");
    let ctx = fixture(root.path(), state.path(), &format!("touch '{}'", marker.display()));
    let invalid = Arc::new(ctx.as_ref().clone().with_turn_id(""));
    assert!(submit(invalid, "input").await.is_err());
    assert!(!marker.exists());
}
#[tokio::test]
async fn codex_plugin_compat_t40_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let marker = state.path().join("events");
    let command = format!(
        "cat >> '{}'; printf '\\n' >> '{}'; printf '%s' '{{\"hookSpecificOutput\":{{\"hookEventName\":\"UserPromptSubmit\",\"additionalContext\":\"scoped context\"}}}}'",
        marker.display(),
        marker.display()
    );
    let ctx = fixture(root.path(), state.path(), &command);
    submit(ctx.clone(), "new input").await.unwrap();
    assert_eq!(ctx.take_hook_contexts(), vec!["scoped context"]);
    assert!(ctx.take_hook_contexts().is_empty());
    let other = ToolUseContext::new("other", Some(root.path().to_string_lossy().into_owned()));
    assert!(other.take_hook_contexts().is_empty());
    assert_eq!(std::fs::read_to_string(marker).unwrap().lines().count(), 1);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn codex_plugin_compat_t40_validates_control_and_plain_context() {
    use super::hook_prompt::parse;
    assert!(
        parse(Some(2), b"", b"policy feedback")
            .unwrap_err()
            .to_string()
            .contains("policy feedback")
    );
    assert!(
        parse(Some(0), br#"{"continue":false,"stopReason":"stop here"}"#, b"")
            .unwrap_err()
            .to_string()
            .contains("stop here")
    );
    assert!(parse(Some(0), br#"{"decision":"block"}"#, b"").is_err());
    assert!(
        parse(Some(0), br#"{"hookSpecificOutput":{"hookEventName":"PostToolUse"}}"#, b"").is_err()
    );
    assert!(parse(Some(0), b"{invalid", b"").is_err());
    assert_eq!(
        parse(Some(0), b"plain context", b"").unwrap()["hookSpecificOutput"]["additionalContext"],
        "plain context"
    );
    assert!(parse(Some(1), b"ignored", b"").unwrap().is_null());
}

#[test]
fn codex_plugin_compat_t40_requires_trusted_source() {
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let manifest: CodexHooksManifest = serde_json::from_value(
        json!({"hooks":{"UserPromptSubmit":[{"hooks":[{"type":"command","command":"exit 2"}]}]}}),
    )
    .unwrap();
    let trust = PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
    assert!(codex_handlers("fixture", root.path(), &manifest, &trust).unwrap().is_empty());
}
