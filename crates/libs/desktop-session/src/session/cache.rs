//! 冷聊天快照的保留、恢复与共享规则；热视图快照由桌面跨域应用。
use session_ui_types::ChatMessage;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub type SharedChatMessages = Arc<[ChatMessage]>;

pub fn prune_snapshots<'a>(
    requesting_session_ids: impl Iterator<Item = &'a str>,
    chats: &mut HashMap<String, SharedChatMessages>,
    message_ids: &mut HashMap<String, Vec<Option<String>>>,
) {
    let active: HashSet<_> = requesting_session_ids.collect();
    chats.retain(|id, _| active.contains(id.as_str()));
    message_ids.retain(|id, _| active.contains(id.as_str()));
}

pub fn restore_snapshot(
    cached: Option<&SharedChatMessages>,
    cached_message_ids: Option<&Vec<Option<String>>>,
    chat: &mut Vec<ChatMessage>,
    message_ids: &mut Vec<Option<String>>,
) -> bool {
    let hit = if let Some(cached) = cached {
        *chat = cached.iter().cloned().collect();
        true
    } else {
        chat.clear();
        false
    };
    *message_ids = cached_message_ids.cloned().unwrap_or_else(|| vec![None; chat.len()]);
    hit
}

pub fn active_shared_messages(cached: Option<&SharedChatMessages>, chat: &[ChatMessage]) -> SharedChatMessages {
    if let Some(cached) = cached && cached.len() == chat.len() {
        return cached.clone();
    }
    Arc::from(chat.to_vec())
}

pub fn needs_cold_fallback(is_requesting: bool, has_cold_fallback: bool) -> bool {
    is_requesting && !has_cold_fallback
}
