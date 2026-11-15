//! Host composition for MCP hooks.
pub use plugin_hooks::hook_mcp::expand;
pub(crate) async fn execute(root: &std::path::Path, handler: &super::hook_manifest::CodexHookHandler, event: &serde_json::Value) -> anyhow::Result<Vec<u8>> {
    plugin_hooks::hook_mcp::execute::<crate::tools::context::AgentContextHost>(root, handler, event).await
}
