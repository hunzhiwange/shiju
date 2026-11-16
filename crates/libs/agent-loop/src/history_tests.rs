//! 历史裁剪、转录与摘要替换的纯算法测试。

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
