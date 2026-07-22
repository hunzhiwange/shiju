//! 维护聊天会话的加载、运行态和 UI 分块派生逻辑。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use super::{App, EMPTY_SESSION_ID};
use crate::app::state::{MAIN_AGENT_KEY, SessionRuntimeState, SessionToolInventory};

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
        let allowed_tools = sorted_unique_tools(entry.allowed_tools.clone());
        (!allowed_tools.is_empty()).then_some(allowed_tools)
    }

    fn default_session_model(&self) -> String {
        self.active_session_id
            .as_ref()
            .and_then(|id| self.session_runtime_states.get(id))
            .map(|s| s.model.clone())
            .unwrap_or_else(|| self.model.clone())
    }

    fn default_session_auto_model(&self) -> bool {
        self.active_session_id
            .as_ref()
            .and_then(|id| self.session_runtime_states.get(id))
            .map(|s| s.auto_model)
            .unwrap_or(self.auto_model)
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
        let static_tools = self.selected_static_allowed_tools(runtime);
        let restricts_tools = static_tools.is_some();
        let base_tools = if let Some(static_tools) = static_tools {
            if available_tools.is_empty() {
                static_tools
            } else {
                let merged = intersect_tools(&available_tools, &static_tools);
                if merged.is_empty() { static_tools } else { merged }
            }
        } else {
            available_tools.clone()
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
        self.model = runtime.model;
        self.auto_model = runtime.auto_model;
        self.reasoning_effort = runtime.reasoning_effort;
        self.acp_agent = runtime.acp_agent;
        self.acp_history_mode = runtime.acp_history_mode;
        self.acp_recent_count = runtime.acp_recent_count;
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
