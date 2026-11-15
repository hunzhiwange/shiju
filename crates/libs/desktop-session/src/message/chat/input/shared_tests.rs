use super::shared::*;
use session_ui_types::{ChatMessage, ChatRole};

#[test]
fn preferred_chat_message_index_prefers_tool_row_for_duplicate_message_ids() {
    let chat = vec![
        ChatMessage {
            role: ChatRole::Assistant,
            content: "分析中".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::Tool,
            content: "tool grep\n{\"status\":\"completed\",\"output\":\"2 matches\"}\n".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];
    let message_ids = vec![Some("msg-1".to_string()), Some("msg-1".to_string())];

    assert_eq!(preferred_chat_message_index_by_id(&chat, &message_ids, "msg-1"), Some(1));
}
#[test]
fn preferred_chat_message_index_falls_back_to_first_match_without_tool_row() {
    let chat = vec![
        ChatMessage {
            role: ChatRole::Assistant,
            content: "第一段".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::Assistant,
            content: "第二段".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];
    let message_ids = vec![Some("msg-2".to_string()), Some("msg-2".to_string())];

    assert_eq!(preferred_chat_message_index_by_id(&chat, &message_ids, "msg-2"), Some(0));
}
#[test]
fn preferred_chat_message_index_returns_none_for_missing_id() {
    let chat = vec![ChatMessage {
        role: ChatRole::Assistant,
        content: "回答".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];
    let message_ids = vec![Some("msg-1".to_string())];

    assert_eq!(preferred_chat_message_index_by_id(&chat, &message_ids, "missing"), None);
}
