//! Structured Git operations and session worktree tools.
pub mod enter_worktree;
pub mod exit_worktree;
pub mod git_operations;
pub use enter_worktree::EnterWorktreeTool;
pub use exit_worktree::ExitWorktreeTool;
pub use git_operations::GitOperationsTool;

/// Private implementation checks mounted by the host's existing test suite.
#[cfg(feature = "test-support")]
#[doc(hidden)]
pub mod testing {
    use super::*;
    use tool_context::host::ContextHost;
    pub fn truncate_commit_message<H: ContextHost>(message: &str) -> String {
        GitOperationsTool::<H>::truncate_commit_message(message)
    }
    pub async fn git_checkout<H: ContextHost>(tool: &GitOperationsTool<H>, args: serde_json::Value) -> anyhow::Result<tool_types::ToolResult> {
        tool.git_checkout(args).await
    }
}

/// Composition action required by the existing worktree service before lifecycle operations.
pub trait GitToolHost: tool_context::host::ContextHost {
    #[cfg(not(target_arch = "wasm32"))]
    fn initialize_worktree_host();
}
