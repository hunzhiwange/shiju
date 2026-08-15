use std::time::Duration;

use serde_json::json;
use tempfile::TempDir;
use vw_acp::server::{
    BackendCancelHandle, BackendEvent, BackendTermination, BackendToolStatus, PromptContent,
    SessionBackend,
};
use vw_api_types::id::TenantScope;

use super::backend::{VibeWindowBackend, map_runtime_event, start_prompt_with_runtime_runner};
use crate::session::processor::{StreamEvent, ToolCallStatus, ToolEvent, ToolEventKind};
use crate::session::runtime::{PromptEvent, PromptTermination};
use crate::session::ui_types::TokenUsage;

fn tenant(suffix: &str) -> TenantScope {
    TenantScope::new(format!("acp-platform-{suffix}"), format!("acp-company-{suffix}"))
        .expect("valid tenant")
}

#[tokio::test]
async fn backend_reuses_native_session_identity_and_history() {
    crate::project::instance::dispose_all().await;
    let root = TempDir::new().expect("temporary cwd");
    let backend = VibeWindowBackend::new(tenant("identity"));

    let created =
        SessionBackend::create_session(&backend, root.path().to_path_buf()).await.expect("create");
    let loaded = SessionBackend::load_session(&backend, &created.id, root.path().to_path_buf())
        .await
        .expect("load");

    assert_eq!(loaded.id, created.id);
    assert_eq!(loaded.cwd, created.cwd);
    assert_eq!(loaded.history, created.history);
    crate::project::instance::dispose_all().await;
}

#[tokio::test]
async fn backend_prompt_persists_turn_once() {
    crate::project::instance::dispose_all().await;
    let root = TempDir::new().expect("temporary cwd");
    let backend = VibeWindowBackend::new(tenant("persist"));
    let created =
        SessionBackend::create_session(&backend, root.path().to_path_buf()).await.expect("create");
    let mut prompt = start_prompt_with_runtime_runner(
        &backend,
        &created.id,
        vec![PromptContent::Text("hello".to_string())],
        |_request, mut emit| {
            emit(StreamEvent::Delta("answer".to_string()));
            emit(StreamEvent::Done(TokenUsage::default()));
        },
    )
    .await
    .expect("prompt");

    while let Some(event) = prompt.events.recv().await {
        if matches!(event, BackendEvent::Terminal(_)) {
            break;
        }
    }
    let loaded = SessionBackend::load_session(&backend, &created.id, root.path().to_path_buf())
        .await
        .expect("reload");
    assert_eq!(loaded.history.len(), 2);
    assert_eq!(loaded.history[0].text, "hello");
    assert_eq!(loaded.history[1].text, "answer");
    crate::project::instance::dispose_all().await;
}

#[test]
fn backend_maps_runtime_events_without_parsing_display_text() {
    let payload = json!({
        "status": "completed",
        "input": {"token": "[REDACTED]"},
        "output": {"ok": true}
    });
    let event = PromptEvent::Tool(ToolEvent {
        call_id: "call-1".to_string(),
        name: "shell".to_string(),
        kind: ToolEventKind::Complete,
        status: ToolCallStatus::Completed,
        occurred_ms: 1,
        payload: payload.clone(),
    });

    let mapped = map_runtime_event(event).expect("mapped event");
    assert!(matches!(
        mapped,
        BackendEvent::ToolComplete {
            call_id,
            status: BackendToolStatus::Completed,
            raw_output: Some(output),
            ..
        } if call_id == "call-1" && output == json!({"ok": true})
    ));
}

#[tokio::test]
async fn backend_cancel_propagates_to_runtime_turn() {
    crate::project::instance::dispose_all().await;
    let root = TempDir::new().expect("temporary cwd");
    let backend = VibeWindowBackend::new(tenant("cancel"));
    let created =
        SessionBackend::create_session(&backend, root.path().to_path_buf()).await.expect("create");
    let mut prompt = start_prompt_with_runtime_runner(
        &backend,
        &created.id,
        vec![PromptContent::Text("wait".to_string())],
        |request, mut emit| {
            let abort = request.abort_token.expect("abort token");
            while !abort.is_cancelled() {
                std::thread::yield_now();
            }
            emit(StreamEvent::Delta("ignored".to_string()));
        },
    )
    .await
    .expect("prompt");

    assert!(prompt.cancel.cancel());
    assert!(!prompt.cancel.cancel());
    let terminal = tokio::time::timeout(Duration::from_secs(1), prompt.events.recv())
        .await
        .expect("terminal timeout")
        .expect("terminal event");
    assert_eq!(terminal, BackendEvent::Terminal(BackendTermination::Cancelled));
    crate::project::instance::dispose_all().await;
}
