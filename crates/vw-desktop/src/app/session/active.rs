//! 维护聊天会话的加载、运行态和 UI 分块派生逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::loading::{SharedChatMessages, shared_chat_messages};
use super::view_cache::{SessionViewCacheInsertResult, SessionViewSnapshot};
use super::{App, ChatMessage, ChatRole, ChatSessionStep, HashMap, StepUiMeta};
use crate::app::state::{SessionStepRawKey, UserQuestionMarker};
use serde_json::Value;

fn message_meta_text(model: Option<&str>, time_ms: Option<u64>) -> Option<String> {
    let model = model?.trim();
    if model.is_empty() {
        return None;
    }
    let Some(time_ms) = time_ms else {
        return Some(model.to_string());
    };
    let time_label = crate::app::components::chat_panel::utils::format_chat_time_label(time_ms);
    Some(format!("{model} · {time_label}"))
}

fn normalize_step_turn_id(step: &mut ChatSessionStep) {
    if step.turn_id.as_ref().is_some_and(|turn_id| turn_id.trim().is_empty()) {
        step.turn_id = None;
    }
}

fn build_message_meta_labels(
    chat: &[ChatMessage],
    step_index_map: &HashMap<u32, StepUiMeta>,
) -> Vec<Option<String>> {
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
                if vw_shared::session::session_utils::parse_task_session_source_marker(
                    &message.content,
                )
                .is_some()
                {
                    Some(
                        vw_shared::session::session_utils::TASK_SESSION_SOURCE_DISPLAY_LABEL
                            .to_string(),
                    )
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

fn build_message_meta_label(
    message: &ChatMessage,
    assistant_count: &mut u32,
    step_index_map: &HashMap<u32, StepUiMeta>,
) -> Option<String> {
    match message.role {
        ChatRole::Assistant => {
            *assistant_count += 1;
            let time_ms = step_index_map.get(assistant_count).map(|step| step.display_time_ms);
            message_meta_text(message.model.as_deref(), time_ms)
        }
        ChatRole::User => {
            if vw_shared::session::session_utils::parse_task_session_source_marker(&message.content)
                .is_some()
            {
                Some(
                    vw_shared::session::session_utils::TASK_SESSION_SOURCE_DISPLAY_LABEL
                        .to_string(),
                )
            } else {
                let step_index = *assistant_count + 1;
                let time_ms = step_index_map.get(&step_index).map(|step| step.started_ms);
                message_meta_text(message.model.as_deref(), time_ms)
            }
        }
        ChatRole::System | ChatRole::Developer | ChatRole::Tool => None,
    }
}

fn build_user_question_markers(chat: &[ChatMessage]) -> Vec<UserQuestionMarker> {
    chat.iter()
        .enumerate()
        .filter_map(|(message_idx, message)| {
            (message.role == ChatRole::User).then(|| UserQuestionMarker {
                message_idx,
                preview: crate::app::components::chat_panel::user_question_preview(
                    vw_shared::session::session_utils::strip_task_session_source_marker(
                        &message.content,
                    )
                    .as_ref(),
                ),
            })
        })
        .collect()
}

impl App {
    fn rebuild_active_session_trajectory_projection(&mut self) {
        self.active_session_view_state.trajectory_projection =
            crate::app::components::chat_panel::trajectory::project_trajectory(
                &self.active_session_view_state.steps,
                &self.active_session_view_state.step_llm_raw,
            );
        #[cfg(test)]
        {
            self.active_session_view_state.trajectory_projection_rebuild_count += 1;
        }
    }

    pub(crate) fn insert_active_session_step_llm_raw(
        &mut self,
        key: SessionStepRawKey,
        raw: Value,
    ) {
        self.active_session_view_state.llm_raw_step_indices.insert(key.clone());
        self.active_session_view_state.step_llm_raw.insert(key, raw);
        self.rebuild_active_session_trajectory_projection();
    }

    pub(crate) fn insert_missing_active_session_step_llm_raw(
        &mut self,
        raw: HashMap<SessionStepRawKey, Value>,
    ) {
        let mut changed = false;
        for (key, value) in raw {
            self.active_session_view_state.llm_raw_step_indices.insert(key.clone());
            if let std::collections::hash_map::Entry::Vacant(entry) =
                self.active_session_view_state.step_llm_raw.entry(key)
            {
                entry.insert(value);
                changed = true;
            }
        }
        if changed {
            self.rebuild_active_session_trajectory_projection();
        }
    }

    /// 移走离开会话的稳定聊天工作集；容量拒绝时保留原始消息缓存作冷恢复后备。
    pub(crate) fn stash_active_session_view(&mut self) {
        let Some(session_id) = self.active_session_id.clone() else {
            return;
        };

        self.close_session_transient_ui();
        let has_cold_chat_fallback = self.session_chat_cache.contains_key(&session_id);
        if self.get_session_runtime(&session_id).is_requesting && !has_cold_chat_fallback {
            self.store_session_chat_snapshot(
                session_id.clone(),
                shared_chat_messages(self.chat.clone()),
                self.chat_message_ids.clone(),
            );
        }
        let snapshot = SessionViewSnapshot::take_from(self);
        let message_count = snapshot.chat().len();

        match self.session_view_cache.insert(session_id.clone(), snapshot) {
            SessionViewCacheInsertResult::Inserted { evicted_session_ids } => {
                tracing::info!(
                    target: "vw_desktop",
                    session_id,
                    cache_hit = false,
                    message_count,
                    restore_source = "view_cache_stash",
                    evicted_sessions = evicted_session_ids.len(),
                    "stashed session view workset"
                );
            }
            SessionViewCacheInsertResult::RejectedOverCapacity { snapshot, estimated_bytes } => {
                snapshot.restore_into(self);
                if !has_cold_chat_fallback {
                    self.store_session_chat_snapshot(
                        session_id.clone(),
                        shared_chat_messages(self.chat.clone()),
                        self.chat_message_ids.clone(),
                    );
                }
                tracing::info!(
                    target: "vw_desktop",
                    session_id,
                    cache_hit = false,
                    message_count,
                    restore_source = "cold_chat_fallback",
                    estimated_bytes,
                    "session view workset exceeded hot cache capacity"
                );
            }
        }
    }

    /// 会话切换不保留短暂交互，避免热恢复到已打开的菜单或悬浮状态。
    pub(crate) fn close_session_transient_ui(&mut self) {
        self.chat_think_hovered_idx = None;
        self.chat_tool_file_hovered = None;
        self.chat_tool_hovered_idx = None;
        self.tool_detail_dialog = None;
        self.chat_context_menu_target = None;
        self.chat_context_menu_pos = None;
        self.chat_context_menu_text.clear();
        self.chat_reset_menu_idx = None;
        self.chat_fork_dialog_idx = None;
        self.show_agent_sessions_popover = false;
        self.show_session_tool_selector_popover = false;
        self.show_session_actions_popover = false;
    }

    /// 执行 rebuild_active_session_message_meta 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn rebuild_active_session_message_meta(&mut self) {
        self.active_session_view_state.message_meta_texts =
            build_message_meta_labels(&self.chat, &self.active_session_view_state.step_index_map);
        if self.active_session_view_state.user_question_indexed_chat_len != self.chat.len() {
            self.active_session_view_state.user_question_markers =
                build_user_question_markers(&self.chat);
            self.active_session_view_state.user_question_indexed_chat_len = self.chat.len();
        }
    }

    /// 将聊天尾部新增的消息增量纳入问题导航索引。
    pub(crate) fn sync_active_session_user_question_index(&mut self) {
        let indexed_len = self.active_session_view_state.user_question_indexed_chat_len;
        if indexed_len > self.chat.len() {
            self.active_session_view_state
                .user_question_markers
                .retain(|marker| marker.message_idx < self.chat.len());
            self.active_session_view_state.user_question_indexed_chat_len = self.chat.len();
            return;
        }

        if indexed_len == self.chat.len() {
            return;
        }

        self.active_session_view_state.user_question_markers.extend(
            self.chat[indexed_len..].iter().enumerate().filter_map(|(offset, message)| {
                (message.role == ChatRole::User).then(|| UserQuestionMarker {
                    message_idx: indexed_len + offset,
                    preview: crate::app::components::chat_panel::user_question_preview(
                        vw_shared::session::session_utils::strip_task_session_source_marker(
                            &message.content,
                        )
                        .as_ref(),
                    ),
                })
            }),
        );
        self.active_session_view_state.user_question_indexed_chat_len = self.chat.len();
    }

    /// 只刷新流式增量影响的尾部消息元信息。
    pub(crate) fn refresh_active_session_tail_message_meta(&mut self, changed_idx: usize) {
        self.sync_active_session_user_question_index();
        if changed_idx >= self.chat.len() {
            self.active_session_view_state.message_meta_texts.truncate(self.chat.len());
            return;
        }

        let meta_len = self.active_session_view_state.message_meta_texts.len();
        if meta_len > self.chat.len() {
            self.active_session_view_state.message_meta_texts.truncate(self.chat.len());
        }

        if meta_len < changed_idx
            || self.active_session_view_state.message_meta_texts.len() + 1 < self.chat.len()
        {
            self.rebuild_active_session_message_meta();
            return;
        }

        let mut assistant_count = self.chat[..changed_idx]
            .iter()
            .filter(|message| matches!(message.role, ChatRole::Assistant))
            .count() as u32;
        let label = build_message_meta_label(
            &self.chat[changed_idx],
            &mut assistant_count,
            &self.active_session_view_state.step_index_map,
        );

        if changed_idx < self.active_session_view_state.message_meta_texts.len() {
            self.active_session_view_state.message_meta_texts[changed_idx] = label;
        } else {
            self.active_session_view_state.message_meta_texts.push(label);
        }
    }

    /// 执行 clear_active_session_steps 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn clear_active_session_steps(&mut self) {
        self.active_session_view_state.steps.clear();
        self.active_session_view_state.calls.clear();
        self.active_session_view_state.step_llm_raw.clear();
        self.active_session_view_state.llm_raw_step_indices.clear();
        self.active_session_view_state.step_index_map.clear();
        self.rebuild_active_session_trajectory_projection();
    }

    #[allow(dead_code)]
    /// 执行 replace_active_session_steps 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn replace_active_session_steps(&mut self, mut steps: Vec<ChatSessionStep>) {
        steps.iter_mut().for_each(normalize_step_turn_id);
        self.active_session_view_state.step_index_map =
            steps.iter().map(|step| (step.index, StepUiMeta::from(step))).collect();
        self.active_session_view_state.steps = steps;
        self.rebuild_active_session_trajectory_projection();
    }

    /// 执行 replace_active_session_calls 对应的领域操作。
    ///
    /// 原始调用记录来自会话 UI 快照，单独维护以避免历史加载时覆盖步骤索引。
    pub(crate) fn replace_active_session_calls(&mut self, calls: Vec<Value>) {
        self.active_session_view_state.calls = calls;
    }

    /// 替换当前会话按稳定 turn/step 键保存的原始 LLM 载荷。
    pub(crate) fn replace_active_session_step_llm_raw(
        &mut self,
        raw: HashMap<SessionStepRawKey, Value>,
    ) {
        self.active_session_view_state.llm_raw_step_indices.extend(raw.keys().cloned());
        self.active_session_view_state.step_llm_raw = raw;
        self.rebuild_active_session_trajectory_projection();
    }

    /// 替换当前会话在存储层已存在的原始 LLM 稳定 turn/step 键。
    pub(crate) fn replace_active_session_llm_raw_step_indices(
        &mut self,
        indices: Vec<SessionStepRawKey>,
    ) {
        self.active_session_view_state.llm_raw_step_indices = indices.into_iter().collect();
        self.active_session_view_state
            .llm_raw_step_indices
            .extend(self.active_session_view_state.step_llm_raw.keys().cloned());
    }

    /// 执行 upsert_active_session_step 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn upsert_active_session_step(&mut self, mut step: ChatSessionStep) {
        normalize_step_turn_id(&mut step);
        self.active_session_view_state.step_index_map.insert(step.index, StepUiMeta::from(&step));
        if let Some(existing_step) =
            self.active_session_view_state.steps.iter_mut().find(|existing| {
                existing.index == step.index
                    && existing.turn_id.as_deref() == step.turn_id.as_deref()
            })
        {
            *existing_step = step;
        } else {
            self.active_session_view_state.steps.push(step);
            self.active_session_view_state
                .steps
                .sort_by_key(|existing| (existing.started_ms, existing.index));
        }
        self.rebuild_active_session_trajectory_projection();
    }

    /// 执行 sync_active_session_from_chat 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn sync_active_session_from_chat(&mut self) {
        if let Some(session_id) = self.active_session_id.as_deref()
            && let Some(info) = self.sessions.iter().find(|session| session.id == session_id)
        {
            self.active_session_view_state.updated_ms =
                self.active_session_view_state.updated_ms.max(info.time.updated);
        }
        self.rebuild_active_session_message_meta();
    }

    /// 执行 cache_active_session_chat 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn cache_active_session_chat(&mut self) {
        if let Some(id) = self.active_session_id.clone() {
            self.store_session_chat_snapshot(
                id,
                shared_chat_messages(self.chat.clone()),
                self.chat_message_ids.clone(),
            );
        }
        self.prune_inactive_session_chat_cache();
    }

    /// 执行 prune_inactive_session_chat_cache 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn prune_inactive_session_chat_cache(&mut self) {
        let active_session_ids: std::collections::HashSet<_> = self
            .session_runtime_states
            .iter()
            .filter(|(_, s)| s.is_requesting)
            .map(|(id, _)| id.clone())
            .collect();
        self.session_chat_cache.retain(|session_id, _| active_session_ids.contains(session_id));
        self.session_chat_message_id_cache
            .retain(|session_id, _| active_session_ids.contains(session_id));
    }

    /// 执行 restore_chat_for_session 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn restore_chat_for_session(&mut self, session_id: &str) {
        if let Some(cached) = self.session_chat_cache.get(session_id).cloned() {
            self.chat = cached.iter().cloned().collect();
            tracing::info!(
                target: "vw_desktop",
                session_id,
                cached_messages = self.chat.len(),
                "restored session chat from local cache"
            );
        } else {
            tracing::warn!(
                target: "vw_desktop",
                session_id,
                "no cached chat found for session, clearing active chat"
            );
            self.chat.clear();
        }
        self.chat_message_ids = self
            .session_chat_message_id_cache
            .get(session_id)
            .cloned()
            .unwrap_or_else(|| vec![None; self.chat.len()]);
    }

    /// 执行 known_session_directory 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn known_session_directory(&self, session_id: &str) -> Option<String> {
        self.sessions
            .iter()
            .find(|session| session.id == session_id)
            .map(|session| session.directory.clone())
            .or_else(|| {
                self.project_sessions
                    .values()
                    .flat_map(|sessions| sessions.iter())
                    .find(|session| session.id == session_id)
                    .map(|session| session.directory.clone())
            })
    }

    /// 执行 cached_chat_messages 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn cached_chat_messages(&self, session_id: &str) -> Option<SharedChatMessages> {
        self.session_chat_cache.get(session_id).cloned()
    }

    /// 执行 cached_chat_message_ids 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn cached_chat_message_ids(&self, session_id: &str) -> Option<Vec<Option<String>>> {
        self.session_chat_message_id_cache.get(session_id).cloned()
    }

    /// 执行 mark_session_history_reset_boundary 对应的领域操作。
    ///
    /// 记录重置时保留的消息 ID 序列，后续网关旧加载结果必须按该边界裁剪。
    pub(crate) fn mark_session_history_reset_boundary(
        &mut self,
        session_id: String,
        retained_message_ids: Vec<Option<String>>,
    ) {
        self.session_history_reset_boundaries.insert(session_id, retained_message_ids);
    }

    /// 执行 session_history_reset_boundary 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn session_history_reset_boundary(
        &self,
        session_id: &str,
    ) -> Option<&[Option<String>]> {
        self.session_history_reset_boundaries.get(session_id).map(Vec::as_slice)
    }

    /// 执行 active_shared_chat_messages 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn active_shared_chat_messages(&self) -> SharedChatMessages {
        if let Some(session_id) = self.active_session_id.as_deref()
            && let Some(cached) = self.cached_chat_messages(session_id)
            && cached.len() == self.chat.len()
        {
            return cached;
        }

        shared_chat_messages(self.chat.clone())
    }

    /// 执行 store_session_chat_snapshot 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn store_session_chat_snapshot(
        &mut self,
        session_id: String,
        chat: SharedChatMessages,
        message_ids: Vec<Option<String>>,
    ) {
        let content_signature = super::loading::session_content_signature(&chat, &message_ids);
        let invalidated_view_cache = self.active_session_id.as_deref() != Some(session_id.as_str())
            && self
                .session_view_cache
                .invalidate_if_content_changed(&session_id, content_signature);
        self.session_chat_cache.insert(session_id.clone(), chat);
        self.session_chat_message_id_cache.insert(session_id.clone(), message_ids);
        if invalidated_view_cache {
            tracing::info!(
                target: "vw_desktop",
                session_id,
                message_count = self
                    .session_chat_cache
                    .get(&session_id)
                    .map_or(0, |messages| messages.len()),
                content_signature,
                branch = "background_snapshot_changed",
                "invalidated stale session view cache"
            );
        }
    }
}

#[cfg(test)]
#[path = "active_tests.rs"]
mod active_tests;
