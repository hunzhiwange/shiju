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

pub fn active_shared_messages(
    cached: Option<&SharedChatMessages>,
    chat: &[ChatMessage],
) -> SharedChatMessages {
    if let Some(cached) = cached
        && cached.len() == chat.len()
    {
        return cached.clone();
    }
    Arc::from(chat.to_vec())
}

pub fn needs_cold_fallback(is_requesting: bool, has_cold_fallback: bool) -> bool {
    is_requesting && !has_cold_fallback
}

#[derive(Default)]
pub struct HistoryResetBoundaries(HashMap<String, Vec<Option<String>>>);
impl HistoryResetBoundaries {
    pub fn insert(&mut self, id: String, messages: Vec<Option<String>>) {
        self.0.insert(id, messages);
    }
    pub fn get(&self, id: &str) -> Option<&Vec<Option<String>>> {
        self.0.get(id)
    }
    pub fn remove(&mut self, id: &str) {
        self.0.remove(id);
    }
}

pub fn known_session_directory<R>(
    id: &str,
    sessions: &[session_info::Info],
    projects: &HashMap<String, Vec<session_info::Info>>,
    states: &HashMap<String, super::agent::SessionRuntimeState<R>>,
) -> Option<String> {
    sessions
        .iter()
        .chain(projects.values().flatten())
        .find(|session| session.id == id)
        .map(|session| session.directory.clone())
        .or_else(|| states.get(id)?.draft.as_ref().map(|draft| draft.directory.clone()))
}

/// Snapshot storage and hot-view invalidation share one content signature boundary.
pub fn store_snapshot<S: super::view_cache::ViewSnapshot>(
    active: Option<&str>,
    id: String,
    chat: SharedChatMessages,
    ids: Vec<Option<String>>,
    chats: &mut HashMap<String, SharedChatMessages>,
    message_ids: &mut HashMap<String, Vec<Option<String>>>,
    views: &mut super::view_cache::SessionViewCache<S>,
) -> bool {
    let signature = super::loading::session_content_signature(&chat, &ids);
    let invalidated =
        active != Some(id.as_str()) && views.invalidate_if_content_changed(&id, signature);
    chats.insert(id.clone(), chat);
    message_ids.insert(id, ids);
    invalidated
}

pub fn cache_active_snapshot<R, S: super::view_cache::ViewSnapshot>(
    active: Option<&str>,
    chat: &[ChatMessage],
    ids: &[Option<String>],
    runtimes: &HashMap<String, super::agent::SessionRuntimeState<R>>,
    chats: &mut HashMap<String, SharedChatMessages>,
    message_ids: &mut HashMap<String, Vec<Option<String>>>,
    views: &mut super::view_cache::SessionViewCache<S>,
) {
    if let Some(id) = active {
        store_snapshot(
            active,
            id.to_owned(),
            Arc::from(chat.to_vec()),
            ids.to_vec(),
            chats,
            message_ids,
            views,
        );
    }
    prune_runtime_snapshots(runtimes, chats, message_ids);
}
pub fn prune_runtime_snapshots<R>(
    runtimes: &HashMap<String, super::agent::SessionRuntimeState<R>>,
    chats: &mut HashMap<String, SharedChatMessages>,
    ids: &mut HashMap<String, Vec<Option<String>>>,
) {
    prune_snapshots(
        runtimes
            .iter()
            .filter(|(_, runtime)| runtime.request_lifecycle.is_requesting)
            .map(|(id, _)| id.as_str()),
        chats,
        ids,
    );
}
