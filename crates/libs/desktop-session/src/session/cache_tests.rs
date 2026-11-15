use super::cache::*;
use std::collections::HashMap;
use session_ui_types::{ChatMessage, ChatRole};
use std::sync::Arc;

fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage { role, content: content.to_string(), model: None, think_timing: Vec::new(), processing_timing: None }
}
fn shared_chat_messages(chat: Vec<ChatMessage>) -> SharedChatMessages { Arc::from(chat) }

#[test]
fn restore_chat_uses_cached_messages_ids_and_missing_id_defaults() {
    let mut chats = HashMap::new();
    let mut ids = HashMap::new();
    let mut chat = Vec::new();
    let mut message_ids = Vec::new();
    chats.insert("with-ids".to_string(), shared_chat_messages(vec![message(ChatRole::Assistant, "cached")]));
    ids.insert("with-ids".to_string(), vec![Some("message-1".to_string())]);
    chats.insert(
        "without-ids".to_string(),
        shared_chat_messages(vec![
            message(ChatRole::User, "first"),
            message(ChatRole::Assistant, "second"),
        ]),
    );

    restore_snapshot(chats.get("with-ids"), ids.get("with-ids"), &mut chat, &mut message_ids);
    assert_eq!(chat[0].content, "cached");
    assert_eq!(message_ids, vec![Some("message-1".to_string())]);

    restore_snapshot(chats.get("without-ids"), ids.get("without-ids"), &mut chat, &mut message_ids);
    assert_eq!(chat.len(), 2);
    assert_eq!(message_ids, vec![None, None]);

    restore_snapshot(chats.get("missing"), ids.get("missing"), &mut chat, &mut message_ids);
    assert!(chat.is_empty());
    assert!(message_ids.is_empty());
}

#[test]
fn active_shared_chat_messages_reuses_only_a_length_matching_active_snapshot() {
    let mut chat = vec![message(ChatRole::User, "live")];
    let matching = shared_chat_messages(vec![message(ChatRole::User, "cached")]);

    let reused = active_shared_messages(Some(&matching), &chat);
    assert!(Arc::ptr_eq(&reused, &matching));
    assert_eq!(reused[0].content, "cached");

    chat.push(message(ChatRole::Assistant, "new"));
    let rebuilt = active_shared_messages(Some(&matching), &chat);
    assert!(!Arc::ptr_eq(&rebuilt, &matching));
    assert_eq!(rebuilt.len(), 2);
    assert_eq!(rebuilt[0].content, "live");
}
