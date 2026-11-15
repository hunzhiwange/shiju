//! Host composition for the Git capability.
pub type ExitWorktreeTool = tool_git::ExitWorktreeTool<super::context::AgentContextHost>;

#[cfg(test)]
#[path = "../../../../libs/tool-git/src/exit_worktree/tests.rs"]
mod tests;
