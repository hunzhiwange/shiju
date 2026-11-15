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

    let allowed_tools =
        sorted_unique_tools(selector.filter_tools(&inventory.base_tools));
    (!allowed_tools.is_empty()).then_some(allowed_tools)
}


pub fn new_session_full_access(restored: bool, last: Option<bool>, default: bool) -> bool {
    !restored && last.unwrap_or(default)
}
pub fn session_full_access(config_ready: bool, runtime: Option<(bool, bool, bool)>, new_access: bool) -> bool {
    if config_ready && runtime.is_none_or(|(initialized, overridden, _)| !initialized && !overridden) { return new_access; }
    runtime.is_some_and(|(_, _, enabled)| enabled)
}

pub fn session_tool_inventory(available_tools: &[String], registered_tools: Option<&[String]>, static_tools: Option<Vec<String>>) -> SessionToolInventory {
        let available_tools = sorted_unique_tools(available_tools.to_vec());
        let registered_tools = registered_tools.map(<[String]>::to_vec)
            .map(sorted_unique_tools)
            .unwrap_or_else(|| available_tools.clone());
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

pub fn allowed_tools_for_request(selector: &desktop_chat_tools::state::SessionToolSelectorState, inventory: &SessionToolInventory, registered_tools: &[String]) -> Option<Vec<String>> {
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
    let busy = lifecycle.is_requesting || lifecycle.active_agent_request.is_some()
        || !lifecycle.queue.is_empty()
        || (options.pending && options.source_request_id.is_none())
        || options.source_pending.is_some() || fork_request_submitted;
    busy || fork_execution_started.unwrap_or((active && !draft) || has_chat)
}

pub fn latest_history_model(steps: &[session_ui_types::ChatSessionStep]) -> Option<String> {
    steps.iter().filter_map(|step| {
        let model = step.model.as_deref()?.trim();
        (!model.is_empty()).then_some((step.index, model))
    }).max_by_key(|(index, _)| *index).map(|(_, model)| model.to_string())
}

pub async fn load_full_access(client: Option<vw_gateway_client::GatewayClient>, target: &str, directory: &str) -> Option<bool> {
    gateway_session::GatewaySessionClient::new(client?.transport().clone())
        .session_ui_summary(target, Some(directory)).await.ok()?.full_access
}
