//! 维护聊天会话的加载、运行态和 UI 分块派生逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::{App, EMPTY_SESSION_ID};
use crate::app::state::{
    MAIN_AGENT_KEY, SessionRuntimeState, SessionToolInventory,
};

use desktop_session::session::runtime::sorted_unique_tools;
impl App {
    fn new_session_full_access(&self, session_id: &str) -> bool {
        // Loading an existing session must not apply newly edited profile defaults.
        let restored = self.sessions.iter().any(|session| session.id == session_id)
            || self.project_sessions.values().flatten().any(|session| session.id == session_id);
        desktop_session::session::runtime::new_session_full_access(restored, self.last_llm_full_access,
            self.agents_settings.entries.iter().find(|entry| entry.key == MAIN_AGENT_KEY).is_some_and(|entry| entry.defaults.full_access))
    }

    fn session_full_access(&self, session_id: &str, runtime: Option<&SessionRuntimeState>) -> bool {
        desktop_session::session::runtime::session_full_access(
            self.agents_settings.config_ready,
            runtime.map(|r| (r.full_access_initialized, r.full_access_overridden, r.full_access_enabled)),
            self.new_session_full_access(session_id),
        )
    }

    pub(crate) fn current_session_full_access(&self) -> bool {
        self.session_full_access(
            self.active_session_id.as_deref().unwrap_or(EMPTY_SESSION_ID),
            self.current_session_runtime_ref(),
        )
    }

    pub(crate) fn restore_session_full_access(&mut self, session_id: &str, full_access: bool) {
        let runtime = self.get_session_runtime_mut(session_id);
        if !runtime.full_access_overridden && !runtime.fork_agent_selected {
            runtime.full_access_enabled = full_access;
            runtime.full_access_initialized = true;
        }
    }

    pub(crate) fn load_session_full_access(
        &self,
        session_id: &str,
    ) -> iced::Task<crate::app::Message> {
        let Some(directory) = self.known_session_directory(session_id) else {
            return iced::Task::none();
        };
        let session_id = session_id.to_owned();
        let target = session_id.clone();
        let client = crate::app::gateway_client();
        iced::Task::perform(
            async move {
                desktop_session::session::runtime::load_full_access(client.ok(), &target, &directory).await
            },
            move |result| {
                result.map_or(crate::app::Message::None, |full_access| {
                    crate::app::Message::Chat(
                        crate::app::message::ChatMessage::SessionFullAccessLoaded {
                            session_id: session_id.clone(),
                            full_access,
                        },
                    )
                })
            },
        )
    }

    pub(crate) fn reset_draft_full_access(&mut self) {
        if let Some(runtime) = self.session_runtime_states.get_mut(EMPTY_SESSION_ID)
            && runtime.acp_agent.is_none()
        {
            runtime.full_access_initialized = self.last_llm_full_access.is_some();
            runtime.full_access_overridden = false;
            runtime.full_access_enabled = self.last_llm_full_access.unwrap_or(false);
        }
    }

    fn selected_static_allowed_tools(&self, runtime: &SessionRuntimeState) -> Option<Vec<String>> {
        let selected_key = runtime.agent.as_deref().unwrap_or(MAIN_AGENT_KEY);
        let entry = self.agents_settings.entries.iter().find(|entry| entry.key == selected_key)?;
        Some(sorted_unique_tools(entry.allowed_tools.clone()))
    }

    fn default_session_model(&self) -> String {
        self.model.clone()
    }

    fn default_session_auto_model(&self) -> bool {
        self.auto_model
    }

    /// 执行 get_session_runtime 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn get_session_runtime(
        &self,
        session_id: &str,
    ) -> crate::app::state::SessionRuntimeState {
        #[cfg(test)]
        crate::app::components::chat_panel::performance::record_runtime_clone();
        let mut runtime =
            self.session_runtime_states.get(session_id).cloned().unwrap_or_else(|| {
                let mut runtime = crate::app::state::SessionRuntimeState::with_defaults(
                    self.default_session_model(),
                    self.default_session_auto_model(),
                );
                runtime.reasoning_effort =
                    self.session_reasoning_efforts.get(session_id).copied().flatten();
                runtime.acp_history_mode = self.acp_history_mode;
                runtime.acp_recent_count = self.acp_recent_count;
                runtime
            });
        runtime.full_access_enabled = self.session_full_access(session_id, Some(&runtime));
        runtime
    }

    /// 执行 get_session_runtime_mut 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn get_session_runtime_mut(
        &mut self,
        session_id: &str,
    ) -> &mut crate::app::state::SessionRuntimeState {
        let full_access = self.new_session_full_access(session_id);
        let config_ready = self.agents_settings.config_ready;
        let reasoning_effort = self.session_reasoning_efforts.get(session_id).copied().flatten();
        let model = self.default_session_model();
        let auto_model = self.default_session_auto_model();
        let acp_history_mode = self.acp_history_mode;
        let acp_recent_count = self.acp_recent_count;
        let runtime =
            self.session_runtime_states.entry(session_id.to_string()).or_insert_with(|| {
                let mut runtime =
                    crate::app::state::SessionRuntimeState::with_defaults(model, auto_model);
                runtime.reasoning_effort = reasoning_effort;
                runtime
            });
        if config_ready && !runtime.full_access_initialized && !runtime.full_access_overridden {
            runtime.full_access_enabled = full_access;
            runtime.full_access_initialized = true;
        }
        runtime.acp_history_mode = acp_history_mode;
        runtime.acp_recent_count = acp_recent_count;
        runtime
    }

    /// 执行 current_session_runtime 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn current_session_runtime(&self) -> crate::app::state::SessionRuntimeState {
        match &self.active_session_id {
            Some(id) => self.get_session_runtime(id),
            None => self.get_session_runtime(EMPTY_SESSION_ID),
        }
    }

    /// 执行 current_session_runtime_ref 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn current_session_runtime_ref(
        &self,
    ) -> Option<&crate::app::state::SessionRuntimeState> {
        if let Some(id) = &self.active_session_id {
            self.session_runtime_states.get(id)
        } else {
            self.session_runtime_states.get(EMPTY_SESSION_ID)
        }
    }

    /// 执行 current_session_runtime_mut 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn current_session_runtime_mut(
        &mut self,
    ) -> &mut crate::app::state::SessionRuntimeState {
        if let Some(id) = self.active_session_id.clone() {
            self.get_session_runtime_mut(&id)
        } else {
            self.get_session_runtime_mut(EMPTY_SESSION_ID)
        }
    }

    pub(crate) fn current_session_has_started(&self) -> bool {
        let runtime = self.current_session_runtime_ref();
        match runtime {
            Some(runtime) => desktop_session::session::runtime::session_has_started(
                &runtime.request_lifecycle, &runtime.acp_options_state,
                runtime.fork_request_submitted, runtime.fork_execution_started,
                self.active_session_id.is_some(), runtime.draft.is_some(), !self.chat.is_empty(),
            ),
            None => self.active_session_id.is_some() || !self.chat.is_empty(),
        }
    }

    /// 执行 mark_session_viewed 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn mark_session_viewed(&mut self, session_id: &str) {
        let runtime = self.get_session_runtime_mut(session_id);
        runtime.has_unseen_success = false;
    }

    /// 执行 session_tool_inventory 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn session_tool_inventory(
        &self,
        runtime: &SessionRuntimeState,
    ) -> SessionToolInventory {
        desktop_session::session::runtime::session_tool_inventory(
            &self.agents_settings.available_tools,
            self.agents_settings.registered_tools.as_deref(),
            self.selected_static_allowed_tools(runtime),
        )
    }

    pub(crate) fn session_allowed_tools_for_request(
        &self,
        runtime: &SessionRuntimeState,
    ) -> Option<Vec<String>> {
        let inventory = self.session_tool_inventory(runtime);
        desktop_session::session::runtime::allowed_tools_for_request(
            &runtime.tool_selector, &inventory,
            self.agents_settings.registered_tools.as_ref().unwrap_or(&self.agents_settings.available_tools),
        )
    }

    /// 执行 mark_active_session_viewed 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn mark_active_session_viewed(&mut self) {
        if let Some(id) = self.active_session_id.clone() {
            self.mark_session_viewed(&id);
        }
    }

    /// 执行 current_session_is_requesting 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn current_session_is_requesting(&self) -> bool {
        self.current_session_runtime_ref().is_some_and(|runtime| runtime.request_lifecycle.is_requesting)
    }

    /// 执行 session_is_requesting 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn session_is_requesting(&self, session_id: &str) -> bool {
        self.session_runtime_states.get(session_id).map(|s| s.request_lifecycle.is_requesting).unwrap_or(false)
    }

    /// 执行 any_session_requesting 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn any_session_requesting(&self) -> bool {
        self.session_runtime_states.values().any(|s| s.request_lifecycle.is_requesting)
    }

    /// 执行 has_active_explore_summary_animation 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn has_active_explore_summary_animation(&self) -> bool {
        desktop_chat_view::stream_render::has_active_explore_summary_animation(
            &self.chat_explore_summary_animations, crate::app::time::now_ms(),
            crate::app::components::status_animation::EXPLORE_SUMMARY_FLIP_DURATION_MS,
        )
    }

    /// 执行 advance_status_animation_frame 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn advance_status_animation_frame(&mut self) {
        self.status_animation_frame = self.status_animation_frame.wrapping_add(1);

        desktop_chat_view::stream_render::advance_explore_summary_animation(
            &mut self.chat_explore_summary_animations, crate::app::time::now_ms(),
            crate::app::components::status_animation::EXPLORE_SUMMARY_FLIP_DURATION_MS,
        );
    }

    /// 执行 find_session_by_request_id 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn find_session_by_request_id(&self, request_id: u64) -> Option<String> {
        for (session_id, runtime) in &self.session_runtime_states {
            if runtime.request_lifecycle.active_agent_request.as_ref().map(|r| r.id) == Some(request_id) {
                return Some(session_id.clone());
            }
        }
        None
    }

    /// 执行 sync_active_session_preferences 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn sync_active_session_preferences(&mut self) {
        self.input_popovers.recommended_prompt = false;
        let Some(session_id) = self.active_session_id.clone() else {
            return;
        };
        let runtime = self.get_session_runtime(&session_id);
        self.reasoning_effort = runtime.reasoning_effort;
        self.acp_agent = runtime.acp_agent;
        self.acp_history_mode = runtime.acp_history_mode;
        self.acp_recent_count = runtime.acp_recent_count;
    }

    /// 首次打开历史会话时，以全局最后选择占位并等待恢复会话自己的模型。
    pub(crate) fn prepare_historical_session_model(&mut self, session_id: &str) {
        if self.session_runtime_states.contains_key(session_id) {
            return;
        }

        let runtime = self.get_session_runtime_mut(session_id);
        runtime.full_access_enabled = false;
        runtime.full_access_initialized = true;
        runtime.restore_model_from_history = true;
    }

    /// 用最后一个有模型记录的历史步骤初始化会话模型。
    pub(crate) fn restore_session_model_from_steps(
        &mut self,
        session_id: &str,
        steps: &[crate::app::models::ChatSessionStep],
    ) -> bool {
        let Some(model) = desktop_session::session::runtime::latest_history_model(steps)
        else {
            return false;
        };

        self.restore_session_model_from_history(session_id, &model)
    }

    /// 用历史记录中的模型初始化等待恢复的会话。
    pub(crate) fn restore_session_model_from_history(
        &mut self,
        session_id: &str,
        model: &str,
    ) -> bool {
        let model = model.trim();
        if model.is_empty() {
            return false;
        }

        let Some(runtime) = self.session_runtime_states.get_mut(session_id) else {
            return false;
        };
        if !runtime.restore_model_from_history {
            return false;
        }

        runtime.model = model.to_string();
        runtime.auto_model = false;
        runtime.task_mode_model = runtime.model.clone();
        runtime.restore_model_from_history = false;
        true
    }

    /// 用户显式选择模型后，历史加载结果不得再覆盖该选择。
    pub(crate) fn cancel_session_model_history_restore(&mut self, session_id: &str) {
        if let Some(runtime) = self.session_runtime_states.get_mut(session_id) {
            runtime.restore_model_from_history = false;
        }
    }

    /// 执行 mark_session_acp_rebuild_required 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn mark_session_acp_rebuild_required(&mut self, session_id: &str) {
        let runtime = self.get_session_runtime_mut(session_id);
        runtime.acp_rebuild_required = true;
        runtime.last_effective_acp_agent = None;
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod runtime_tests;
