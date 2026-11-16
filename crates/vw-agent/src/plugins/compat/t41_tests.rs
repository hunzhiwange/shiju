//! Shared fixture for actual host execution-loop boundary tests.
use crate::tools::ToolUseContext;
use serde_json::json;
use std::sync::Arc;

pub(crate) fn fixture(
    root: &std::path::Path,
    state: &std::path::Path,
    marker: &std::path::Path,
) -> Arc<ToolUseContext> {
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    let command = format!(
        "cat >> '{}'; printf '\\n' >> '{}'; printf '%s' '{{\"continue\":false,\"hookSpecificOutput\":{{\"additionalContext\":\"ignored\"}}}}'; exit 2",
        marker.display(),
        marker.display()
    );
    let manifest: CodexHooksManifest = serde_json::from_value(json!({"hooks":{
        "PreModelRequest":[{"hooks":[{"type":"command","command":command}]}],
        "PostModelRequest":[{"hooks":[{"type":"command","command":command}]}]}}))
    .unwrap();
    let mut trust = PluginHookTrustStore::load_or_default(&state.join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root, &manifest).unwrap();
    let handlers = codex_handlers("fixture", root, &manifest, &trust)
        .expect("model events must be executable");
    let mut runner = crate::hooks::new_hook_runner();
    for handler in handlers {
        runner.register(handler);
    }
    Arc::new(
        ToolUseContext::new("session", Some(root.to_string_lossy().into_owned()))
            .with_turn_id("turn")
            .with_model("fixture")
            .with_hook_runner(Arc::new(runner)),
    )
}
