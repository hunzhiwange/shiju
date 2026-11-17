//! Compatibility exports from the plugin MCP implementation owner.
#[cfg(any(test, target_arch = "wasm32"))]
pub use plugin_mcp::mcp_legacy::*;
