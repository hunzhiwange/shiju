//! Tool inventory normalization and session permission decisions.
use desktop_settings_agents::state::merge_builtin_required_tools;
#[derive(Debug, Clone, Default)]
/// 表示 SessionToolInventory 相关的应用状态或派生数据。
pub struct SessionToolInventory {
    pub base_tools: Vec<String>,
    pub restricts_tools: bool,
}

pub fn sorted_unique_tools(mut tools: Vec<String>) -> Vec<String> {
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

pub fn intersect_tools(left: &[String], right: &[String]) -> Vec<String> {
    left.iter().filter(|tool| right.iter().any(|candidate| candidate == *tool)).cloned().collect()
}

pub fn request_allowed_tools_from_inventory(
    selector: &desktop_chat_tools::state::SessionToolSelectorState,
    inventory: &SessionToolInventory,
) -> Option<Vec<String>> {
    if !inventory.restricts_tools && !selector.has_tool_filter() {
        return None;
    }

    let allowed_tools = sorted_unique_tools(selector.filter_tools(&inventory.base_tools));
    (!allowed_tools.is_empty()).then_some(allowed_tools)
}

pub fn new_session_full_access(restored: bool, last: Option<bool>, default: bool) -> bool {
    !restored && last.unwrap_or(default)
}
pub fn session_full_access(
    config_ready: bool,
    runtime: Option<(bool, bool, bool)>,
    new_access: bool,
) -> bool {
    if config_ready
        && runtime.is_none_or(|(initialized, overridden, _)| !initialized && !overridden)
    {
        return new_access;
    }
    runtime.is_some_and(|(_, _, enabled)| enabled)
}

pub fn session_tool_inventory(
    available_tools: &[String],
    registered_tools: Option<&[String]>,
    static_tools: Option<Vec<String>>,
) -> SessionToolInventory {
    let available_tools = sorted_unique_tools(available_tools.to_vec());
    let registered_tools = registered_tools
        .map(<[String]>::to_vec)
        .map(sorted_unique_tools)
        .unwrap_or_else(|| available_tools.clone());
    let restricts_tools = static_tools.is_some();
    let base_tools = if let Some(static_tools) = static_tools {
        if registered_tools.is_empty() {
            static_tools
        } else {
            let effective_tools = merge_builtin_required_tools(&static_tools, &registered_tools);
            intersect_tools(&registered_tools, &effective_tools)
        }
    } else {
        registered_tools
    };

    SessionToolInventory { base_tools, restricts_tools }
}

pub fn allowed_tools_for_request(
    selector: &desktop_chat_tools::state::SessionToolSelectorState,
    inventory: &SessionToolInventory,
    registered_tools: &[String],
) -> Option<Vec<String>> {
    let mut allowed_tools = request_allowed_tools_from_inventory(selector, inventory);
    if let Some(allowed_tools) = &mut allowed_tools {
        if registered_tools.iter().any(|tool| tool.eq_ignore_ascii_case("AgentTool"))
            && !allowed_tools.iter().any(|tool| tool.eq_ignore_ascii_case("AgentTool"))
        {
            allowed_tools.push("AgentTool".to_string());
            allowed_tools.sort_unstable();
        }
    } else if inventory.restricts_tools
        && registered_tools.iter().any(|tool| tool.eq_ignore_ascii_case("AgentTool"))
    {
        allowed_tools = Some(vec!["AgentTool".to_string()]);
    }
    allowed_tools
}

/// Busy capability refreshes carry IDs; other reads keep the session locked.
pub fn session_has_started(
    lifecycle: &crate::message::chat::request::RequestLifecycle,
    options: &crate::message::chat::acp_options::AcpOptionsState,
    fork_request_submitted: bool,
    fork_execution_started: Option<bool>,
    active: bool,
    draft: bool,
    has_chat: bool,
) -> bool {
    let busy = lifecycle.is_requesting
        || lifecycle.active_agent_request.is_some()
        || !lifecycle.queue.is_empty()
        || (options.pending && options.source_request_id.is_none())
        || options.source_pending.is_some()
        || fork_request_submitted;
    busy || fork_execution_started.unwrap_or((active && !draft) || has_chat)
}

pub fn latest_history_model(steps: &[session_ui_types::ChatSessionStep]) -> Option<String> {
    steps
        .iter()
        .filter_map(|step| {
            let model = step.model.as_deref()?.trim();
            (!model.is_empty()).then_some((step.index, model))
        })
        .max_by_key(|(index, _)| *index)
        .map(|(_, model)| model.to_string())
}

pub async fn load_full_access(
    client: Option<vw_gateway_client::GatewayClient>,
    target: &str,
    directory: &str,
) -> Option<bool> {
    gateway_session::GatewaySessionClient::new(client?.transport().clone())
        .session_ui_summary(target, Some(directory))
        .await
        .ok()?
        .full_access
}

/// Merge project defaults into the empty draft without replacing entered task settings.
pub fn apply_draft_preferences<R>(
    runtime: &mut super::agent::SessionRuntimeState<R>,
    model: String,
    auto_model: bool,
    acp_agent: Option<String>,
) {
    if runtime.task_mode_executor.is_none() && runtime.task_mode_model == runtime.model {
        runtime.task_mode_model = model.clone();
    }
    runtime.model = model;
    runtime.auto_model = auto_model;
    if runtime.acp_agent != acp_agent {
        runtime.acp_agent = acp_agent;
        runtime.agent = None;
        runtime.agent_label = None;
        runtime.acp_model_source = Default::default();
        runtime.acp_options_state.options = Default::default();
        runtime.acp_options_state.pending = false;
        runtime.acp_options_state.model_input_draft = None;
        runtime.acp_options_state.source_pending = None;
        runtime.acp_options_state.source_request_id = None;
    }
}

pub fn restore_session_full_access<R>(
    runtime: &mut super::agent::SessionRuntimeState<R>,
    full_access: bool,
) {
    if !runtime.full_access_overridden && !runtime.fork_agent_selected {
        runtime.full_access_enabled = full_access;
        runtime.full_access_initialized = true;
    }
}

pub fn restore_session_model_from_history<R>(
    runtime: &mut super::agent::SessionRuntimeState<R>,
    model: &str,
) -> bool {
    let model = model.trim();
    if model.is_empty() {
        return false;
    }

    if !runtime.restore_model_from_history {
        return false;
    }

    runtime.model = model.to_string();
    runtime.auto_model = false;
    runtime.task_mode_model = runtime.model.clone();
    runtime.restore_model_from_history = false;
    true
}

pub fn mark_session_acp_rebuild_required<R>(runtime: &mut super::agent::SessionRuntimeState<R>) {
    runtime.acp_rebuild_required = true;
    runtime.last_effective_acp_agent = None;
}

use super::agent::SessionRuntimeState;
use crate::message::chat::request::AcpHistoryReplayMode;
use desktop_settings_agents::{agents_state::AgentsSettingsState, state::MAIN_AGENT_KEY};
use std::collections::HashMap;

/// Captured defaults for one selected session. Existing session values remain authoritative.
pub struct RuntimeDefaults {
    pub model: String,
    pub auto_model: bool,
    pub reasoning_effort: Option<vw_api_chat_identity::ReasoningEffortDto>,
    pub acp_history_mode: AcpHistoryReplayMode,
    pub acp_recent_count: usize,
    pub full_access: bool,
    pub config_ready: bool,
}
impl RuntimeDefaults {
    pub fn read<R: Default + Clone>(
        &self,
        stored: Option<&SessionRuntimeState<R>>,
    ) -> SessionRuntimeState<R> {
        let mut runtime = stored.cloned().unwrap_or_else(|| {
            let mut runtime =
                SessionRuntimeState::with_defaults(self.model.clone(), self.auto_model);
            runtime.reasoning_effort = self.reasoning_effort;
            runtime.acp_history_mode = self.acp_history_mode;
            runtime.acp_recent_count = self.acp_recent_count;
            runtime
        });
        runtime.full_access_enabled = session_full_access(
            self.config_ready,
            Some((
                runtime.full_access_initialized,
                runtime.full_access_overridden,
                runtime.full_access_enabled,
            )),
            self.full_access,
        );
        runtime
    }
    pub fn initialize<'a, R: Default>(
        &self,
        states: &'a mut HashMap<String, SessionRuntimeState<R>>,
        id: &str,
    ) -> &'a mut SessionRuntimeState<R> {
        let runtime = states.entry(id.to_owned()).or_insert_with(|| {
            let mut runtime =
                SessionRuntimeState::with_defaults(self.model.clone(), self.auto_model);
            runtime.reasoning_effort = self.reasoning_effort;
            runtime
        });
        if self.config_ready && !runtime.full_access_initialized && !runtime.full_access_overridden
        {
            runtime.full_access_enabled = self.full_access;
            runtime.full_access_initialized = true;
        }
        runtime.acp_history_mode = self.acp_history_mode;
        runtime.acp_recent_count = self.acp_recent_count;
        runtime
    }
}
pub fn selected_static_allowed_tools<R>(
    settings: &AgentsSettingsState,
    runtime: &SessionRuntimeState<R>,
) -> Option<Vec<String>> {
    let key = runtime.agent.as_deref().unwrap_or(MAIN_AGENT_KEY);
    Some(sorted_unique_tools(
        settings.entries.iter().find(|entry| entry.key == key)?.allowed_tools.clone(),
    ))
}
pub fn tool_inventory<R>(
    settings: &AgentsSettingsState,
    runtime: &SessionRuntimeState<R>,
) -> SessionToolInventory {
    session_tool_inventory(
        &settings.available_tools,
        settings.registered_tools.as_deref(),
        selected_static_allowed_tools(settings, runtime),
    )
}
pub fn request_tools<R>(
    settings: &AgentsSettingsState,
    runtime: &SessionRuntimeState<R>,
) -> Option<Vec<String>> {
    allowed_tools_for_request(
        &runtime.tool_selector,
        &tool_inventory(settings, runtime),
        settings.registered_tools.as_ref().unwrap_or(&settings.available_tools),
    )
}
pub fn reset_draft_full_access<R>(
    runtime: Option<&mut SessionRuntimeState<R>>,
    last: Option<bool>,
) {
    if let Some(runtime) = runtime
        && runtime.acp_agent.is_none()
    {
        runtime.full_access_initialized = last.is_some();
        runtime.full_access_overridden = false;
        runtime.full_access_enabled = last.unwrap_or(false);
    }
}
pub fn prepare_historical_session_model<R: Default>(
    defaults: RuntimeDefaults,
    states: &mut HashMap<String, SessionRuntimeState<R>>,
    id: &str,
) {
    if states.contains_key(id) {
        return;
    }
    let runtime = defaults.initialize(states, id);
    runtime.full_access_enabled = false;
    runtime.full_access_initialized = true;
    runtime.restore_model_from_history = true;
}
pub fn current_session_has_started<R>(
    runtime: Option<&SessionRuntimeState<R>>,
    active: bool,
    has_chat: bool,
) -> bool {
    match runtime {
        Some(runtime) => session_has_started(
            &runtime.request_lifecycle,
            &runtime.acp_options_state,
            runtime.fork_request_submitted,
            runtime.fork_execution_started,
            active,
            runtime.draft.is_some(),
            has_chat,
        ),
        None => active || has_chat,
    }
}
pub fn find_session_by_request_id<R>(
    states: &HashMap<String, SessionRuntimeState<R>>,
    id: u64,
) -> Option<String> {
    states
        .iter()
        .find(|(_, runtime)| {
            runtime.request_lifecycle.active_agent_request.as_ref().map(|r| r.id) == Some(id)
        })
        .map(|(session, _)| session.clone())
}

pub fn new_session_access(
    settings: &AgentsSettingsState,
    sessions: &[session_info::Info],
    projects: &HashMap<String, Vec<session_info::Info>>,
    id: &str,
    last: Option<bool>,
) -> bool {
    let restored =
        sessions.iter().chain(projects.values().flatten()).any(|session| session.id == id);
    new_session_full_access(
        restored,
        last,
        settings
            .entries
            .iter()
            .find(|entry| entry.key == MAIN_AGENT_KEY)
            .is_some_and(|entry| entry.defaults.full_access),
    )
}
pub fn mark_viewed<R>(runtime: &mut SessionRuntimeState<R>) {
    runtime.has_unseen_success = false;
}
pub fn cancel_model_history_restore<R>(runtime: Option<&mut SessionRuntimeState<R>>) {
    if let Some(runtime) = runtime {
        runtime.restore_model_from_history = false;
    }
}

pub fn read_runtime<R: Default + Clone>(
    stored: Option<&SessionRuntimeState<R>>,
    config_ready: bool,
    access: bool,
    defaults: impl FnOnce() -> RuntimeDefaults,
) -> SessionRuntimeState<R> {
    if let Some(stored) = stored {
        let mut runtime = stored.clone();
        runtime.full_access_enabled = session_full_access(
            config_ready,
            Some((
                runtime.full_access_initialized,
                runtime.full_access_overridden,
                runtime.full_access_enabled,
            )),
            access,
        );
        runtime
    } else {
        defaults().read(None)
    }
}
