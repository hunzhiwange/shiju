use super::tools::ToolUseContext;
use serde_json::json;
use std::sync::Arc;
pub(crate) fn fixture(
    root: &std::path::Path,
    state: &std::path::Path,
    session: &str,
) -> Arc<ToolUseContext> {
    fixture_output(root, state, session, "", "")
}
fn fixture_output(
    root: &std::path::Path,
    state: &std::path::Path,
    session: &str,
    pre: &str,
    post: &str,
) -> Arc<ToolUseContext> {
    use super::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    let command = format!(
        "cat >> '{}'; printf '\\n' >> '{}'",
        state.join("events").display(),
        state.join("events").display()
    );
    let manifest: CodexHooksManifest = serde_json::from_value(json!({"hooks":{
        "PreCompact":[{"hooks":[{"type":"command","command":format!("{command}; printf '%s' '{pre}'")}]}],
        "PostCompact":[{"hooks":[{"type":"command","command":format!("{command}; printf '%s' '{post}'")}]}]}}))
    .unwrap();
    let mut trust = PluginHookTrustStore::load_or_default(&state.join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root, &manifest).unwrap();
    let mut runner = super::hooks::new_hook_runner();
    for handler in codex_handlers("fixture", root, &manifest, &trust).unwrap() {
        runner.register(handler);
    }
    Arc::new(
        ToolUseContext::new(session, Some(root.to_string_lossy().into_owned()))
            .with_turn_id("turn")
            .with_model("fixture")
            .with_hook_runner(Arc::new(runner)),
    )
}
#[tokio::test]
async fn codex_plugin_compat_t42_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let ctx = fixture(root.path(), state.path(), "owner");
    super::tools::scope_tool_use_context(ctx.clone(), async {
        assert!(super::hook_compact::emit("other", "turn", "fixture", false, false).await.is_err());
    })
    .await;
    assert!(!state.path().join("events").exists());
    assert!(ctx.take_hook_contexts().is_empty());
}
