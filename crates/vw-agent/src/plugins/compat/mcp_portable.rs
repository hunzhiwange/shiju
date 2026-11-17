//! 便携 MCP 协议校验的兼容入口。
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use plugin_mcp::mcp_portable::{parse_portable_mcp, env_names_match, SCHEMA};
