//! 活动会话的步骤与消息派生数据。
use crate::message::chat::trajectory::SessionStepRawKey;
use serde_json::Value;
use session_ui_types::{ChatMessage, ChatRole, ChatSessionStep};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct StepUiMeta {
    pub model: Option<String>,
    pub started_ms: u64,
    pub finished_ms: Option<u64>,
    pub display_time_ms: u64,
}

impl From<&ChatSessionStep> for StepUiMeta {
    fn from(step: &ChatSessionStep) -> Self {
        Self {
            model: step.model.clone(),
            started_ms: step.started_ms,
            finished_ms: step.finished_ms,
            display_time_ms: step.finished_ms.unwrap_or(step.started_ms),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct UserQuestionMarker {
    pub message_idx: usize,
    pub preview: String,
}

pub fn message_meta_text(
    model: Option<&str>,
    time_ms: Option<u64>,
) -> Option<desktop_i18n::chat::MessageMeta> {
    let model = model?.trim();
    if model.is_empty() {
        return None;
    }
    Some(desktop_i18n::chat::MessageMeta::Model { model: model.to_string(), time_ms })
}

pub fn normalize_step_turn_id(step: &mut ChatSessionStep) {
    if step.turn_id.as_ref().is_some_and(|turn_id| turn_id.trim().is_empty()) {
        step.turn_id = None;
    }
}

pub fn build_message_meta_labels(
    chat: &[ChatMessage],
    step_index_map: &HashMap<u32, StepUiMeta>,
) -> Vec<Option<desktop_i18n::chat::MessageMeta>> {
    let mut assistant_count = 0u32;
    let mut labels = Vec::with_capacity(chat.len());

    for message in chat {
        let label = match message.role {
            ChatRole::Assistant => {
                assistant_count += 1;
                let time_ms = step_index_map.get(&assistant_count).map(|step| step.display_time_ms);
                message_meta_text(message.model.as_deref(), time_ms)
            }
            ChatRole::User => {
                if session_text::parse_task_session_source_marker(&message.content).is_some() {
                    Some(desktop_i18n::chat::MessageMeta::TaskSource)
                } else {
                    let step_index = assistant_count + 1;
                    let time_ms = step_index_map.get(&step_index).map(|step| step.started_ms);
                    message_meta_text(message.model.as_deref(), time_ms)
                }
            }
            ChatRole::System | ChatRole::Developer | ChatRole::Tool => None,
        };
        labels.push(label);
    }

    labels
}

pub fn build_message_meta_label(
    message: &ChatMessage,
    assistant_count: &mut u32,
    step_index_map: &HashMap<u32, StepUiMeta>,
) -> Option<desktop_i18n::chat::MessageMeta> {
    match message.role {
        ChatRole::Assistant => {
            *assistant_count += 1;
            let time_ms = step_index_map.get(assistant_count).map(|step| step.display_time_ms);
            message_meta_text(message.model.as_deref(), time_ms)
        }
        ChatRole::User => {
            if session_text::parse_task_session_source_marker(&message.content).is_some() {
                Some(desktop_i18n::chat::MessageMeta::TaskSource)
            } else {
                let step_index = *assistant_count + 1;
                let time_ms = step_index_map.get(&step_index).map(|step| step.started_ms);
                message_meta_text(message.model.as_deref(), time_ms)
            }
        }
        ChatRole::System | ChatRole::Developer | ChatRole::Tool => None,
    }
}

pub fn build_user_question_markers(chat: &[ChatMessage]) -> Vec<UserQuestionMarker> {
    chat.iter()
        .enumerate()
        .filter_map(|(message_idx, message)| {
            (message.role == ChatRole::User).then(|| UserQuestionMarker {
                message_idx,
                preview: user_question_preview(
                    session_text::strip_task_session_source_marker(&message.content).as_ref(),
                ),
            })
        })
        .collect()
}

pub fn user_question_preview(content: &str) -> String {
    const PREVIEW_CHAR_LIMIT: usize = 100;

    let mut preview = String::new();
    let mut char_count = 0;
    let mut pending_space = false;
    let mut truncated = false;

    for character in content.chars() {
        if character.is_whitespace() {
            pending_space = !preview.is_empty();
            continue;
        }

        if char_count == PREVIEW_CHAR_LIMIT {
            truncated = true;
            break;
        }
        if pending_space {
            preview.push(' ');
            char_count += 1;
            pending_space = false;
            if char_count == PREVIEW_CHAR_LIMIT {
                truncated = true;
                break;
            }
        }
        preview.push(character);
        char_count += 1;
    }

    if preview.is_empty() {
        return String::new();
    }
    if truncated {
        preview.push('…');
    }
    preview
}

pub fn upsert_step(
    steps: &mut Vec<ChatSessionStep>,
    step_index_map: &mut HashMap<u32, StepUiMeta>,
    mut step: ChatSessionStep,
) {
    normalize_step_turn_id(&mut step);
    step_index_map.insert(step.index, StepUiMeta::from(&step));
    if let Some(existing_step) = steps.iter_mut().find(|existing| {
        existing.index == step.index && existing.turn_id.as_deref() == step.turn_id.as_deref()
    }) {
        *existing_step = step;
    } else {
        steps.push(step);
        steps.sort_by_key(|existing| (existing.started_ms, existing.index));
    }
}

pub fn sync_user_question_index(
    chat: &[ChatMessage],
    markers: &mut Vec<UserQuestionMarker>,
    indexed_len: &mut usize,
) {
    let previous_len = *indexed_len;
    if previous_len > chat.len() {
        markers.retain(|marker| marker.message_idx < chat.len());
        *indexed_len = chat.len();
        return;
    }

    if previous_len == chat.len() {
        return;
    }

    markers.extend(chat[previous_len..].iter().enumerate().filter_map(|(offset, message)| {
        (message.role == ChatRole::User).then(|| UserQuestionMarker {
            message_idx: previous_len + offset,
            preview: user_question_preview(
                session_text::strip_task_session_source_marker(&message.content).as_ref(),
            ),
        })
    }));
    *indexed_len = chat.len();
}

pub fn refresh_tail_message_meta(
    chat: &[ChatMessage],
    labels: &mut Vec<Option<desktop_i18n::chat::MessageMeta>>,
    step_index_map: &HashMap<u32, StepUiMeta>,
    changed_idx: usize,
) {
    if changed_idx >= chat.len() {
        labels.truncate(chat.len());
        return;
    }

    let meta_len = labels.len();
    if meta_len > chat.len() {
        labels.truncate(chat.len());
    }

    if meta_len < changed_idx || labels.len() + 1 < chat.len() {
        *labels = build_message_meta_labels(chat, step_index_map);
        return;
    }

    let mut assistant_count = chat[..changed_idx]
        .iter()
        .filter(|message| matches!(message.role, ChatRole::Assistant))
        .count() as u32;
    let label = build_message_meta_label(&chat[changed_idx], &mut assistant_count, &step_index_map);

    if changed_idx < labels.len() {
        labels[changed_idx] = label;
    } else {
        labels.push(label);
    }
}

/// 原始载荷增量不能覆盖已有的完整缓存；索引始终覆盖所有稳定键。
pub fn insert_missing_raw(
    stored: &mut HashMap<crate::message::chat::trajectory::SessionStepRawKey, serde_json::Value>,
    indices: &mut std::collections::HashSet<crate::message::chat::trajectory::SessionStepRawKey>,
    incoming: HashMap<crate::message::chat::trajectory::SessionStepRawKey, serde_json::Value>,
) -> bool {
    let mut changed = false;
    for (key, value) in incoming {
        indices.insert(key.clone());
        if let std::collections::hash_map::Entry::Vacant(entry) = stored.entry(key) {
            entry.insert(value);
            changed = true;
        }
    }
    changed
}

/// 估算会话投影中持有的文本与 JSON 数据容量。
pub fn estimated_bytes(
    steps: &[ChatSessionStep],
    calls: &[Value],
    step_llm_raw: &HashMap<crate::message::chat::trajectory::SessionStepRawKey, Value>,
    step_index_map: &HashMap<u32, StepUiMeta>,
    message_meta_texts: &[Option<desktop_i18n::chat::MessageMeta>],
    user_question_markers: &[UserQuestionMarker],
) -> usize {
    let step_bytes = steps.iter().map(|step| {
        step.model.as_deref().map_or(0, str::len)
            + step.start_snapshot_path.as_deref().map_or(0, str::len)
            + step.finish_snapshot_path.as_deref().map_or(0, str::len)
            + step.finish_reason.as_deref().map_or(0, str::len)
    });
    let call_bytes = calls.iter().map(json_value_bytes);
    let step_llm_raw_bytes = step_llm_raw.values().map(json_value_bytes);
    let step_meta_bytes =
        step_index_map.values().map(|meta| meta.model.as_deref().map_or(0, str::len));
    let message_meta_bytes = message_meta_texts
        .iter()
        .filter_map(Option::as_ref)
        .map(desktop_i18n::chat::MessageMeta::cached_bytes);
    let user_question_preview_bytes =
        user_question_markers.iter().map(|marker| marker.preview.len());

    step_bytes
        .chain(call_bytes)
        .chain(step_llm_raw_bytes)
        .chain(step_meta_bytes)
        .chain(message_meta_bytes)
        .chain(user_question_preview_bytes)
        .sum()
}

fn json_value_bytes(value: &Value) -> usize {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => 0,
        Value::String(text) => text.len(),
        Value::Array(values) => values.iter().map(json_value_bytes).sum(),
        Value::Object(entries) => {
            entries.iter().map(|(key, value)| key.len() + json_value_bytes(value)).sum()
        }
    }
}

use std::collections::HashSet;
#[derive(Debug, Clone, Default)]
pub struct ActiveSessionViewState<P> {
    pub updated_ms: u64,
    pub steps: Vec<ChatSessionStep>,
    pub calls: Vec<Value>,
    pub step_llm_raw: HashMap<SessionStepRawKey, Value>,
    pub trajectory_projection: P,
    #[doc(hidden)]
    pub trajectory_projection_rebuild_count: usize,
    pub llm_raw_step_indices: HashSet<SessionStepRawKey>,
    pub step_index_map: HashMap<u32, StepUiMeta>,
    pub message_meta_texts: Vec<Option<desktop_i18n::chat::MessageMeta>>,
    pub user_question_markers: Vec<UserQuestionMarker>,
    pub user_question_indexed_chat_len: usize,
    pub ui_preparing: bool,
    pub base_ready: bool,
    pub prepared_chat_ui_chunks: HashSet<usize>,
    pub preparing_chat_ui_chunks: HashSet<usize>,
    pub dirty_preparing_chat_ui_chunks: HashSet<usize>,
    pub last_visited_chat_ui_chunk_start: Option<usize>,
    pub pinned_chat_ui_chunk_start: Option<usize>,
}

impl<P: Default> ActiveSessionViewState<P> {
    pub fn estimated_bytes(&self) -> usize {
        estimated_bytes(
            &self.steps,
            &self.calls,
            &self.step_llm_raw,
            &self.step_index_map,
            &self.message_meta_texts,
            &self.user_question_markers,
        )
    }
    pub fn rebuild_active_session_message_meta(&mut self, chat: &[ChatMessage]) {
        self.message_meta_texts = build_message_meta_labels(chat, &self.step_index_map);
        if self.user_question_indexed_chat_len != chat.len() {
            self.user_question_markers = build_user_question_markers(&chat);
            self.user_question_indexed_chat_len = chat.len();
        }
    }
    pub fn sync_active_session_user_question_index(&mut self, chat: &[ChatMessage]) {
        sync_user_question_index(
            chat,
            &mut self.user_question_markers,
            &mut self.user_question_indexed_chat_len,
        );
    }
    pub fn refresh_active_session_tail_message_meta(
        &mut self,
        changed_idx: usize,
        chat: &[ChatMessage],
    ) {
        self.sync_active_session_user_question_index(chat);
        refresh_tail_message_meta(
            chat,
            &mut self.message_meta_texts,
            &self.step_index_map,
            changed_idx,
        );
    }
    pub fn clear_active_session_steps(
        &mut self,
        project: impl FnOnce(&[ChatSessionStep], &HashMap<SessionStepRawKey, Value>) -> P,
    ) {
        self.steps.clear();
        self.calls.clear();
        self.step_llm_raw.clear();
        self.llm_raw_step_indices.clear();
        self.step_index_map.clear();
        self.trajectory_projection = project(&self.steps, &self.step_llm_raw);
        self.trajectory_projection_rebuild_count += 1;
    }
    pub fn replace_active_session_steps(
        &mut self,
        mut steps: Vec<ChatSessionStep>,
        project: impl FnOnce(&[ChatSessionStep], &HashMap<SessionStepRawKey, Value>) -> P,
    ) {
        steps.iter_mut().for_each(normalize_step_turn_id);
        self.step_index_map =
            steps.iter().map(|step| (step.index, StepUiMeta::from(step))).collect();
        self.steps = steps;
        self.trajectory_projection = project(&self.steps, &self.step_llm_raw);
        self.trajectory_projection_rebuild_count += 1;
    }
    pub fn replace_active_session_calls(&mut self, calls: Vec<Value>) {
        self.calls = calls;
    }
    pub fn replace_active_session_step_llm_raw(
        &mut self,
        raw: HashMap<SessionStepRawKey, Value>,
        project: impl FnOnce(&[ChatSessionStep], &HashMap<SessionStepRawKey, Value>) -> P,
    ) {
        self.llm_raw_step_indices.extend(raw.keys().cloned());
        self.step_llm_raw = raw;
        self.trajectory_projection = project(&self.steps, &self.step_llm_raw);
        self.trajectory_projection_rebuild_count += 1;
    }
    pub fn replace_active_session_llm_raw_step_indices(&mut self, indices: Vec<SessionStepRawKey>) {
        self.llm_raw_step_indices = indices.into_iter().collect();
        self.llm_raw_step_indices.extend(self.step_llm_raw.keys().cloned());
    }
    pub fn upsert_active_session_step(
        &mut self,
        step: ChatSessionStep,
        project: impl FnOnce(&[ChatSessionStep], &HashMap<SessionStepRawKey, Value>) -> P,
    ) {
        upsert_step(&mut self.steps, &mut self.step_index_map, step);
        self.trajectory_projection = project(&self.steps, &self.step_llm_raw);
        self.trajectory_projection_rebuild_count += 1;
    }
    pub fn insert_active_session_step_llm_raw(
        &mut self,
        key: SessionStepRawKey,
        raw: Value,
        project: impl FnOnce(&[ChatSessionStep], &HashMap<SessionStepRawKey, Value>) -> P,
    ) {
        self.llm_raw_step_indices.insert(key.clone());
        self.step_llm_raw.insert(key, raw);
        self.trajectory_projection = project(&self.steps, &self.step_llm_raw);
        self.trajectory_projection_rebuild_count += 1;
    }
    pub fn insert_missing_active_session_step_llm_raw(
        &mut self,
        raw: HashMap<SessionStepRawKey, Value>,
        project: impl FnOnce(&[ChatSessionStep], &HashMap<SessionStepRawKey, Value>) -> P,
    ) {
        if insert_missing_raw(&mut self.step_llm_raw, &mut self.llm_raw_step_indices, raw) {
            self.trajectory_projection = project(&self.steps, &self.step_llm_raw);
            self.trajectory_projection_rebuild_count += 1;
        }
    }
}

impl<P: Default> ActiveSessionViewState<P> {
    pub fn sync_from_chat(&mut self, chat: &[ChatMessage], updated_ms: Option<u64>) {
        if let Some(updated_ms) = updated_ms {
            self.updated_ms = self.updated_ms.max(updated_ms);
        }
        self.rebuild_active_session_message_meta(chat);
    }
}
