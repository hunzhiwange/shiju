//! Compatibility exports for plugin-owned resource/prompt operations.
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub use plugin_mcp::mcp_resources::call;
#[cfg(any(test, target_arch = "wasm32"))]
pub use plugin_mcp::mcp_resources::{RawResponse, call, request, validate_contents, list_entries, meta, string, uri};
