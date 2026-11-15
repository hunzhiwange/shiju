//! 宿主类型组合及依赖真实运行时的集成测试注册。
pub type CreateThreadTool = tool_agents::thread_tool::CreateThreadTool<super::agent_hosts::ToolAgentHost>;
pub type SendMessageToThreadTool = tool_agents::thread_tool::SendMessageToThreadTool<super::agent_hosts::ToolAgentHost>;
pub type ForkThreadTool = tool_agents::thread_tool::ForkThreadTool<super::agent_hosts::ToolAgentHost>;
pub type HandoffThreadTool = tool_agents::thread_tool::HandoffThreadTool<super::agent_hosts::ToolAgentHost>;
pub type ListThreadsTool = tool_agents::thread_tool::ListThreadsTool<super::agent_hosts::ToolAgentHost>;
pub type SetThreadPinnedTool = tool_agents::thread_tool::SetThreadPinnedTool<super::agent_hosts::ToolAgentHost>;
pub type ReadThreadTool = tool_agents::thread_tool::ReadThreadTool<super::agent_hosts::ToolAgentHost>;
pub type SetThreadTitleTool = tool_agents::thread_tool::SetThreadTitleTool<super::agent_hosts::ToolAgentHost>;
pub type SetThreadArchivedTool = tool_agents::thread_tool::SetThreadArchivedTool<super::agent_hosts::ToolAgentHost>;
pub type WaitThreadsTool = tool_agents::thread_tool::WaitThreadsTool<super::agent_hosts::ToolAgentHost>;
#[cfg(test)]
#[path = "../../../../libs/tool-agents/src/thread_tool/tests.rs"]
mod tests;
#[cfg(test)]
#[path = "../../../../libs/tool-agents/src/thread_tool/execution_tests.rs"]
mod execution_tests;
