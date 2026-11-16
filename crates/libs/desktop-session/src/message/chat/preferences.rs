//! 会话模型和智能体切换的局部状态转换。
use crate::session::agent::SessionRuntimeState;
pub fn select_agent<R>(
    runtime: &mut SessionRuntimeState<R>,
    agent: Option<String>,
    label: Option<String>,
    acp_agent: Option<String>,
    full_access: bool,
) {
    if !runtime.full_access_overridden {
        runtime.full_access_enabled = full_access;
    }
    runtime.full_access_initialized = true;
    runtime.agent = super::session::normalize_delegate_agent(agent);
    runtime.agent_label = label;
    runtime.acp_agent = acp_agent.clone();
    if runtime.fork_execution_started == Some(false) {
        runtime.fork_agent_selected = true;
        runtime.restore_model_from_history = false;
    }
    runtime.acp_options_state.model_input_draft = None;
    runtime.acp_options_state.source_pending = None;
    runtime.acp_options_state.source_request_id = None;
    runtime.acp_options_state.open = false;
    runtime.last_effective_acp_agent = None;
    runtime.acp_rebuild_required = acp_agent.is_some();
    runtime.acp_options_state.options = Default::default();
    runtime.acp_options_state.pending = false;
    runtime.acp_model_source = Default::default();
    runtime.tool_selector.reset();
}
pub fn select_model<R>(
    runtime: &mut SessionRuntimeState<R>,
    model: &str,
    input: bool,
) -> (String, bool) {
    runtime.model = super::session::normalize_model(model);
    runtime.auto_model = input && runtime.model == "auto";
    runtime.task_mode_model = runtime.model.clone();
    (runtime.model.clone(), runtime.auto_model)
}
pub fn toggle_auto_model<R>(runtime: &mut SessionRuntimeState<R>, enabled: bool) -> (String, bool) {
    runtime.auto_model = enabled;
    if enabled {
        runtime.task_mode_model = "auto".to_string();
    }
    (runtime.model.clone(), runtime.auto_model)
}

pub fn prepare_tool_selection<R>(runtime: &mut SessionRuntimeState<R>, started: bool) {
    if !started && !runtime.fork_agent_selected {
        runtime.agent = None;
    }
}

pub fn is_plan_run_child<'a>(
    session_id: Option<&str>,
    runtime_plan_run: bool,
    sessions: impl Iterator<Item = &'a session_info::Info>,
) -> bool {
    let Some(id) = session_id else {
        return false;
    };
    runtime_plan_run
        || sessions
            .filter(|session| session.id == id)
            .next()
            .is_some_and(session_info::Info::is_plan_run)
}

pub fn acp_history_preferences(
    mode: super::request::AcpHistoryReplayMode,
    recent_count: usize,
) -> serde_json::Map<String, serde_json::Value> {
    serde_json::Map::from_iter([
        ("acp_history_strategy".to_string(), serde_json::json!(mode.as_str())),
        ("acp_history_recent_count".to_string(), serde_json::json!(recent_count.clamp(1, 20))),
    ])
}
