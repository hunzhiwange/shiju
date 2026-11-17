//! Compatibility exports; implementation belongs to plugin-mcp.
#[cfg(any(test, target_arch = "wasm32"))]
pub use plugin_mcp::mcp_content::*;
