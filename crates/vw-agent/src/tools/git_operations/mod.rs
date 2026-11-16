//! Host composition for the Git capability.
pub type GitOperationsTool = tool_git::GitOperationsTool<super::context::AgentContextHost>;

#[cfg(test)]
mod tests;
