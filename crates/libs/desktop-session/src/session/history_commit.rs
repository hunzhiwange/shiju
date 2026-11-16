//! History installation using session-owned data. The host supplies rendering and preference effects.
use super::{
    active::ActiveSessionViewState,
    chat_timeline::{ChatEntryKey, PageMergeOutcome, PagedChatTimeline, TimelinePage},
    loading::PreparedSessionOpen,
};
use crate::message::chat::trajectory::SessionStepRawKey;
use serde_json::Value;
use session_ui_types::{ChatMessage, ChatSessionStep, TokenUsage};
use std::collections::{HashMap, HashSet};

pub struct HistoryState<'a, P> {
    pub chat: &'a mut Vec<ChatMessage>,
    pub ids: &'a mut Vec<Option<String>>,
    pub usage: &'a mut TokenUsage,
    pub view: &'a mut ActiveSessionViewState<P>,
    pub timeline: &'a mut Option<PagedChatTimeline>,
}

pub struct PreparedCommit<U> {
    pub session_id: String,
    pub steps: Vec<ChatSessionStep>,
    pub calls: Vec<Value>,
    pub raw: HashMap<SessionStepRawKey, Value>,
    pub raw_indices: Vec<SessionStepRawKey>,
    pub session_model: Option<String>,
    pub base_ui_chunk: Option<U>,
    pub seq_base: Option<u64>,
}

impl<P: Default> HistoryState<'_, P> {
    pub fn install<U>(
        &mut self,
        prepared: PreparedSessionOpen<U>,
        generation: u64,
    ) -> PreparedCommit<U> {
        let PreparedSessionOpen {
            session_id,
            messages,
            message_ids,
            usage,
            steps,
            calls,
            step_llm_raw,
            llm_raw_step_indices,
            updated_ms,
            session_model,
            base_ui_chunk,
            timeline_page,
            ..
        } = prepared;
        let seq_base = timeline_page.map(|page| {
            let seq_base = page.entries.first().and_then(|entry| entry.seq_no).unwrap_or(0);
            let mut timeline = PagedChatTimeline::new(session_id.clone(), generation);
            timeline.merge_initial_page(page);
            *self.timeline = Some(timeline);
            seq_base
        });
        *self.chat = messages;
        *self.ids = message_ids;
        *self.usage = usage;
        self.view.updated_ms = updated_ms;
        PreparedCommit {
            session_id,
            steps,
            calls,
            raw: step_llm_raw,
            raw_indices: llm_raw_step_indices,
            session_model,
            base_ui_chunk,
            seq_base,
        }
    }

    pub fn prepend(
        &mut self,
        active: Option<&str>,
        session_id: &str,
        current_generation: bool,
        generation: u64,
        cursor: &str,
        result: Result<TimelinePage, String>,
    ) -> Option<PrependCommit> {
        if active != Some(session_id) || !current_generation {
            return None;
        }
        let timeline = self.timeline.as_mut()?;
        let page = match result {
            Ok(page) => page,
            Err(error) => {
                timeline.cancel_prepend(cursor);
                tracing::warn!(%session_id, %error, "failed to load older chat history page");
                return None;
            }
        };
        let old_keys =
            timeline.entries().iter().map(|entry| entry.key.clone()).collect::<HashSet<_>>();
        let inserted = match timeline.merge_prepend_page(session_id, generation, cursor, page) {
            PageMergeOutcome::Applied { inserted } => inserted,
            _ => return None,
        };
        let entries = timeline
            .entries()
            .iter()
            .filter(|entry| !old_keys.contains(&entry.key))
            .take(inserted)
            .collect::<Vec<_>>();
        let messages = entries.iter().map(|entry| entry.message.clone()).collect::<Vec<_>>();
        let ids = entries
            .iter()
            .map(|entry| match &entry.key {
                ChatEntryKey::PersistedId(id) => Some(id.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        self.chat.splice(0..0, messages);
        self.ids.splice(0..0, ids);
        Some(PrependCommit {
            inserted,
            seq_base: timeline.loaded_seq_range().map_or(0, |(first, _)| first),
        })
    }
}
pub struct PrependCommit {
    pub inserted: usize,
    pub seq_base: u64,
}

pub struct LoadedCommit {
    pub shared: super::cache::SharedChatMessages,
    pub same_visible: bool,
    pub shorter_gateway: Option<(usize, usize)>,
}
impl<P: Default> HistoryState<'_, P> {
    pub fn loaded(
        &mut self,
        messages: Vec<message_types::WithParts>,
        local: Option<Vec<ChatMessage>>,
        local_ids: Option<Vec<Option<String>>>,
        reset: Option<Vec<Option<String>>>,
        usage: TokenUsage,
        compaction_text: &str,
        project: impl Fn(&[ChatSessionStep], &HashMap<SessionStepRawKey, Value>) -> P,
    ) -> LoadedCommit {
        use desktop_workspace::project_message::session::lifecycle::{
            same_visible_history, select_loaded_history, steps_from_gateway_messages,
        };
        let steps = steps_from_gateway_messages(&messages);
        let (chat, ids, shorter_gateway) =
            select_loaded_history(messages, local, local_ids, reset, compaction_text);
        let same_visible = same_visible_history(self.chat, &chat);
        *self.ids = ids;
        *self.usage = usage;
        if same_visible {
            if !steps.is_empty() {
                self.view.replace_active_session_steps(steps, &project);
            }
            self.view.rebuild_active_session_message_meta(self.chat);
        } else {
            *self.chat = chat;
            let calls = self.view.calls.clone();
            let steps = if steps.is_empty() { self.view.steps.clone() } else { steps };
            self.view.updated_ms = 0;
            self.view.clear_active_session_steps(&project);
            self.view.replace_active_session_steps(steps, &project);
            self.view.replace_active_session_calls(calls);
        }
        LoadedCommit {
            shared: super::loading::shared_chat_messages(self.chat.clone()),
            same_visible,
            shorter_gateway,
        }
    }

    pub fn restore_cached(
        &mut self,
        messages: Vec<ChatMessage>,
        ids: Option<Vec<Option<String>>>,
        project: impl Fn(&[ChatSessionStep], &HashMap<SessionStepRawKey, Value>) -> P,
    ) -> super::cache::SharedChatMessages {
        *self.ids = ids
            .filter(|ids| ids.len() == messages.len())
            .unwrap_or_else(|| vec![None; messages.len()]);
        *self.chat = messages;
        self.view.clear_active_session_steps(project);
        self.view.ui_preparing = true;
        self.view.base_ready = false;
        super::loading::shared_chat_messages(self.chat.clone())
    }
}

impl<P: Default> HistoryState<'_, P> {
    pub fn close(&mut self) {
        self.chat.clear();
        *self.usage = TokenUsage::default();
        *self.view = ActiveSessionViewState::default();
    }
    pub fn blank_draft(&mut self) {
        self.close();
        self.ids.clear();
    }
    pub fn begin_cold_open(
        &mut self,
        project: impl FnOnce(&[ChatSessionStep], &HashMap<SessionStepRawKey, Value>) -> P,
    ) {
        *self.usage = TokenUsage::default();
        self.view.updated_ms = 0;
        self.view.clear_active_session_steps(project);
        self.view.ui_preparing = true;
        self.view.base_ready = false;
    }
}

impl<P: Default> HistoryState<'_, P> {
    pub fn prepare_loaded_ui(&mut self) -> bool {
        let needs_preparation = !self.chat.is_empty();
        self.view.ui_preparing = needs_preparation;
        self.view.base_ready = !needs_preparation;
        needs_preparation
    }
    pub fn finish_base_ui(&mut self) {
        self.view.base_ready = true;
        self.view.rebuild_active_session_message_meta(self.chat);
        self.view.ui_preparing = false;
    }
}

impl<P: Default> HistoryState<'_, P> {
    pub fn refresh_metadata<U>(
        &mut self,
        prepared: PreparedSessionOpen<U>,
        project: impl Fn(&[ChatSessionStep], &HashMap<SessionStepRawKey, Value>) -> P,
    ) {
        self.view.ui_preparing = false;
        *self.usage = prepared.usage;
        if !prepared.steps.is_empty() {
            self.view.replace_active_session_steps(prepared.steps, &project);
        }
        if !prepared.step_llm_raw.is_empty() {
            self.view.replace_active_session_step_llm_raw(prepared.step_llm_raw, &project);
        }
        if !prepared.llm_raw_step_indices.is_empty() {
            self.view.replace_active_session_llm_raw_step_indices(prepared.llm_raw_step_indices);
        }
    }
}

pub struct PreparedViewCompletion<U> {
    pub session_model: Option<String>,
    pub base_ui_chunk: Option<U>,
}
impl<U> PreparedCommit<U> {
    pub fn apply_view<P: Default>(
        self,
        view: &mut ActiveSessionViewState<P>,
        project: impl Fn(&[ChatSessionStep], &HashMap<SessionStepRawKey, Value>) -> P,
    ) -> PreparedViewCompletion<U> {
        view.replace_active_session_steps(self.steps, &project);
        view.replace_active_session_calls(self.calls);
        view.replace_active_session_step_llm_raw(self.raw, &project);
        view.replace_active_session_llm_raw_step_indices(self.raw_indices);
        PreparedViewCompletion {
            session_model: self.session_model,
            base_ui_chunk: self.base_ui_chunk,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ColdHistory {
    Remote,
    RequestingEmpty,
    RequestingMessages,
}
impl<P: Default> HistoryState<'_, P> {
    pub fn cold_history(&mut self, requesting: bool, cached: bool) -> ColdHistory {
        if !requesting || !cached {
            return ColdHistory::Remote;
        }
        if !self.chat.is_empty() {
            return ColdHistory::RequestingMessages;
        }
        self.view.ui_preparing = false;
        self.view.base_ready = true;
        ColdHistory::RequestingEmpty
    }
    pub fn begin_history_ui(&mut self) {
        self.view.ui_preparing = true;
        self.view.base_ready = false;
    }
    pub fn finish_history_ui(&mut self, base_installed: bool) {
        if base_installed {
            self.view.base_ready = true;
            self.view.rebuild_active_session_message_meta(self.chat);
        }
        self.view.ui_preparing = false;
    }
}
