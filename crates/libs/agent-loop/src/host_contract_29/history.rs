//! Compaction scenarios parameterized only by the existing HistoryHost port.
use crate::history::{HistoryHost, compact_history_with_internal_service};
use model_types::ChatMessage;

pub async fn internal_compaction_returns_false_without_prior_history<H: HistoryHost>() {
    let mut history = vec![ChatMessage::system("system"), ChatMessage::user("short")];
    assert!(
        !compact_history_with_internal_service::<H>(
            &mut history,
            "empty",
            "test/model",
            &Default::default(),
            &agent_provider::provider::resolve_model_limit(None, None, None)
        )
        .await
        .unwrap()
    );
}

pub async fn internal_compaction_uses_shared_runner_and_preserves_current_turn<H: HistoryHost>(session: &str) {
    let mut history = vec![
        ChatMessage::system("system"),
        ChatMessage::user("中文😀".repeat(50_000)),
        ChatMessage::assistant("recent decision"),
        ChatMessage::user("current request"),
        ChatMessage::assistant("current round"),
    ];
    assert!(
        compact_history_with_internal_service::<H>(
            &mut history,
            session,
            "test/model",
            &Default::default(),
            &agent_provider::provider::resolve_model_limit(None, None, None)
        )
        .await
        .unwrap()
    );
    assert_eq!(history.len(), 4);
    assert_eq!(history[0].content, "system");
    assert!(history[1].content.contains("remembered decision"));
    assert_eq!(history[2].content, "current request");
    assert_eq!(history[3].content, "current round");
}

pub async fn internal_compaction_keeps_history_when_provider_fails<H: HistoryHost>() {
    let mut history = vec![ChatMessage::user("old"), ChatMessage::user("current")];
    let reliability =
        config_reliability::ReliabilityConfig { provider_retries: 0, ..Default::default() };
    let error = compact_history_with_internal_service::<H>(
        &mut history,
        "failed-summary",
        "test/model",
        &reliability,
        &agent_provider::provider::resolve_model_limit(None, None, None),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("summary failed"));
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].content, "old");
}

pub async fn codex_plugin_compat_t42_agent_loop_emits_around_real_summary<H: HistoryHost>(state: &std::path::Path, session: &str) {
    let mut history = vec![
        ChatMessage::user("old"),
        ChatMessage::assistant("old answer"),
        ChatMessage::user("current"),
    ];
    assert!(
            compact_history_with_internal_service::<H>(
                &mut history,
                session,
                "fixture/model",
                &Default::default(),
                &agent_provider::provider::resolve_model_limit(None, None, None)
            )
        .await
        .unwrap()
    );
    assert!(history[0].content.contains("fixture summary"));
    let contents = std::fs::read_to_string(state.join("events")).unwrap();
    let events: Vec<serde_json::Value> =
        contents.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["hook_event_name"], "PreCompact");
    assert_eq!(events[1]["hook_event_name"], "PostCompact");
}
