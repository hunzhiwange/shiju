use super::hook_model::{emit, validate};
use crate::{hooks::HookRunner, tools::ToolUseContext};
use serde_json::{Value, json};
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
    let mut runner = HookRunner::new();
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
#[tokio::test]
async fn codex_plugin_compat_t41_wraps_actual_model_request() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let marker = state.path().join("events");
    let ctx = fixture(root.path(), state.path(), &marker);
    crate::tools::scope_tool_use_context(ctx.clone(), async {
        emit(None, Value::Null).await;
        std::fs::write(state.path().join("provider"), "executed").unwrap();
        emit(Some("completed"), json!({"input_tokens":3,"output_tokens":2,"total_tokens":5})).await;
    })
    .await;
    assert!(state.path().join("provider").exists());
    let events: Vec<Value> = std::fs::read_to_string(marker)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["hook_event_name"], "PreModelRequest");
    assert_eq!(events[1]["status"], "completed");
    assert_eq!(events[1]["token_usage"]["total_tokens"], 5);
    assert!(ctx.take_hook_contexts().is_empty());
}
#[tokio::test]
async fn codex_plugin_compat_t41_rejects_invalid_input() {
    assert!(validate(&json!({"hook_event_name":"PreModelRequest"})).is_err());
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let marker = state.path().join("events");
    let ctx = fixture(root.path(), state.path(), &marker);
    let ctx = Arc::new(ctx.as_ref().clone().with_turn_id(""));
    crate::tools::scope_tool_use_context(ctx, emit(None, Value::Null)).await;
    assert!(!marker.exists());
}
#[tokio::test]
async fn codex_plugin_compat_t41_preserves_isolation() {
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let marker = state.path().join("events");
    let ctx = fixture(root.path(), state.path(), &marker);
    crate::tools::scope_tool_use_context(ctx.clone(), async {
        for status in ["failed", "cancelled"] {
            emit(None, Value::Null).await;
            emit(Some(status), Value::Null).await;
        }
    })
    .await;
    emit(None, Value::Null).await;
    let text = std::fs::read_to_string(marker).unwrap();
    let events: Vec<Value> = text.lines().map(|s| serde_json::from_str(s).unwrap()).collect();
    assert_eq!(events.len(), 4);
    assert_eq!(events[1]["status"], "failed");
    assert_eq!(events[3]["status"], "cancelled");
    for event in events {
        assert_eq!(event["turn_id"], "turn");
        assert!(event.get("messages").is_none());
        assert!(event.get("api_key").is_none());
    }
    assert!(ctx.take_hook_contexts().is_empty());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
