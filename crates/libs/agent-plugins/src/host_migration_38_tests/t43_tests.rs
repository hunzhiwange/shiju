use super::hook_subagent::emit;
use super::tools::ToolUseContext;
use serde_json::{Value, json};
use std::sync::Arc;
fn fixture(root: &std::path::Path, state: &std::path::Path, parent: &str) -> Arc<ToolUseContext> {
    fixture_output(root, state, parent, "", "")
}
fn fixture_output(
    root: &std::path::Path,
    state: &std::path::Path,
    parent: &str,
    start: &str,
    stop: &str,
) -> Arc<ToolUseContext> {
    use super::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
    let command = format!(
        "payload=$(cat); printf '%s\\n' \"$payload\" >> '{}'",
        state.join("events").display()
    );
    let manifest: CodexHooksManifest = serde_json::from_value(json!({"hooks":{
        "SubagentStart":[{"matcher":"reviewer", "hooks":[{"type":"command","command":format!("{command}; printf '%s' '{start}'")}]}],
        "SubagentStop":[{"matcher":"reviewer", "hooks":[{"type":"command","command":format!("{command}; case \"$payload\" in *'\"stop_hook_active\":true'*) ;; *) printf '%s' '{stop}' ;; esac")}]}]}}))
    .unwrap();
    let mut trust = PluginHookTrustStore::load_or_default(&state.join("trust.json")).unwrap();
    trust.trust_codex_hooks("fixture", root, &manifest).unwrap();
    let mut runner = super::hooks::new_hook_runner();
    for handler in codex_handlers("fixture", root, &manifest, &trust).unwrap() {
        runner.register(handler);
    }
    Arc::new(
        ToolUseContext::new(parent, Some(root.to_string_lossy().into_owned()))
            .with_turn_id("parent-turn")
            .with_model("fixture")
            .with_hook_runner(Arc::new(runner)),
    )
}
#[tokio::test]
async fn codex_plugin_compat_t43_binds_events_to_child_session() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let ctx = fixture(root.path(), state.path(), "parent");
    emit(Some(ctx.clone()), "parent", "child", "reviewer", None, false).await.unwrap();
    emit(Some(ctx), "parent", "child", "reviewer", Some("handoff"), true).await.unwrap();
    let events = std::fs::read_to_string(state.path().join("events"))
        .expect("child lifecycle must reach plugin");
    let events: Vec<Value> = events.lines().map(|s| serde_json::from_str(s).unwrap()).collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["hook_event_name"], "SubagentStart");
    assert_eq!(events[1]["hook_event_name"], "SubagentStop");
    for event in &events {
        assert_eq!(event["agent_id"], "child");
        assert_eq!(event["session_id"], "parent");
        assert_eq!(event["agent_type"], "reviewer");
    }
    assert_eq!(events[1]["last_assistant_message"], "handoff");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn codex_plugin_compat_t43_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let ctx = fixture(root.path(), state.path(), "parent");
    assert!(emit(Some(ctx.clone()), "other", "child", "reviewer", None, false).await.is_err());
    assert!(!state.path().join("events").exists());
    let (a, b) = tokio::join!(
        emit(Some(ctx.clone()), "parent", "child-a", "reviewer", None, false),
        emit(Some(ctx.clone()), "parent", "child-b", "reviewer", None, false)
    );
    a.unwrap();
    b.unwrap();
    let events = std::fs::read_to_string(state.path().join("events"))
        .expect("concurrent children must reach plugin");
    assert!(events.contains("child-a"));
    assert!(events.contains("child-b"));
    assert!(ctx.take_hook_contexts().is_empty());
}
