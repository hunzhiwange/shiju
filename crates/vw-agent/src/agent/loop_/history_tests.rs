//! 测试会话历史的裁剪、规范化与系统消息保留规则。
//! 脚本化压缩供应商确认失败时保留原历史，成功时只替换可压缩窗口。

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
fn compaction_transcript_preserves_long_sources() {
    let messages = vec![ChatMessage::user(format!("{}TAIL_DECISION", "中文😀".repeat(5000)))];

    let transcript = build_compaction_transcript(&messages);

    assert!(transcript.contains(&"中文😀".repeat(5000)));
    assert!(transcript.contains("TAIL_DECISION"));
}

#[tokio::test]
async fn internal_compaction_returns_false_without_prior_history() {
    let provider = Testcov0094Provider::ok("unused");
    let mut history = vec![ChatMessage::system("system"), ChatMessage::user("short")];

    let compacted = compact_history_with_internal_service(
        &mut history,
        &provider,
        "test-model",
        &crate::app::agent::provider::provider::ModelLimit {
            context: 128_000,
            input: None,
            output: 4096,
        },
    )
    .await
    .unwrap();

    assert!(!compacted);
    assert_eq!(provider.calls(), 0);
    assert_eq!(history.len(), 2);
}

#[tokio::test]
async fn internal_compaction_keeps_the_current_user_and_assistant_round() {
    let provider = Testcov0094Provider::ok("unused");
    let mut history = vec![ChatMessage::user("older"), ChatMessage::assistant("newer")];

    let compacted = compact_history_with_internal_service(
        &mut history,
        &provider,
        "test-model",
        &crate::app::agent::provider::provider::ModelLimit {
            context: 128_000,
            input: None,
            output: 4096,
        },
    )
    .await
    .unwrap();

    assert!(!compacted);
    assert_eq!(provider.calls(), 0);
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].content, "older");
    assert_eq!(history[1].content, "newer");
}

#[tokio::test]
async fn internal_compaction_preserves_system_and_current_user() {
    let provider = Testcov0094Provider::ok("- remembered decision");
    let mut history = vec![ChatMessage::system("system")];
    for index in 0..23 {
        history.push(ChatMessage::user(format!("message-{index}")));
    }

    let compacted = compact_history_with_internal_service(
        &mut history,
        &provider,
        "test-model",
        &crate::app::agent::provider::provider::ModelLimit {
            context: 128_000,
            input: None,
            output: 4096,
        },
    )
    .await
    .unwrap();

    assert!(compacted);
    assert_eq!(provider.calls(), 1);
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].role, "system");
    assert_eq!(history[1].role, "assistant");
    assert!(history[1].content.contains("[Compaction summary]"));
    assert!(history[1].content.contains("remembered decision"));
    assert_eq!(history[2].content, "message-22");
    assert_eq!(history.last().unwrap().content, "message-22");
}

#[tokio::test]
async fn internal_compaction_keeps_history_when_provider_fails() {
    let provider = Testcov0094Provider::failing();
    let mut history = Vec::new();
    for index in 0..22 {
        history.push(ChatMessage::user(format!("fallback-{index}")));
    }

    let error = compact_history_with_internal_service(
        &mut history,
        &provider,
        "test-model",
        &crate::app::agent::provider::provider::ModelLimit {
            context: 128_000,
            input: None,
            output: 4096,
        },
    )
    .await
    .expect_err("summary failure should preserve history");

    assert!(error.to_string().contains("internal compaction model request failed"));
    assert_eq!(provider.calls(), 1);
    assert_eq!(history.len(), 22);
    assert_eq!(history[0].content, "fallback-0");
}

#[tokio::test]
async fn token_compaction_summarizes_few_large_messages_and_keeps_current_turn() {
    let provider = Testcov0094Provider::ok("remembered decision");
    let mut history = vec![
        ChatMessage::system("system"),
        ChatMessage::user("中文😀".repeat(5000)),
        ChatMessage::assistant("recent decision"),
        ChatMessage::user("current request"),
        ChatMessage::assistant("current tool round"),
    ];
    assert!(
        compact_history_with_internal_service(
            &mut history,
            &provider,
            "test-model",
            &crate::app::agent::provider::provider::ModelLimit {
                context: 128_000,
                input: None,
                output: 4096
            }
        )
        .await
        .unwrap()
    );
    assert!(provider.calls() > 1);
    assert_eq!(history.len(), 4);
    assert_eq!(history[0].content, "system");
    assert!(history[1].content.contains("remembered decision"));
    assert_eq!(history[2].content, "current request");
    assert_eq!(history[3].content, "current tool round");
}
