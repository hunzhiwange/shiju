//! File and compatibility contracts owned by the tool domain.
mod host;
use host::Host;
#[cfg(test)]
type ToolUseContext = tool_context::context::ToolUseContext<Host>;
#[cfg(test)]
async fn scope_tool_use_context<F: std::future::Future>(context: std::sync::Arc<ToolUseContext>, future: F) -> F::Output {
    tool_context::context::scope_tool_use_context(context, future).await
}
#[cfg(test)]
type FileEditTool = tool_files::file_edit::FileEditTool<Host>;
#[cfg(test)]
type FileWriteTool = tool_files::file_write::FileWriteTool<Host>;
#[cfg(test)]
type GlobTool = tool_files::glob::GlobTool<Host>;
#[cfg(test)]
type GitOperationsTool = tool_git::GitOperationsTool<Host>;
#[cfg(test)]
mod file_edit;
#[cfg(test)]
mod file_write;
#[cfg(test)]
mod glob;
#[cfg(test)]
mod git_operations;
#[cfg(test)]
mod mcp_values;
#[cfg(test)]
mod hooks;
mod mcp_host;
pub mod mcp_catalog;
