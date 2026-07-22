//! 维护聊天会话的加载、运行态和 UI 分块派生逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::loading::{SharedChatMessages, shared_chat_messages};
use super::{App, ChatMessage, ChatRole, ChatSessionStep, HashMap, StepUiMeta};
use serde_json::Value;

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
                step_index_map.get(&assistant_count).and_then(|step| {
                    let model_label = step.model.as_deref()?;
                    let time_ms = step.display_time_ms;
                    let time_label =
                        crate::app::components::chat_panel::utils::format_chat_time_label(time_ms);
                    Some(format!("{model_label} · {time_label}"))
                })
            }
            ChatRole::User => {
                let step_index = assistant_count + 1;
                step_index_map.get(&step_index).and_then(|step| {
                    let model_label = step.model.as_deref()?;
                    let time_ms = step.started_ms;
                    let time_label =
                        crate::app::components::chat_panel::utils::format_chat_time_label(time_ms);
                    Some(format!("{model_label} · {time_label}"))
                })
            }
            ChatRole::System | ChatRole::Tool => None,
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
            step_index_map.get(assistant_count).and_then(|step| {
                let model_label = step.model.as_deref()?;
                let time_ms = step.display_time_ms;
                let time_label =
                    crate::app::components::chat_panel::utils::format_chat_time_label(time_ms);
                Some(format!("{model_label} · {time_label}"))
            })
        }
        ChatRole::User => {
            let step_index = *assistant_count + 1;
            step_index_map.get(&step_index).and_then(|step| {
                let model_label = step.model.as_deref()?;
                let time_ms = step.started_ms;
                let time_label =
                    crate::app::components::chat_panel::utils::format_chat_time_label(time_ms);
                Some(format!("{model_label} · {time_label}"))
            })
        }
        ChatRole::System | ChatRole::Tool => None,
    }
}

impl App {
    /// 执行 rebuild_active_session_message_meta 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn rebuild_active_session_message_meta(&mut self) {
        self.active_session_view_state.message_meta_texts =
            build_message_meta_labels(&self.chat, &self.active_session_view_state.step_index_map);
    }

    /// 只刷新流式增量影响的尾部消息元信息。
    pub(crate) fn refresh_active_session_tail_message_meta(&mut self, changed_idx: usize) {
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
        self.active_session_view_state.step_index_map.clear();
    }

    #[allow(dead_code)]
    /// 执行 replace_active_session_steps 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn replace_active_session_steps(&mut self, steps: Vec<ChatSessionStep>) {
        self.active_session_view_state.step_index_map =
            steps.iter().map(|step| (step.index, StepUiMeta::from(step))).collect();
        self.active_session_view_state.steps = steps;
    }

    /// 执行 replace_active_session_calls 对应的领域操作。
    ///
    /// 原始调用记录来自会话 UI 快照，单独维护以避免历史加载时覆盖步骤索引。
    pub(crate) fn replace_active_session_calls(&mut self, calls: Vec<Value>) {
        self.active_session_view_state.calls = calls;
    }

    /// 执行 upsert_active_session_step 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn upsert_active_session_step(&mut self, step: ChatSessionStep) {
        self.active_session_view_state.step_index_map.insert(step.index, StepUiMeta::from(&step));
        if let Some(existing_step) = self
            .active_session_view_state
            .steps
            .iter_mut()
            .find(|existing| existing.index == step.index)
        {
            *existing_step = step;
        } else {
            self.active_session_view_state.steps.push(step);
            self.active_session_view_state.steps.sort_by_key(|existing| existing.index);
        }
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
            self.session_chat_cache.insert(id.clone(), shared_chat_messages(self.chat.clone()));
            self.session_chat_message_id_cache.insert(id, self.chat_message_ids.clone());
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
        self.session_chat_cache.insert(session_id.clone(), chat);
        self.session_chat_message_id_cache.insert(session_id, message_ids);
    }
}

#[cfg(test)]
#[path = "active_tests.rs"]
mod active_tests;
