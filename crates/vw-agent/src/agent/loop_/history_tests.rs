//! 真实宿主摘要服务和插件压缩钩子的契约测试。

use super::*;

#[tokio::test]
async fn internal_compaction_returns_false_without_prior_history() {
    let mut history = vec![ChatMessage::system("system"), ChatMessage::user("short")];
    assert!(
        !compact_history_with_internal_service(
            &mut history,
            "empty",
            "test/model",
            &Default::default(),
            &crate::provider::provider::resolve_model_limit(None, None, None)
        )
        .await
        .unwrap()
    );
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serialize the shared scripted model boundary.
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
    let mut history = vec![
        ChatMessage::system("system"),
        ChatMessage::user("中文😀".repeat(50_000)),
        ChatMessage::assistant("recent decision"),
        ChatMessage::user("current request"),
        ChatMessage::assistant("current round"),
    ];
    assert!(
        compact_history_with_internal_service(
            &mut history,
            &session,
            "test/model",
            &Default::default(),
            &crate::provider::provider::resolve_model_limit(None, None, None)
        )
        .await
        .unwrap()
    );
    assert_eq!(crate::session::processor::take_test_llm_messages(&session).len(), 1);
    assert_eq!(history.len(), 4);
    assert_eq!(history[0].content, "system");
    assert!(history[1].content.contains("remembered decision"));
    assert_eq!(history[2].content, "current request");
    assert_eq!(history[3].content, "current round");
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serialize the shared scripted model boundary.
async fn internal_compaction_keeps_history_when_provider_fails() {
    let _guard = crate::session::processor::test_llm_stream_guard();
    crate::session::processor::enqueue_test_llm_stream(vec![
        crate::session::llm::StreamEvent::Error(crate::session::message::AssistantError::Unknown {
            message: "summary failed".into(),
        }),
    ]);
    let mut history = vec![ChatMessage::user("old"), ChatMessage::user("current")];
    let reliability =
        crate::config::ReliabilityConfig { provider_retries: 0, ..Default::default() };
    let error = compact_history_with_internal_service(
        &mut history,
        "failed-summary",
        "test/model",
        &reliability,
        &crate::provider::provider::resolve_model_limit(None, None, None),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("summary failed"));
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].content, "old");
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // Serialize the deterministic provider boundary.
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
    let mut history = vec![
        ChatMessage::user("old"),
        ChatMessage::assistant("old answer"),
        ChatMessage::user("current"),
    ];
    assert!(
        crate::tools::scope_tool_use_context(
            context,
            compact_history_with_internal_service(
                &mut history,
                &session,
                "fixture/model",
                &Default::default(),
                &crate::provider::provider::resolve_model_limit(None, None, None)
            )
        )
        .await
        .unwrap()
    );
    assert!(history[0].content.contains("fixture summary"));
    let contents = std::fs::read_to_string(state.path().join("events")).unwrap();
    let events: Vec<serde_json::Value> =
        contents.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["hook_event_name"], "PreCompact");
    assert_eq!(events[1]["hook_event_name"], "PostCompact");
}
