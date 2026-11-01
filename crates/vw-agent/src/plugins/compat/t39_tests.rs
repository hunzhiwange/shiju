use super::hook_session::{close, start, validate};
use crate::{hooks::HookRunner, tools::ToolUseContext};
use serde_json::json;
use std::sync::Arc;

async fn fixture(
    session: &str,
    root: &std::path::Path,
    marker: &std::path::Path,
) -> Arc<ToolUseContext> {
    fixture_with_exit(session, root, marker, 0).await
}
async fn fixture_with_exit(
    session: &str,
    root: &std::path::Path,
    marker: &std::path::Path,
    exit: i32,
) -> Arc<ToolUseContext> {
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    let state = tempfile::tempdir().unwrap();
    let manifest: CodexHooksManifest = serde_json::from_value(json!({"hooks":{
        "SessionStart":[{"hooks":[{"type":"command","command":format!("cat >> '{}'; printf '\\n' >> '{}'; printf '%s' '{{\"hookSpecificOutput\":{{\"hookEventName\":\"SessionStart\",\"additionalContext\":\"startup context\"}}}}'; exit {exit}",marker.display(),marker.display())}]}],
        "SessionEnd":[{"hooks":[{"type":"command","command":format!("cat >> '{}'; printf '\\n' >> '{}'",marker.display(),marker.display())}]}]
    }})).unwrap();
    let mut trust =
        PluginHookTrustStore::load_or_default(&state.path().join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root, &manifest).unwrap();
    let mut runner = HookRunner::new();
    for handler in codex_handlers("fixture", root, &manifest, &trust).unwrap() {
        runner.register(handler);
    }
    Arc::new(
        ToolUseContext::new(session, Some(root.to_string_lossy().into_owned()))
            .with_model("fixture")
            .with_hook_runner(Arc::new(runner)),
    )
}
#[tokio::test]
async fn codex_plugin_compat_t39_emits_session_events_once() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let marker = state.path().join("events");
    let context = fixture("once", root.path(), &marker).await;
    start(context.clone(), false).await.unwrap();
    start(context.clone(), true).await.unwrap();
    assert_eq!(context.take_hook_contexts(), vec!["startup context"]);
    assert!(context.take_hook_contexts().is_empty());
    close("once", root.path()).await;
    close("once", root.path()).await;
    let events: Vec<serde_json::Value> = std::fs::read_to_string(marker)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["hook_event_name"], "SessionStart");
    assert_eq!(events[0]["source"], "startup");
    assert_eq!(events[1]["reason"], "other");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
#[test]
fn codex_plugin_compat_t39_rejects_invalid_input() {
    assert!(
        validate(
            &json!({"hook_event_name":"SessionStart","session_id":"","cwd":"/","source":"startup"})
        )
        .is_err()
    );
    assert!(validate(&json!({"hook_event_name":"SessionStart","session_id":"s","cwd":"relative","source":"resume"})).is_err());
}
#[tokio::test]
async fn codex_plugin_compat_t39_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let marker = state.path().join("events");
    start(fixture("first", root.path(), &marker).await, true).await.unwrap();
    start(fixture("second", root.path(), &marker).await, false).await.unwrap();
    close("unrelated", root.path()).await;
    close("first", root.path()).await;
    close("second", root.path()).await;
    let lines = std::fs::read_to_string(marker).unwrap();
    assert_eq!(lines.lines().count(), 4);
    assert!(lines.contains("resume"));
}

#[tokio::test]
async fn codex_plugin_compat_t39_failed_start_cleans_up() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let marker = state.path().join("events");
    let context = fixture_with_exit("failure", root.path(), &marker, 2).await;
    assert!(start(context.clone(), false).await.unwrap_err().to_string().contains("blocked"));
    close("failure", root.path()).await;
    assert_eq!(std::fs::read_to_string(marker).unwrap().lines().count(), 2);
    assert!(context.take_hook_contexts().is_empty());
}
