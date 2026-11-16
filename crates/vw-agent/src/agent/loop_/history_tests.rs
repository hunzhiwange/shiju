//! Real summary runner and plugin hook bindings.
use super::*;
use agent_loop::host_contract_29::history as scenarios;

#[tokio::test]
async fn internal_compaction_returns_false_without_prior_history() {
    scenarios::internal_compaction_returns_false_without_prior_history::<HistoryHost>().await;
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn internal_compaction_uses_shared_runner_and_preserves_current_turn() {
    let _guard = crate::session::processor::test_llm_stream_guard();
    crate::session::processor::enqueue_test_llm_stream(vec![
        crate::session::llm::StreamEvent::Delta("remembered decision".into()),
        crate::session::llm::StreamEvent::Done {
            usage: Default::default(),
            finish_reason: Some("stop".into()),
        },
    ]);
    let session = format!("history-summary-{}", uuid::Uuid::new_v4());
    scenarios::internal_compaction_uses_shared_runner_and_preserves_current_turn::<HistoryHost>(&session).await;
    assert_eq!(crate::session::processor::take_test_llm_messages(&session).len(), 1);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn internal_compaction_keeps_history_when_provider_fails() {
    let _guard = crate::session::processor::test_llm_stream_guard();
    crate::session::processor::enqueue_test_llm_stream(vec![
        crate::session::llm::StreamEvent::Error(crate::session::message::AssistantError::Unknown {
            message: "summary failed".into(),
        }),
    ]);
    scenarios::internal_compaction_keeps_history_when_provider_fails::<HistoryHost>().await;
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn codex_plugin_compat_t42_agent_loop_emits_around_real_summary() {
    let _guard = crate::session::processor::test_llm_stream_guard();
    crate::session::processor::enqueue_test_llm_stream(vec![
        crate::session::llm::StreamEvent::Delta("fixture summary".into()),
        crate::session::llm::StreamEvent::Done {
            usage: Default::default(),
            finish_reason: Some("stop".into()),
        },
    ]);
    let root = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let session = format!("t42-loop-{}", uuid::Uuid::new_v4());
    let context = crate::plugins::t42_tests::fixture(root.path(), state.path(), &session);
    crate::tools::scope_tool_use_context(context, scenarios::codex_plugin_compat_t42_agent_loop_emits_around_real_summary::<HistoryHost>(state.path(), &session)).await;
}
