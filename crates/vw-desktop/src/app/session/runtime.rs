//! 维护聊天会话的加载、运行态和 UI 分块派生逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::{App, EMPTY_SESSION_ID};
use crate::app::state::{
    MAIN_AGENT_KEY, SessionRuntimeState, SessionToolInventory, merge_builtin_required_tools,
};

fn sorted_unique_tools(mut tools: Vec<String>) -> Vec<String> {
    tools = tools
        .into_iter()
        .filter_map(|tool| {
            let trimmed = tool.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .collect();
    tools.sort();
    tools.dedup();
    tools
}

fn intersect_tools(left: &[String], right: &[String]) -> Vec<String> {
    left.iter().filter(|tool| right.iter().any(|candidate| candidate == *tool)).cloned().collect()
}

pub(crate) fn request_allowed_tools_from_inventory(
    runtime: &SessionRuntimeState,
    inventory: &SessionToolInventory,
) -> Option<Vec<String>> {
    if !inventory.restricts_tools && !runtime.tool_selector.has_tool_filter() {
        return None;
    }

    let allowed_tools =
        sorted_unique_tools(runtime.tool_selector.filter_tools(&inventory.base_tools));
    (!allowed_tools.is_empty()).then_some(allowed_tools)
}

impl App {
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
        self.session_runtime_states.get(session_id).cloned().unwrap_or_else(|| {
            let mut runtime = crate::app::state::SessionRuntimeState::with_defaults(
                self.default_session_model(),
                self.default_session_auto_model(),
            );
            runtime.acp_agent = self.acp_agent.clone();
            runtime.acp_history_mode = self.acp_history_mode;
            runtime.acp_recent_count = self.acp_recent_count;
            runtime
        })
    }

    /// 执行 get_session_runtime_mut 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub(crate) fn get_session_runtime_mut(
        &mut self,
        session_id: &str,
    ) -> &mut crate::app::state::SessionRuntimeState {
        let model = self.default_session_model();
        let auto_model = self.default_session_auto_model();
        let acp_agent = self.acp_agent.clone();
        let acp_history_mode = self.acp_history_mode;
        let acp_recent_count = self.acp_recent_count;
        let runtime =
            self.session_runtime_states.entry(session_id.to_string()).or_insert_with(|| {
                crate::app::state::SessionRuntimeState::with_defaults(model, auto_model)
            });
        if runtime.acp_agent.is_none() {
            runtime.acp_agent = acp_agent;
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
        let runtime = self.current_session_runtime();
        !self.chat.is_empty()
            || runtime.active_agent_request.is_some()
            || !runtime.queue.is_empty()
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
        let available_tools = sorted_unique_tools(self.agents_settings.available_tools.clone());
        let registered_tools = self
            .agents_settings
            .registered_tools
            .clone()
            .map(sorted_unique_tools)
            .unwrap_or_else(|| available_tools.clone());
        let static_tools = self.selected_static_allowed_tools(runtime);
        let restricts_tools = static_tools.is_some();
        let base_tools = if let Some(static_tools) = static_tools {
            if registered_tools.is_empty() {
                static_tools
            } else {
                let effective_tools =
                    merge_builtin_required_tools(&static_tools, &registered_tools);
                intersect_tools(&registered_tools, &effective_tools)
            }
        } else {
            registered_tools
        };

        SessionToolInventory { base_tools, restricts_tools }
    }

    pub(crate) fn session_allowed_tools_for_request(
        &self,
        runtime: &SessionRuntimeState,
    ) -> Option<Vec<String>> {
        let inventory = self.session_tool_inventory(runtime);
        request_allowed_tools_from_inventory(runtime, &inventory)
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
        self.current_session_runtime().is_requesting
    }

    /// 执行 session_is_requesting 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn session_is_requesting(&self, session_id: &str) -> bool {
        self.session_runtime_states.get(session_id).map(|s| s.is_requesting).unwrap_or(false)
    }

    /// 执行 any_session_requesting 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn any_session_requesting(&self) -> bool {
        self.session_runtime_states.values().any(|s| s.is_requesting)
    }

    /// 执行 has_active_explore_summary_animation 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn has_active_explore_summary_animation(&self) -> bool {
        let now_ms = crate::app::time::now_ms();
        self.chat_explore_summary_animations.values().any(|state| {
            state.changed_at_ms.is_some_and(|changed_at_ms| {
                now_ms.saturating_sub(changed_at_ms)
                    < crate::app::components::status_animation::EXPLORE_SUMMARY_FLIP_DURATION_MS
            })
        })
    }

    /// 执行 advance_status_animation_frame 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn advance_status_animation_frame(&mut self) {
        self.status_animation_frame = self.status_animation_frame.wrapping_add(1);

        let now_ms = crate::app::time::now_ms();
        for state in self.chat_explore_summary_animations.values_mut() {
            if let Some(changed_at_ms) = state.changed_at_ms
                && now_ms.saturating_sub(changed_at_ms)
                    >= crate::app::components::status_animation::EXPLORE_SUMMARY_FLIP_DURATION_MS
            {
                state.previous_summary_text = state.current_summary_text.clone();
                state.changed_at_ms = None;
            }
        }
    }

    /// 执行 find_session_by_request_id 对应的领域操作。
    ///
    /// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
    /// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
    pub fn find_session_by_request_id(&self, request_id: u64) -> Option<String> {
        for (session_id, runtime) in &self.session_runtime_states {
            if runtime.active_agent_request.as_ref().map(|r| r.id) == Some(request_id) {
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
        runtime.restore_model_from_history = true;
    }

    /// 用最后一个有模型记录的历史步骤初始化会话模型。
    pub(crate) fn restore_session_model_from_steps(
        &mut self,
        session_id: &str,
        steps: &[crate::app::models::ChatSessionStep],
    ) -> bool {
        let Some(model) = steps
            .iter()
            .filter_map(|step| {
                let model = step.model.as_deref()?.trim();
                (!model.is_empty()).then_some((step.index, model))
            })
            .max_by_key(|(index, _)| *index)
            .map(|(_, model)| model.to_string())
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
