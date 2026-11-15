use session_ui_types::{ChatMessage, ChatRole, ChatSession, ChatSessionMeta, ChatSessionStep, TokenUsage};
fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}
fn session_meta(id: &str, last_content: Option<&str>) -> ChatSessionMeta {
    ChatSessionMeta {
        id: id.to_string(),
        title: format!("Session {id}"),
        updated_ms: 20,
        message_count: 2,
        call_count: 1,
        last_content: last_content.map(str::to_string),
    }
}
fn session_step(index: u32, usage: TokenUsage) -> ChatSessionStep {
    ChatSessionStep {
        index,
        turn_id: None,
        started_ms: u64::from(index),
        finished_ms: None,
        start_snapshot_path: None,
        finish_snapshot_path: None,
        usage,
        cost_usd: None,
        finish_reason: None,
        model: None,
        tool_timings: Vec::new(),
    }
}
#[test]
fn shared_chat_messages_preserves_message_order_and_content() {
    let shared = super::shared_chat_messages(vec![
        message(ChatRole::User, "question"),
        message(ChatRole::Assistant, "answer"),
    ]);

    assert_eq!(shared.len(), 2);
    assert_eq!(shared[0].role, ChatRole::User);
    assert_eq!(shared[0].content, "question");
    assert_eq!(shared[1].role, ChatRole::Assistant);
    assert_eq!(shared[1].content, "answer");
}
#[test]
fn shared_chat_messages_accepts_an_empty_history() {
    let shared = super::shared_chat_messages(Vec::new());

    assert!(shared.is_empty());
}
#[test]
fn build_session_previews_filters_missing_and_blank_content() {
    let previews = super::build_session_previews(vec![
        session_meta("missing", None),
        session_meta("blank", Some(" \n\t ")),
        session_meta("visible", Some("content")),
    ]);

    assert_eq!(previews.len(), 1);
    assert_eq!(previews.get("visible").map(String::as_str), Some("content"));
}
#[test]
fn build_session_previews_uses_first_non_empty_trimmed_line() {
    let previews = super::build_session_previews(vec![session_meta(
        "session-1",
        Some(" \r\n\t\n  首轮问题  \r\nsecond  "),
    )]);

    assert_eq!(previews.get("session-1").map(String::as_str), Some("首轮问题"));
}
#[test]
fn build_session_previews_truncates_long_content_to_display_limit() {
    let content = "x".repeat(121);
    let expected = format!("{}…", "x".repeat(119));
    let previews = super::build_session_previews(vec![session_meta("session-1", Some(&content))]);

    assert_eq!(previews.get("session-1"), Some(&expected));
}
#[test]
fn build_session_previews_keeps_content_at_exact_display_limit() {
    let content = "x".repeat(120);
    let previews = super::build_session_previews(vec![session_meta("session-1", Some(&content))]);

    assert_eq!(previews.get("session-1"), Some(&content));
}
#[test]
fn build_session_previews_keeps_last_value_for_duplicate_session_id() {
    let previews = super::build_session_previews(vec![
        session_meta("session-1", Some("older")),
        session_meta("session-1", Some("newer")),
    ]);

    assert_eq!(previews.len(), 1);
    assert_eq!(previews.get("session-1").map(String::as_str), Some("newer"));
}
#[test]
fn session_usage_returns_zeroes_for_session_without_steps() {
    let session = ChatSession {
        id: "session-1".to_string(),
        title: "Session".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 10,
        updated_ms: 20,
    };

    assert_eq!(super::session_usage(&session), TokenUsage::default());
}
#[test]
fn session_usage_sums_every_token_category_across_steps() {
    let session = ChatSession {
        id: "session-1".to_string(),
        title: "Session".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: vec![
            session_step(
                1,
                TokenUsage {
                    input_tokens: 10,
                    output_tokens: 20,
                    cached_tokens: 3,
                    reasoning_tokens: 4,
                },
            ),
            session_step(
                2,
                TokenUsage {
                    input_tokens: 5,
                    output_tokens: 7,
                    cached_tokens: 11,
                    reasoning_tokens: 13,
                },
            ),
        ],
        created_ms: 10,
        updated_ms: 20,
    };

    assert_eq!(
        super::session_usage(&session),
        TokenUsage { input_tokens: 15, output_tokens: 27, cached_tokens: 14, reasoning_tokens: 17 }
    );
}
