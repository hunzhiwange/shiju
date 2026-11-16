//! 宿主具体类型组合与集成测试注册。
pub type SessionTool = tool_agents::session_tool::SessionTool<super::agent_hosts::ToolAgentHost>;
#[cfg(test)]
use session_port::chat_stream::GatewayChatStreamCollected;
#[cfg(test)]
fn persist_task_state(coordinator: &crate::session::task_session::TaskSessionCoordinator, tenant: &vw_api_transport::id::TenantScope, parent: &str, task: &str, project: &str) -> anyhow::Result<()> {
    tool_agents::session_tool::persist_task_state::<super::agent_hosts::ToolAgentHost>(&coordinator.clone().into(), tenant, parent, task, project)
}
#[cfg(test)]
#[path = "../../../../libs/tool-agents/src/session_tool/tests.rs"]
mod tests;
