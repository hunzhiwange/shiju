use super::*;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Testcov0094Provider {
    response: Option<String>,
    calls: AtomicUsize,
}

impl Testcov0094Provider {
    fn ok(response: &str) -> Self {
        Self { response: Some(response.to_string()), calls: AtomicUsize::new(0) }
    }

    fn failing() -> Self {
        Self { response: None, calls: AtomicUsize::new(0) }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for Testcov0094Provider {
    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(system_prompt.is_none());
        assert!(message.contains("Summarize the conversation below for a future continuation"));
        assert_eq!(model, "test-model");
        assert_eq!(temperature, 0.2);

        self.response.clone().ok_or_else(|| anyhow::anyhow!("summary failed"))
    }
}

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
fn compaction_transcript_truncates_long_sources() {
    let messages = vec![ChatMessage::user("x".repeat(COMPACTION_MAX_SOURCE_CHARS + 100))];

    let transcript = build_compaction_transcript(&messages);

    assert!(transcript.chars().count() <= COMPACTION_MAX_SOURCE_CHARS + 3);
}

#[tokio::test]
async fn internal_compaction_returns_false_when_under_limit() {
    let provider = Testcov0094Provider::ok("unused");
    let mut history = vec![ChatMessage::system("system"), ChatMessage::user("short")];

    let compacted =
        compact_history_with_internal_service(&mut history, &provider, "test-model", 3).await.unwrap();

    assert!(!compacted);
    assert_eq!(provider.calls(), 0);
    assert_eq!(history.len(), 2);
}

#[tokio::test]
async fn internal_compaction_returns_false_when_recent_window_covers_all_messages() {
    let provider = Testcov0094Provider::ok("unused");
    let mut history = vec![ChatMessage::user("older"), ChatMessage::assistant("newer")];

    let compacted =
        compact_history_with_internal_service(&mut history, &provider, "test-model", 1).await.unwrap();

    assert!(!compacted);
    assert_eq!(provider.calls(), 0);
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].content, "older");
    assert_eq!(history[1].content, "newer");
}

#[tokio::test]
async fn internal_compaction_preserves_system_and_recent_messages() {
    let provider = Testcov0094Provider::ok("- remembered decision");
    let mut history = vec![ChatMessage::system("system")];
    for index in 0..23 {
        history.push(ChatMessage::user(format!("message-{index}")));
    }

    let compacted =
        compact_history_with_internal_service(&mut history, &provider, "test-model", 2).await.unwrap();

    assert!(compacted);
    assert_eq!(provider.calls(), 1);
    assert_eq!(history.len(), 22);
    assert_eq!(history[0].role, "system");
    assert_eq!(history[1].role, "assistant");
    assert!(history[1].content.contains("[Compaction summary]"));
    assert!(history[1].content.contains("remembered decision"));
    assert_eq!(history[2].content, "message-3");
    assert_eq!(history.last().unwrap().content, "message-22");
}

#[tokio::test]
async fn internal_compaction_keeps_history_when_provider_fails() {
    let provider = Testcov0094Provider::failing();
    let mut history = Vec::new();
    for index in 0..22 {
        history.push(ChatMessage::user(format!("fallback-{index}")));
    }

    let error = compact_history_with_internal_service(&mut history, &provider, "test-model", 1)
        .await
        .expect_err("summary failure should preserve history");

    assert!(error.to_string().contains("internal compaction model request failed"));
    assert_eq!(provider.calls(), 1);
    assert_eq!(history.len(), 22);
    assert_eq!(history[0].content, "fallback-0");
}
