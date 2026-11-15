//! Desktop WebView process control and host-mediated MCP app actions.
pub mod mcp_form;

#[cfg(not(target_arch = "wasm32"))]
pub mod mcp_app_bridge;
#[cfg(not(target_arch = "wasm32"))]
pub mod mcp_app_download;
#[cfg(not(target_arch = "wasm32"))]
pub mod webview_runtime;
