//! 测试会话历史的裁剪、规范化与系统消息保留规则。
//! 脚本化压缩供应商确认失败时保留原历史，成功时只替换可压缩窗口。

use super::*;
#[test]
fn trim_history_keeps_system_and_recent_messages() {
    let mut history = vec![
        ChatMessage::system("system"),
        ChatMessage::user("old"),
        ChatMessage::assistant("middle"),
        ChatMessage::user("new"),
    ];

    trim_history(&mut history, 2);

    assert_eq!(history.len(), 3);
    assert_eq!(history[0].content, "system");
    assert_eq!(history[1].content, "middle");
    assert_eq!(history[2].content, "new");
}

#[test]
fn trim_history_handles_no_system_and_zero_limit() {
    let mut history =
        vec![ChatMessage::user("old"), ChatMessage::assistant("middle"), ChatMessage::user("new")];

    trim_history(&mut history, 0);

    assert!(history.is_empty());
}

#[test]
fn compaction_transcript_and_summary_are_deterministic() {
    let messages = vec![ChatMessage::user(" hello "), ChatMessage::assistant("world")];
    let transcript = build_compaction_transcript(&messages);

    assert!(transcript.contains("USER: hello"));
    assert!(transcript.contains("ASSISTANT: world"));

    let mut history = messages;
    apply_compaction_summary(&mut history, 0, 1, "summary");
    assert_eq!(history[0].role, "assistant");
    assert!(history[0].content.contains("[Compaction summary]"));
}

#[test]
fn compaction_transcript_preserves_long_sources() {
    let messages = vec![ChatMessage::user(format!("{}TAIL_DECISION", "中文😀".repeat(50_000)))];

    let transcript = build_compaction_transcript(&messages);

    assert!(transcript.contains(&"中文😀".repeat(50_000)));
    assert!(transcript.contains("TAIL_DECISION"));
}

#[tokio::test]
async fn internal_compaction_returns_false_without_prior_history() {
    let mut history = vec![ChatMessage::system("system"), ChatMessage::user("short")];
    assert!(
        !compact_history_with_internal_service(
            &mut history,
            "empty",
            "test/model",
            &Default::default()
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
            &Default::default()
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
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("summary failed"));
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].content, "old");
}
