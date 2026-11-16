//! 规划与执行会话的处理器选项转换。
use crate::SessionRunRequest;
use serde_json::{Map, Value, json};

/// 构建规划/执行子会话的 processor options。
pub fn gateway_session_options(
    request: &SessionRunRequest,
    acp_full_access: bool,
) -> Map<String, Value> {
    let acp_agent = selected_acp_agent(request.acp_agent.as_deref());
    let mut options = Map::new();
    options.insert("full_access".to_string(), json!(acp_agent.is_none() || acp_full_access));
    options.insert("cwd".to_string(), json!(request.root.as_str()));
    options.insert("goal_mode".to_string(), json!(true));
    options.insert("workflow_mode".to_string(), json!(true));

    if let Some(agent) = request.agent.as_ref().filter(|value| !value.trim().is_empty()) {
        options.insert("agent".to_string(), json!(agent));
    }

    let acp_enabled = acp_agent.is_some();
    options.insert("acp_test".to_string(), json!(acp_enabled));
    if let Some(acp_agent) = acp_agent {
        options.insert("acp_agent".to_string(), json!(acp_agent));
        options.insert(
            "acp_permission_mode".to_string(),
            json!(if acp_full_access { "approve-all" } else { "approve-reads" }),
        );
        options.insert("acp_non_interactive_permissions".to_string(), json!("deny"));
        options.insert("acp_force_new_session".to_string(), json!(true));
        options.insert("acp_close_client_after_request".to_string(), json!(true));
        options.insert("acp_history_strategy".to_string(), json!("discard"));
        options.insert("acp_history_recent_count".to_string(), json!(1));
    }

    options
}

fn selected_acp_agent(agent: Option<&str>) -> Option<&str> {
    agent.map(str::trim).filter(|agent| !agent.is_empty())
}
