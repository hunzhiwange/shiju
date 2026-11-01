use super::hook_stop::{emit, validate};
use crate::{hooks::HookRunner, tools::ToolUseContext};
use serde_json::{Value, json};
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
    let mut runner = HookRunner::new();
    for handler in codex_handlers("fixture", root, manifest, &trust).unwrap() {
        runner.register(handler);
    }
    Arc::new(runner)
}
#[tokio::test]
async fn codex_plugin_compat_t44_distinguishes_stop_and_interrupt() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let ctx = fixture(root.path(), state.path(), "turn");
    assert_eq!(
        emit(ctx.clone(), false, false, Some("done")).await.unwrap().contexts,
        ["verify work"]
    );
    assert!(emit(ctx.clone(), false, true, Some("done")).await.unwrap().contexts.is_empty());
    let interrupt = emit(ctx, true, false, None).await.unwrap();
    assert!(!interrupt.stop);
    assert!(interrupt.contexts.is_empty());
    let events: Vec<Value> = std::fs::read_to_string(state.path().join("events"))
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["hook_event_name"], "Stop");
    assert_eq!(events[2]["hook_event_name"], "Interrupt");
}
#[test]
fn codex_plugin_compat_t44_rejects_invalid_input() {
    assert!(validate(&json!({})).is_err());
    let mut input = json!({"session_id":"s","turn_id":"t","model":"m","permission_mode":"default","cwd":"relative","hook_event_name":"Stop","stop_hook_active":false});
    assert!(validate(&input).is_err());
    input["cwd"] = json!("/tmp");
    input["stop_hook_active"] = Value::Null;
    assert!(validate(&input).is_err());
    let invalid = super::hook_stop::parse(Some(0), br#"{"decision":"block"}"#, b"", true).unwrap();
    assert!(super::hook_stop::outcome(&invalid, true).contexts.is_empty());
}
#[tokio::test]
async fn codex_plugin_compat_t44_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let a = fixture(root.path(), state.path(), "a");
    let b = fixture(root.path(), state.path(), "b");
    emit(a.clone(), false, true, Some("a")).await.unwrap();
    assert_eq!(emit(b, false, false, Some("b")).await.unwrap().contexts, ["verify work"]);
    assert!(a.take_hook_contexts().is_empty());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn codex_plugin_compat_t44_cancellation_remains_responsive() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let ctx = fixture(root.path(), state.path(), "cancelled");
    let token = tokio_util::sync::CancellationToken::new();
    token.cancel();
    let ctx = Arc::new(ctx.as_ref().clone().with_abort_token(token));
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        emit(ctx.clone(), false, false, None),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(result.contexts.is_empty());
    emit(ctx, true, false, None).await.unwrap();
    let events = std::fs::read_to_string(state.path().join("events")).unwrap();
    assert_eq!(events.lines().count(), 1);
    assert!(events.contains("Interrupt"));
}

#[test]
fn codex_plugin_compat_t44_stop_precedes_block_and_exit_two_continues() {
    use super::hook_stop::{Outcome, outcome, parse};
    let block = parse(Some(2), b"ignored", b" finish checks ", true).unwrap();
    assert_eq!(outcome(&block, true).contexts, ["finish checks"]);
    let stop =
        parse(Some(0), br#"{"continue":false,"decision":"block","reason":"ignored"}"#, b"", true)
            .unwrap();
    let mut combined = Outcome::default();
    combined.merge(outcome(&block, true));
    combined.merge(outcome(&stop, true));
    combined.merge(outcome(&block, true));
    assert!(combined.stop);
    assert!(combined.contexts.is_empty());
}

#[tokio::test]
async fn codex_plugin_compat_t44_cancels_in_flight_stop_and_bounds_interrupt() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let marker = state.path().join("started");
    let manifest = serde_json::from_value(json!({"hooks":{
        "Stop":[{"hooks":[{"type":"command","command":format!("cat >/dev/null; touch '{}'; sleep 30", marker.display())}]}],
        "Interrupt":[{"hooks":[{"type":"command","command":"cat >/dev/null; sleep 30", "timeout":1}]}]
    }})).unwrap();
    let token = tokio_util::sync::CancellationToken::new();
    let ctx = Arc::new(
        ToolUseContext::new("session", Some(root.path().to_string_lossy().into_owned()))
            .with_model("fixture")
            .with_turn_id("turn")
            .with_abort_token(token.clone())
            .with_hook_runner(register(root.path(), state.path(), &manifest)),
    );
    let running = tokio::spawn(emit(ctx.clone(), false, false, None));
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while !marker.exists() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    token.cancel();
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(1), running)
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .contexts
            .is_empty()
    );
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(2), emit(ctx, true, false, None))
            .await
            .unwrap()
            .unwrap()
            .contexts
            .is_empty()
    );
}
#[test]
fn codex_plugin_compat_t44_stop_without_block_reason_still_honors_continue_false() {
    let value =
        super::hook_stop::parse(Some(0), br#"{"continue":false,"decision":"block"}"#, b"", true)
            .unwrap();
    assert!(super::hook_stop::outcome(&value, true).stop);
}

#[tokio::test]
async fn codex_plugin_compat_t44_session_close_does_not_emit_stop_or_interrupt() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let original = br#"{"name":"fixture"}"#;
    std::fs::write(root.path().join("plugin.json"), original).unwrap();
    let context = fixture(root.path(), state.path(), "closing-turn");
    super::hook_session::start(context, false).await.unwrap();
    super::hook_session::close("session", root.path()).await;
    assert!(!state.path().join("events").exists());
    assert_eq!(std::fs::read(root.path().join("plugin.json")).unwrap(), original);
}
