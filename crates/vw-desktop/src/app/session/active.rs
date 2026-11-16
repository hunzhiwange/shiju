//! 维护聊天会话的加载、运行态和 UI 分块派生逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::loading::{SharedChatMessages, shared_chat_messages};
use super::view_cache::{SessionViewCacheInsertResult, SessionViewSnapshot};
use super::{App, ChatSessionStep, HashMap};
#[cfg(test)]
use super::{ChatMessage, ChatRole};
use crate::app::state::SessionStepRawKey;
use serde_json::Value;

use desktop_session::session::cache;

impl App {
    pub(crate) fn insert_active_session_step_llm_raw(
        &mut self,
        key: SessionStepRawKey,
        raw: Value,
    ) {
        self.active_session_view_state.insert_active_session_step_llm_raw(
            key,
            raw,
            crate::app::components::chat_panel::trajectory::project_trajectory,
        );
    }

    pub(crate) fn insert_missing_active_session_step_llm_raw(
        &mut self,
        raw: HashMap<SessionStepRawKey, Value>,
    ) {
        self.active_session_view_state.insert_missing_active_session_step_llm_raw(
            raw,
            crate::app::components::chat_panel::trajectory::project_trajectory,
        );
    }

    /// 移走离开会话的稳定聊天工作集；容量拒绝时保留原始消息缓存作冷恢复后备。
    pub(crate) fn stash_active_session_view(&mut self) {
        let Some(session_id) = self.active_session_id.clone() else {
            return;
        };

        self.close_session_transient_ui();
        let has_cold_chat_fallback = self.session_chat_cache.contains_key(&session_id);
        if cache::needs_cold_fallback(
            self.get_session_runtime(&session_id).request_lifecycle.is_requesting,
            has_cold_chat_fallback,
        ) {
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
        // 工作集已交给缓存，必须同时解除活动会话归属。否则后续项目重载会
        // 把 take_from 留下的空聊天再次保存到旧会话，覆盖后台流的历史基础。
        self.active_session_id = None;
    }

    /// 会话切换不保留短暂交互，避免热恢复到已打开的菜单或悬浮状态。
    pub(crate) fn close_session_transient_ui(&mut self) {
        self.input_popovers.recommended_prompt = false;
        self.chat_think_hovered_idx = None;
        self.chat_tool_file_hovered = None;
        self.chat_tool_hovered_idx = None;
        self.tool_detail_dialog = None;
        self.chat_context_menu.target = None;
        self.chat_context_menu.position = None;
        self.chat_context_menu.text.clear();
        self.chat_reset_menu_idx = None;
        self.chat_fork_dialog = None;
        self.input_popovers.agent_sessions = false;
        self.popover_visibility.session_tool_selector = false;
        self.popover_visibility.session_actions = false;
    }

    pub fn rebuild_active_session_message_meta(&mut self) {
        self.active_session_view_state.rebuild_active_session_message_meta(&self.chat);
    }

    /// 将聊天尾部新增的消息增量纳入问题导航索引。
    pub(crate) fn sync_active_session_user_question_index(&mut self) {
        self.active_session_view_state.sync_active_session_user_question_index(&self.chat);
    }

    /// 只刷新流式增量影响的尾部消息元信息。
    pub(crate) fn refresh_active_session_tail_message_meta(&mut self, changed_idx: usize) {
        self.active_session_view_state
            .refresh_active_session_tail_message_meta(changed_idx, &self.chat);
    }

    pub(crate) fn clear_active_session_steps(&mut self) {
        self.active_session_view_state.clear_active_session_steps(
            crate::app::components::chat_panel::trajectory::project_trajectory,
        );
    }

    #[allow(dead_code)]
    pub(crate) fn replace_active_session_steps(&mut self, steps: Vec<ChatSessionStep>) {
        self.active_session_view_state.replace_active_session_steps(
            steps,
            crate::app::components::chat_panel::trajectory::project_trajectory,
        );
    }

    /// 执行 replace_active_session_calls 对应的领域操作。
    ///
    /// 原始调用记录来自会话 UI 快照，单独维护以避免历史加载时覆盖步骤索引。
    pub(crate) fn replace_active_session_calls(&mut self, calls: Vec<Value>) {
        self.active_session_view_state.replace_active_session_calls(calls);
    }

    /// 替换当前会话按稳定 turn/step 键保存的原始 LLM 载荷。
    pub(crate) fn replace_active_session_step_llm_raw(
        &mut self,
        raw: HashMap<SessionStepRawKey, Value>,
    ) {
        self.active_session_view_state.replace_active_session_step_llm_raw(
            raw,
            crate::app::components::chat_panel::trajectory::project_trajectory,
        );
    }

    /// 替换当前会话在存储层已存在的原始 LLM 稳定 turn/step 键。
    pub(crate) fn replace_active_session_llm_raw_step_indices(
        &mut self,
        indices: Vec<SessionStepRawKey>,
    ) {
        self.active_session_view_state.replace_active_session_llm_raw_step_indices(indices);
    }

    pub(crate) fn upsert_active_session_step(&mut self, step: ChatSessionStep) {
        self.active_session_view_state.upsert_active_session_step(
            step,
            crate::app::components::chat_panel::trajectory::project_trajectory,
        );
    }

    pub fn sync_active_session_from_chat(&mut self) {
        let updated_ms = self
            .active_session_id
            .as_deref()
            .and_then(|id| self.sessions.iter().find(|s| s.id == id))
            .map(|s| s.time.updated);
        self.active_session_view_state.sync_from_chat(&self.chat, updated_ms);
    }

    pub fn cache_active_session_chat(&mut self) {
        cache::cache_active_snapshot(
            self.active_session_id.as_deref(),
            &self.chat,
            &self.chat_message_ids,
            &self.session_runtime_states,
            &mut self.session_chat_cache,
            &mut self.session_chat_message_id_cache,
            &mut self.session_view_cache,
        );
    }

    pub fn prune_inactive_session_chat_cache(&mut self) {
        cache::prune_runtime_snapshots(
            &self.session_runtime_states,
            &mut self.session_chat_cache,
            &mut self.session_chat_message_id_cache,
        );
    }

    pub fn restore_chat_for_session(&mut self, session_id: &str) {
        let hit = cache::restore_snapshot(
            self.session_chat_cache.get(session_id),
            self.session_chat_message_id_cache.get(session_id),
            &mut self.chat,
            &mut self.chat_message_ids,
        );
        if hit {
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
        }
    }

    pub(crate) fn known_session_directory(&self, session_id: &str) -> Option<String> {
        cache::known_session_directory(
            session_id,
            &self.sessions,
            &self.project_sessions,
            &self.session_runtime_states,
        )
    }

    pub(crate) fn cached_chat_messages(&self, session_id: &str) -> Option<SharedChatMessages> {
        self.session_chat_cache.get(session_id).cloned()
    }

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

    pub(crate) fn session_history_reset_boundary(
        &self,
        session_id: &str,
    ) -> Option<&[Option<String>]> {
        self.session_history_reset_boundaries.get(session_id).map(Vec::as_slice)
    }

    pub(crate) fn active_shared_chat_messages(&self) -> SharedChatMessages {
        let cached =
            self.active_session_id.as_deref().and_then(|id| self.session_chat_cache.get(id));
        cache::active_shared_messages(cached, &self.chat)
    }

    pub(crate) fn store_session_chat_snapshot(
        &mut self,
        session_id: String,
        chat: SharedChatMessages,
        message_ids: Vec<Option<String>>,
    ) {
        let invalidated_view_cache = cache::store_snapshot(
            self.active_session_id.as_deref(),
            session_id.clone(),
            chat,
            message_ids,
            &mut self.session_chat_cache,
            &mut self.session_chat_message_id_cache,
            &mut self.session_view_cache,
        );
        if invalidated_view_cache {
            tracing::info!(
                target: "vw_desktop",
                session_id,
                message_count = self
                    .session_chat_cache
                    .get(&session_id)
                    .map_or(0, |messages| messages.len()),

                branch = "background_snapshot_changed",
                "invalidated stale session view cache"
            );
        }
    }
}

#[cfg(test)]
#[path = "active_tests.rs"]
mod active_tests;
