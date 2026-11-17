//! Connection catalog state has one owner below plugin assembly.
#[cfg(any(test, target_arch = "wasm32"))]
pub use mcp_client::notifications::{McpCatalogs, ToolCatalogBinding};
