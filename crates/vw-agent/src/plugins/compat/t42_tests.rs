use crate::{
    hooks::HookRunner,
    session::compaction::{self, CompactionOutcome, ProcessInput},
    tools::ToolUseContext,
};
use serde_json::{Value, json};
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
    use crate::plugins::{CodexHooksManifest, PluginHookTrustStore, hook_handler::codex_handlers};
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
    let mut runner = HookRunner::new();
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
async fn run(session: &str, auto: bool, fail: bool) -> CompactionOutcome {
    attempt(session, auto, fail).await.unwrap()
}
async fn attempt(
    session: &str,
    auto: bool,
    fail: bool,
) -> Result<CompactionOutcome, crate::session::session::Error> {
    use compaction::compaction_tests::{model_with_limits, user_message};
    compaction::process_with_summarizer(
        ProcessInput {
            session_id: session.into(),
            parent_id: "boundary".into(),
            auto,
            messages: vec![
                user_message(session, "old", "fixture history"),
                user_message(session, "boundary", ""),
            ],
        },
        &model_with_limits(100_000, None, 10_000),
        |_| {
            if fail { Err("provider failed".into()) } else { Ok("fixture summary".into()) }
        },
    )
    .await
}
#[tokio::test]
async fn codex_plugin_compat_t42_emits_events_around_compaction() {
    for auto in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let state = tempfile::tempdir().unwrap();
        let session = format!("t42-{}", uuid::Uuid::new_v4());
        let ctx = fixture(root.path(), state.path(), &session);
        crate::tools::scope_tool_use_context(ctx, async {
            assert!(matches!(run(&session, auto, true).await, CompactionOutcome::Failed { .. }));
            assert!(matches!(
                run(&session, auto, false).await,
                CompactionOutcome::Compacted { .. }
            ));
        })
        .await;
        let contents = std::fs::read_to_string(state.path().join("events"))
            .expect("compaction must notify plugins");
        let events: Vec<Value> =
            contents.lines().map(|s| serde_json::from_str(s).unwrap()).collect();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0]["hook_event_name"], "PreCompact");
        assert_eq!(events[1]["hook_event_name"], "PreCompact");
        assert_eq!(events[2]["hook_event_name"], "PostCompact");
        assert_eq!(events[2]["trigger"], if auto { "auto" } else { "manual" });
        assert_eq!(events[2]["turn_id"], "turn");
        assert!(events[2].get("messages").is_none());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
#[test]
fn codex_plugin_compat_t42_rejects_invalid_input() {
    use super::hook_compact::{parse, validate};
    assert!(validate(&json!({})).is_err());
    assert!(parse(Some(0), br#"{"continue":false,"stopReason":"fixture"}"#).is_err());
    assert!(parse(Some(2), b"private diagnostic").is_ok());
    assert!(parse(Some(0), br#"{"decision":"block"}"#).is_ok());
    assert!(parse(Some(0), br#"{"continue":false,"stopReason":42}"#).is_ok());
    assert!(parse(Some(0), br#"{"continue":false,"decision":"block"}"#).is_ok());
}
#[tokio::test]
async fn codex_plugin_compat_t42_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let ctx = fixture(root.path(), state.path(), "owner");
    crate::tools::scope_tool_use_context(ctx.clone(), async {
        assert!(super::hook_compact::emit("other", "turn", "fixture", false, false).await.is_err());
    })
    .await;
    assert!(!state.path().join("events").exists());
    assert!(ctx.take_hook_contexts().is_empty());
}

#[tokio::test]
async fn codex_plugin_compat_t42_applies_stop_at_the_correct_boundary() {
    for post in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let state = tempfile::tempdir().unwrap();
        let session = format!("t42-stop-{}", uuid::Uuid::new_v4());
        let stop = r#"{"continue":false,"stopReason":"fixture stop"}"#;
        let ctx = fixture_output(
            root.path(),
            state.path(),
            &session,
            if post { "" } else { stop },
            if post { stop } else { "" },
        );
        let error = crate::tools::scope_tool_use_context(ctx, attempt(&session, false, false))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("fixture stop"));
        let events = std::fs::read_to_string(state.path().join("events")).unwrap();
        assert_eq!(events.lines().count(), if post { 2 } else { 1 });
        let source = vec![json!({"role":"user","content":"fixture history"})];
        let restored =
            crate::session::compaction_checkpoint::restore(&session, &source).await.unwrap();
        assert_eq!(restored[0]["content"].as_str().unwrap().contains("fixture summary"), post);
    }
}
