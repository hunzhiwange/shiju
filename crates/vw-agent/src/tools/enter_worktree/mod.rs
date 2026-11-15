//! Host composition for the Git capability.
pub type EnterWorktreeTool = tool_git::EnterWorktreeTool<super::context::AgentContextHost>;

#[cfg(test)]
#[path = "../../../../libs/tool-git/src/enter_worktree/tests.rs"]
mod tests;
