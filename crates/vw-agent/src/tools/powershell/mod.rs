//! PowerShell execution is owned by tool-shell.
pub type PowerShellTool = tool_shell::PowerShellTool<crate::tools::context::AgentContextHost>;
#[cfg(test)]
mod tests;
