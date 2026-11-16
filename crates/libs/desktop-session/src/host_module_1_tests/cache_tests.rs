use crate::session::{
    active::ActiveSessionViewState,
    agent::SessionRuntimeState,
    cache::*,
    view_cache::{SessionViewCache, ViewSnapshot},
};
use session_info::{Info, TimeInfo};
use session_ui_types::{ChatMessage, ChatRole};
use std::collections::HashMap;
use std::sync::Arc;
struct Snapshot;
impl ViewSnapshot for Snapshot {
    fn estimated_bytes(&self) -> usize {
        0
    }
    fn content_signature(&self) -> u64 {
        0
    }
}
#[derive(Default)]
struct Fixture {
    active_session_id: Option<String>,
    chat: Vec<ChatMessage>,
    chat_message_ids: Vec<Option<String>>,
    active_session_view_state: ActiveSessionViewState<()>,
    sessions: Vec<Info>,
    project_sessions: HashMap<String, Vec<Info>>,
    runtimes: HashMap<String, SessionRuntimeState<()>>,
    chats: HashMap<String, SharedChatMessages>,
    ids: HashMap<String, Vec<Option<String>>>,
    views: SessionViewCache<Snapshot>,
    boundaries: HistoryResetBoundaries,
}
impl Fixture {
    fn sync_active_session_from_chat(&mut self) {
        let updated = self
            .active_session_id
            .as_deref()
            .and_then(|id| self.sessions.iter().find(|s| s.id == id))
            .map(|s| s.time.updated);
        self.active_session_view_state.sync_from_chat(&self.chat, updated);
    }
    fn get_session_runtime_mut(&mut self, id: &str) -> &mut SessionRuntimeState<()> {
        self.runtimes.entry(id.into()).or_default()
    }
    fn store_session_chat_snapshot(
        &mut self,
        id: String,
        chat: SharedChatMessages,
        ids: Vec<Option<String>>,
    ) {
        store_snapshot(
            self.active_session_id.as_deref(),
            id,
            chat,
            ids,
            &mut self.chats,
            &mut self.ids,
            &mut self.views,
        );
    }
    fn cache_active_session_chat(&mut self) {
        cache_active_snapshot(
            self.active_session_id.as_deref(),
            &self.chat,
            &self.chat_message_ids,
            &self.runtimes,
            &mut self.chats,
            &mut self.ids,
            &mut self.views,
        );
    }
    fn prune_inactive_session_chat_cache(&mut self) {
        prune_runtime_snapshots(&self.runtimes, &mut self.chats, &mut self.ids);
    }
    fn cached_chat_messages(&self, id: &str) -> Option<SharedChatMessages> {
        self.chats.get(id).cloned()
    }
    fn cached_chat_message_ids(&self, id: &str) -> Option<Vec<Option<String>>> {
        self.ids.get(id).cloned()
    }
    fn known_session_directory(&self, id: &str) -> Option<String> {
        known_session_directory(id, &self.sessions, &self.project_sessions, &self.runtimes)
    }
    fn mark_session_history_reset_boundary(&mut self, id: String, ids: Vec<Option<String>>) {
        self.boundaries.insert(id, ids);
    }
    fn session_history_reset_boundary(&self, id: &str) -> Option<&[Option<String>]> {
        self.boundaries.get(id).map(Vec::as_slice)
    }
}
fn shared_chat_messages(messages: Vec<ChatMessage>) -> SharedChatMessages {
    Arc::from(messages)
}
fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

fn session_info(id: &str, directory: &str, updated_ms: u64) -> Info {
    Info {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: "project".to_string(),
        directory: directory.to_string(),
        parent_id: None,
        summary: None,
        share: None,
        title: id.to_string(),
        version: "0.0.0".to_string(),
        time: TimeInfo { created: 1, updated: updated_ms, compacting: None, archived: None },
        permission: None,
        fork_execution_started: None,
        revert: None,
    }
}

#[test]
fn sync_active_session_from_chat_only_advances_session_timestamp() {
    let mut app = Fixture::default();
    app.active_session_id = Some("active".to_string());
    app.sessions = vec![session_info("active", "/active", 2_000)];
    app.active_session_view_state.updated_ms = 1_000;
    app.chat = vec![message(ChatRole::User, "question")];

    app.sync_active_session_from_chat();
    assert_eq!(app.active_session_view_state.updated_ms, 2_000);
    assert_eq!(app.active_session_view_state.message_meta_texts, vec![None]);

    app.sessions[0].time.updated = 500;
    app.sync_active_session_from_chat();
    assert_eq!(app.active_session_view_state.updated_ms, 2_000);
}

#[test]
fn cache_pruning_retains_requesting_sessions_and_removes_inactive_sessions() {
    let mut app = Fixture::default();
    app.active_session_id = Some("requesting".to_string());
    app.chat = vec![message(ChatRole::User, "kept")];
    app.chat_message_ids = vec![Some("message-1".to_string())];
    app.get_session_runtime_mut("requesting").request_lifecycle.is_requesting = true;
    app.store_session_chat_snapshot(
        "inactive".to_string(),
        shared_chat_messages(vec![message(ChatRole::User, "removed")]),
        vec![Some("message-2".to_string())],
    );

    app.cache_active_session_chat();

    assert!(app.cached_chat_messages("requesting").is_some());
    assert_eq!(
        app.cached_chat_message_ids("requesting"),
        Some(vec![Some("message-1".to_string())])
    );
    assert!(app.cached_chat_messages("inactive").is_none());
    assert!(app.cached_chat_message_ids("inactive").is_none());

    app.get_session_runtime_mut("requesting").request_lifecycle.is_requesting = false;
    app.prune_inactive_session_chat_cache();
    assert!(app.cached_chat_messages("requesting").is_none());
    assert!(app.cached_chat_message_ids("requesting").is_none());
}

#[test]
fn known_session_directory_prefers_loaded_sessions_then_searches_projects() {
    let mut app = Fixture::default();
    app.sessions = vec![session_info("duplicate", "/loaded", 1)];
    app.project_sessions.insert(
        "/project".to_string(),
        vec![
            session_info("duplicate", "/project-duplicate", 1),
            session_info("project-only", "/project-only", 1),
        ],
    );

    assert_eq!(app.known_session_directory("duplicate").as_deref(), Some("/loaded"));
    assert_eq!(app.known_session_directory("project-only").as_deref(), Some("/project-only"));
    assert_eq!(app.known_session_directory("missing"), None);
}

#[test]
fn history_reset_boundary_replaces_previous_value_and_is_session_scoped() {
    let mut app = Fixture::default();
    app.mark_session_history_reset_boundary("session-a".to_string(), vec![Some("old".to_string())]);
    app.mark_session_history_reset_boundary(
        "session-a".to_string(),
        vec![Some("retained".to_string()), None],
    );

    assert_eq!(
        app.session_history_reset_boundary("session-a"),
        Some([Some("retained".to_string()), None].as_slice())
    );
    assert_eq!(app.session_history_reset_boundary("session-b"), None);
}
