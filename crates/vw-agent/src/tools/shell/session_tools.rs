//! Bind shell session tools to the real agent context.
pub type BashWaitTool = tool_shell::shell::session_tools::BashWaitTool<crate::tools::context::AgentContextHost>;
pub type BashCancelTool = tool_shell::shell::session_tools::BashCancelTool<crate::tools::context::AgentContextHost>;
