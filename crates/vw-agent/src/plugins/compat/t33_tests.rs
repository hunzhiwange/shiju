use super::hook_manifest::{CodexHookEvent as E, CodexHooksManifest};
use super::hook_matcher::HookMatcher;
#[cfg(unix)]
use crate::app::agent::hooks::HookResult;
use crate::plugins::{PluginHookTrustStore, hook_handler::codex_handlers};
#[cfg(unix)]
use serde_json::json;

fn manifest() -> CodexHooksManifest {
    serde_json::from_str(include_str!("../../../testdata/plugin_compat/t33/hooks.json")).unwrap()
}
#[cfg(unix)]
#[tokio::test]
async fn codex_plugin_compat_t33_executes_only_matching_handlers() {
    let root = tempfile::tempdir().unwrap();
    let fixture_context = std::sync::Arc::new(
        crate::tools::ToolUseContext::new(
            "fixture-session",
            Some(root.path().to_str().unwrap().into()),
        )
        .with_turn_id("fixture-turn")
        .with_tool_call_id("fixture-call")
        .with_model("fixture-model")
        .with_hook_input(serde_json::json!({})),
    );
    crate::tools::scope_tool_use_context(fixture_context, async {
        let state = tempfile::tempdir().unwrap();
        let manifest = manifest();
        let mut trust =
            PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
        trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
        let handlers = codex_handlers("fixture", root.path(), &manifest, &trust).unwrap();
        assert_eq!(handlers.len(), 1);
        for handler in handlers {
            assert!(matches!(
                handler.before_tool_call("Write".into(), json!({})).await,
                HookResult::Continue(_)
            ));
        }
        assert_eq!(std::fs::read(root.path().join("result")).unwrap(), b"abbc");
        std::fs::write(root.path().join("result"), b"").unwrap();
        let mut after = manifest.clone();
        let groups = after.hooks.remove(&E::PreToolUse).unwrap();
        after.hooks.insert(E::PostToolUse, groups);
        trust.trust_codex_hooks("fixture", root.path(), &after).unwrap();
        let mut runner = crate::app::agent::hooks::HookRunner::new();
        for handler in codex_handlers("fixture", root.path(), &after, &trust).unwrap() {
            runner.register(handler);
        }
        let result = crate::app::agent::tools::ToolResult {
            success: true,
            output: String::new(),
            error: None,
        };
        runner.fire_after_tool_call("Write", &result, std::time::Duration::ZERO).await;
        assert_eq!(std::fs::read(root.path().join("result")).unwrap(), b"abbc");
    })
    .await;
}
#[test]
fn codex_plugin_compat_t33_rejects_invalid_input() {
    assert_eq!(
        HookMatcher::new(E::PreToolUse, Some("[secret")).err().unwrap().to_string(),
        "invalid Codex hook matcher"
    );
    assert!(HookMatcher::new(E::Stop, Some("[")).unwrap().matches(None));
    for pattern in [None, Some(""), Some("*")] {
        assert!(HookMatcher::new(E::PreToolUse, pattern).unwrap().matches(None));
    }
    assert!(
        !HookMatcher::new(E::PreToolUse, Some("Write|Edit")).unwrap().matches(Some("Overwrite"))
    );
    for event in [
        E::SessionStart,
        E::SessionEnd,
        E::PreCompact,
        E::PostCompact,
        E::SubagentStart,
        E::SubagentStop,
        E::PermissionRequest,
    ] {
        let matcher = HookMatcher::new(event, Some("^auto$")).unwrap();
        assert!(matcher.matches(Some("auto")));
        assert!(!matcher.matches(Some("manual")));
    }
}
#[test]
fn codex_plugin_compat_t33_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let manifest = manifest();
    let before = serde_json::to_vec(&manifest).unwrap();
    let mut trust =
        PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
    assert!(codex_handlers("fixture", root.path(), &manifest, &trust).unwrap().is_empty());
    trust.trust_codex_hooks("fixture", root.path(), &manifest).unwrap();
    assert!(codex_handlers("other", root.path(), &manifest, &trust).unwrap().is_empty());
    assert!(codex_handlers("fixture", other.path(), &manifest, &trust).unwrap().is_empty());
    let mut changed = manifest.clone();
    changed.hooks.get_mut(&E::PreToolUse).unwrap()[0].matcher = None;
    assert!(codex_handlers("fixture", root.path(), &changed, &trust).unwrap().is_empty());
    assert_eq!(before, serde_json::to_vec(&manifest).unwrap());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
