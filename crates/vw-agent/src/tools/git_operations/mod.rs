//! Host composition for the Git capability.
pub type GitOperationsTool = tool_git::GitOperationsTool<super::context::AgentContextHost>;

#[cfg(test)]
#[path = "../../../../libs/tool-git/src/git_operations/tests.rs"]
mod tests;
