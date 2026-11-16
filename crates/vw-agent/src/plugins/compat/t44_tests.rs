//! Shared fixture for actual host execution-loop boundary tests.
use crate::{hooks::HookRunner, tools::ToolUseContext};
use serde_json::json;
use std::sync::Arc;
pub(crate) fn fixture(
    root: &std::path::Path,
    state: &std::path::Path,
    turn: &str,
) -> Arc<ToolUseContext> {
    Arc::new(
        ToolUseContext::new("session", Some(root.to_string_lossy().into_owned()))
            .with_turn_id(turn)
            .with_model("fixture")
            .with_hook_runner(runner(root, state)),
    )
}
pub(crate) fn runner(root: &std::path::Path, state: &std::path::Path) -> Arc<HookRunner> {
    use crate::plugins::CodexHooksManifest;
    let capture = format!(
        "payload=$(cat); printf '%s\\n' \"$payload\" >> '{}'",
        state.join("events").display()
    );
    let manifest: CodexHooksManifest = serde_json::from_value(json!({"hooks":{
        "Stop":[{"hooks":[{"type":"command","command":format!("{capture}; case \"$payload\" in *'\"stop_hook_active\":true'*) ;; *) printf '%s' '{{\"decision\":\"block\",\"reason\":\"verify work\"}}' ;; esac")}]}],
        "Interrupt":[{"hooks":[{"type":"command","command":format!("{capture}; printf '%s' '{{\"continue\":false}}'")}]}]}})).unwrap();
    register(root, state, &manifest)
}
fn register(
    root: &std::path::Path,
    state: &std::path::Path,
    manifest: &crate::plugins::CodexHooksManifest,
) -> Arc<HookRunner> {
    use crate::plugins::{PluginHookTrustStore, hook_handler::codex_handlers};
    let mut trust = PluginHookTrustStore::load_or_default(&state.join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root, manifest).unwrap();
    let mut runner = crate::hooks::new_hook_runner();
    for handler in codex_handlers("fixture", root, manifest, &trust).unwrap() {
        runner.register(handler);
    }
    Arc::new(runner)
}
