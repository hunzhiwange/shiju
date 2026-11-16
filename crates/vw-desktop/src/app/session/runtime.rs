//! 维护聊天会话的加载、运行态和 UI 分块派生逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::{App, EMPTY_SESSION_ID};
use crate::app::state::{SessionRuntimeState, SessionToolInventory};

impl App {
    fn session_runtime_defaults(
        &self,
        session_id: &str,
    ) -> desktop_session::session::runtime::RuntimeDefaults {
        desktop_session::session::runtime::RuntimeDefaults {
            model: self.session_state.model.clone(),
            auto_model: self.session_state.auto_model,
            reasoning_effort: self.session_state.session_reasoning_efforts.get(session_id).copied().flatten(),
            acp_history_mode: self.session_state.acp_history_mode,
            acp_recent_count: self.session_state.acp_recent_count,
            full_access: self.new_session_full_access(session_id),
            config_ready: self.agents_settings.config_ready,
        }
    }

    fn new_session_full_access(&self, session_id: &str) -> bool {
        desktop_session::session::runtime::new_session_access(
            &self.agents_settings,
            &self.session_state.sessions,
            &self.workspace_state.project_sessions,
            session_id,
            self.session_state.last_llm_full_access,
        )
    }

    fn session_full_access(&self, session_id: &str, runtime: Option<&SessionRuntimeState>) -> bool {
        desktop_session::session::runtime::session_full_access(
            self.agents_settings.config_ready,
            runtime.map(|r| {
                (r.full_access_initialized, r.full_access_overridden, r.full_access_enabled)
            }),
            self.new_session_full_access(session_id),
        )
    }

    pub(crate) fn current_session_full_access(&self) -> bool {
        self.session_full_access(
            self.session_state.active_session_id.as_deref().unwrap_or(EMPTY_SESSION_ID),
            self.current_session_runtime_ref(),
        )
    }

    pub(crate) fn restore_session_full_access(&mut self, session_id: &str, full_access: bool) {
        desktop_session::session::runtime::restore_session_full_access(
            self.get_session_runtime_mut(session_id),
            full_access,
        );
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
                desktop_session::session::runtime::load_full_access(
                    client.ok(),
                    &target,
                    &directory,
                )
                .await
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
        desktop_session::session::runtime::reset_draft_full_access(
            self.session_state.session_runtime_states.get_mut(EMPTY_SESSION_ID),
            self.session_state.last_llm_full_access,
        );
    }

    pub(crate) fn get_session_runtime(
        &self,
        session_id: &str,
    ) -> crate::app::state::SessionRuntimeState {
        #[cfg(test)]
        crate::app::components::chat_panel::performance::record_runtime_clone();
        desktop_session::session::runtime::read_runtime(
            self.session_state.session_runtime_states.get(session_id),
            self.agents_settings.config_ready,
            self.new_session_full_access(session_id),
            || self.session_runtime_defaults(session_id),
        )
    }

    pub(crate) fn get_session_runtime_mut(
        &mut self,
        session_id: &str,
    ) -> &mut crate::app::state::SessionRuntimeState {
        self.session_runtime_defaults(session_id)
            .initialize(&mut self.session_state.session_runtime_states, session_id)
    }

    pub(crate) fn current_session_runtime(&self) -> crate::app::state::SessionRuntimeState {
        match &self.session_state.active_session_id {
            Some(id) => self.get_session_runtime(id),
            None => self.get_session_runtime(EMPTY_SESSION_ID),
        }
    }

    pub(crate) fn current_session_runtime_ref(
        &self,
    ) -> Option<&crate::app::state::SessionRuntimeState> {
        if let Some(id) = &self.session_state.active_session_id {
            self.session_state.session_runtime_states.get(id)
        } else {
            self.session_state.session_runtime_states.get(EMPTY_SESSION_ID)
        }
    }

    pub(crate) fn current_session_runtime_mut(
        &mut self,
    ) -> &mut crate::app::state::SessionRuntimeState {
        if let Some(id) = self.session_state.active_session_id.clone() {
            self.get_session_runtime_mut(&id)
        } else {
            self.get_session_runtime_mut(EMPTY_SESSION_ID)
        }
    }

    pub(crate) fn current_session_has_started(&self) -> bool {
        desktop_session::session::runtime::current_session_has_started(
            self.current_session_runtime_ref(),
            self.session_state.active_session_id.is_some(),
            !self.chat_view.chat.is_empty(),
        )
    }

    pub(crate) fn mark_session_viewed(&mut self, session_id: &str) {
        desktop_session::session::runtime::mark_viewed(self.get_session_runtime_mut(session_id));
    }

    pub(crate) fn session_tool_inventory(
        &self,
        runtime: &SessionRuntimeState,
    ) -> SessionToolInventory {
        desktop_session::session::runtime::tool_inventory(&self.agents_settings, runtime)
    }

    pub(crate) fn session_allowed_tools_for_request(
        &self,
        runtime: &SessionRuntimeState,
    ) -> Option<Vec<String>> {
        desktop_session::session::runtime::request_tools(&self.agents_settings, runtime)
    }

    pub(crate) fn mark_active_session_viewed(&mut self) {
        if let Some(id) = self.session_state.active_session_id.clone() {
            self.mark_session_viewed(&id);
        }
    }

    pub fn current_session_is_requesting(&self) -> bool {
        self.current_session_runtime_ref()
            .is_some_and(|runtime| runtime.request_lifecycle.is_requesting)
    }

    pub fn session_is_requesting(&self, session_id: &str) -> bool {
        self.session_state.session_runtime_states
            .get(session_id)
            .map(|s| s.request_lifecycle.is_requesting)
            .unwrap_or(false)
    }

    pub fn any_session_requesting(&self) -> bool {
        self.session_state.session_runtime_states.values().any(|s| s.request_lifecycle.is_requesting)
    }

    pub fn has_active_explore_summary_animation(&self) -> bool {
        desktop_chat_view::stream_render::has_active_explore_summary_animation(
            &self.chat_interaction.chat_explore_summary_animations,
            crate::app::time::now_ms(),
            crate::app::components::status_animation::EXPLORE_SUMMARY_FLIP_DURATION_MS,
        )
    }

    pub fn advance_status_animation_frame(&mut self) {
        desktop_chat_view::stream_render::advance_status_animation_frame(
            &mut self.status_animation_frame,
            &mut self.chat_interaction.chat_explore_summary_animations,
            crate::app::time::now_ms(),
            crate::app::components::status_animation::EXPLORE_SUMMARY_FLIP_DURATION_MS,
        );
    }

    pub fn find_session_by_request_id(&self, request_id: u64) -> Option<String> {
        desktop_session::session::runtime::find_session_by_request_id(
            &self.session_state.session_runtime_states,
            request_id,
        )
    }

    pub fn sync_active_session_preferences(&mut self) {
        self.session_state.input_popovers.recommended_prompt = false;
        let Some(session_id) = self.session_state.active_session_id.clone() else {
            return;
        };
        let runtime = self.get_session_runtime(&session_id);
        self.session_state.reasoning_effort = runtime.reasoning_effort;
        self.session_state.acp_agent = runtime.acp_agent;
        self.session_state.acp_history_mode = runtime.acp_history_mode;
        self.session_state.acp_recent_count = runtime.acp_recent_count;
    }

    /// 首次打开历史会话时，以全局最后选择占位并等待恢复会话自己的模型。
    pub(crate) fn prepare_historical_session_model(&mut self, session_id: &str) {
        desktop_session::session::runtime::prepare_historical_session_model(
            self.session_runtime_defaults(session_id),
            &mut self.session_state.session_runtime_states,
            session_id,
        );
    }

    /// 用最后一个有模型记录的历史步骤初始化会话模型。
    pub(crate) fn restore_session_model_from_steps(
        &mut self,
        session_id: &str,
        steps: &[crate::app::models::ChatSessionStep],
    ) -> bool {
        let Some(model) = desktop_session::session::runtime::latest_history_model(steps) else {
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
        let Some(runtime) = self.session_state.session_runtime_states.get_mut(session_id) else {
            return false;
        };
        desktop_session::session::runtime::restore_session_model_from_history(runtime, model)
    }

    /// 用户显式选择模型后，历史加载结果不得再覆盖该选择。
    pub(crate) fn cancel_session_model_history_restore(&mut self, session_id: &str) {
        desktop_session::session::runtime::cancel_model_history_restore(
            self.session_state.session_runtime_states.get_mut(session_id),
        );
    }

    pub(crate) fn mark_session_acp_rebuild_required(&mut self, session_id: &str) {
        desktop_session::session::runtime::mark_session_acp_rebuild_required(
            self.get_session_runtime_mut(session_id),
        );
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod runtime_tests;
